//! J17b（C2=(b)）：ProductDrive 的开关、相位、post-shippable 进入与交接 gate。

mod support;

use continuum_persist::{Db, Value};
use continuum_product::{
    Milestone, NotShippableReason, Product, ProductError, ProductPhase, ShippableUndetermined,
    ShippableVerdict, begin_post_shippable_work, load_product_drive, product_intent_for_handoff,
    propose_next_batch, save_lowered_intent, save_milestones, set_continuous_product_drive,
    record_shippable_evaluation,
};
use continuum_semantics::{ContractId, ContractRef, Priority};
use support::{boundary, milestone_generator, saved_product};

fn contract() -> ContractRef {
    ContractRef {
        id: ContractId::from_str("c1"),
        version: 1,
    }
}

async fn scenario(count: usize) -> (tempfile::TempDir, Db, Product, Vec<Milestone>) {
    let (dir, db) = support::new_db();
    let product = saved_product(&db, "p1").await;
    let objectives: Vec<String> = (0..count).map(|i| format!("m{i}")).collect();
    let objectives: Vec<&str> = objectives.iter().map(String::as_str).collect();
    let milestones = propose_next_batch(
        &product,
        &[],
        boundary(count.max(1), 8),
        &milestone_generator(&objectives),
    )
    .await
    .unwrap();
    let tx = db.begin().unwrap();
    save_milestones(&tx, &product, &milestones, 100).unwrap();
    tx.commit().unwrap();
    let tx = db.begin().unwrap();
    let product = continuum_product::load_product(&tx, product.id())
        .unwrap()
        .unwrap();
    let milestones = continuum_product::load_milestones(&tx, product.id()).unwrap();
    tx.commit().unwrap();
    (dir, db, product, milestones)
}

fn not_shippable() -> ShippableVerdict {
    ShippableVerdict::NotShippable {
        reason: NotShippableReason::CompletionBlocked,
    }
}

fn undetermined() -> ShippableVerdict {
    ShippableVerdict::Undetermined {
        reason: ShippableUndetermined::MissingProduct,
    }
}

#[tokio::test]
async fn set_continuous_product_drive_changes_only_the_switch() {
    let (_dir, db, product, _milestones) = scenario(1).await;
    let tx = db.begin().unwrap();
    let on = set_continuous_product_drive(&tx, product.id(), true, 101).unwrap();
    assert!(on.continuous_product_drive.enabled);
    assert_eq!(on.phase, ProductPhase::PreShippable);
    let off = set_continuous_product_drive(&tx, product.id(), false, 102).unwrap();
    assert!(!off.continuous_product_drive.enabled);
    assert_eq!(off.phase, ProductPhase::PreShippable);
    let stored = load_product_drive(&tx, product.id()).unwrap().unwrap();
    assert!(!stored.continuous_product_drive.enabled);
    assert_eq!(stored.phase, ProductPhase::PreShippable);
}

#[tokio::test]
async fn begin_post_shippable_work_rejects_pre_shippable() {
    let (_dir, db, product, milestones) = scenario(1).await;
    let tx = db.begin().unwrap();
    let err = begin_post_shippable_work(&tx, product.id(), milestones[0].id(), 101)
        .unwrap_err();
    assert!(matches!(err, ProductError::NotShippableYet));
}

#[tokio::test]
async fn begin_post_shippable_work_rejects_a_disabled_drive() {
    let (_dir, db, product, milestones) = scenario(1).await;
    let tx = db.begin().unwrap();
    let state = record_shippable_evaluation(
        &tx,
        product.id(),
        &ShippableVerdict::Shippable,
        101,
    )
    .unwrap();
    assert_eq!(state.phase, ProductPhase::Shippable);
    let err = begin_post_shippable_work(&tx, product.id(), milestones[0].id(), 102)
        .unwrap_err();
    assert!(matches!(err, ProductError::DriveDisabled));
}

#[tokio::test]
async fn begin_post_shippable_work_enters_post_once() {
    let (_dir, db, product, milestones) = scenario(1).await;
    let tx = db.begin().unwrap();
    record_shippable_evaluation(&tx, product.id(), &ShippableVerdict::Shippable, 101)
        .unwrap();
    set_continuous_product_drive(&tx, product.id(), true, 102).unwrap();
    let first = begin_post_shippable_work(&tx, product.id(), milestones[0].id(), 103)
        .unwrap();
    assert_eq!(first.phase, ProductPhase::PostShippableImprovement);
    let second = begin_post_shippable_work(&tx, product.id(), milestones[0].id(), 104)
        .unwrap();
    assert_eq!(second.phase, ProductPhase::PostShippableImprovement);
}

#[tokio::test]
async fn recording_not_shippable_never_leaves_pre_shippable() {
    let (_dir, db, product, _milestones) = scenario(1).await;
    let tx = db.begin().unwrap();
    for verdict in [not_shippable(), undetermined()] {
        let state = record_shippable_evaluation(&tx, product.id(), &verdict, 101).unwrap();
        assert_eq!(state.phase, ProductPhase::PreShippable);
    }
}

#[tokio::test]
async fn recording_shippable_then_not_shippable_does_not_regress() {
    let (_dir, db, product, _milestones) = scenario(1).await;
    let tx = db.begin().unwrap();
    record_shippable_evaluation(&tx, product.id(), &ShippableVerdict::Shippable, 101)
        .unwrap();
    let state = record_shippable_evaluation(&tx, product.id(), &not_shippable(), 102)
        .unwrap();
    assert_eq!(state.phase, ProductPhase::Shippable);
}

#[test]
fn the_phase_has_a_single_write_point() {
    let src = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
    let mut hits = Vec::new();
    for entry in std::fs::read_dir(src).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let source = std::fs::read_to_string(&path).unwrap();
        for (index, line) in source.lines().enumerate() {
            if line.contains("phase = ?") {
                hits.push((path.display().to_string(), index + 1, line.trim().to_owned()));
            }
        }
    }
    assert_eq!(hits.len(), 1, "phase 写点必须唯一: {hits:?}");
    assert!(
        hits[0].2.contains("product_drive"),
        "唯一的 phase 写点不在 product_drive: {:?}",
        hits[0]
    );
}

#[tokio::test]
async fn handoff_gate_closes_when_the_drive_is_disabled_in_post_phase() {
    let (_dir, db, product, milestones) = scenario(1).await;
    let tx = db.begin().unwrap();
    record_shippable_evaluation(&tx, product.id(), &ShippableVerdict::Shippable, 101)
        .unwrap();
    set_continuous_product_drive(&tx, product.id(), true, 102).unwrap();
    begin_post_shippable_work(&tx, product.id(), milestones[0].id(), 103).unwrap();
    let intent = save_lowered_intent(
        &tx,
        &product,
        &milestones[0],
        contract(),
        Priority::from_str("normal"),
        104,
        104,
    )
    .unwrap();
    tx.execute(
        "UPDATE intent SET status = 'ready' WHERE id = ?1",
        &[Value::text(intent.id.as_str())],
    )
    .unwrap();

    set_continuous_product_drive(&tx, product.id(), false, 105).unwrap();
    assert!(
        product_intent_for_handoff(&tx, &product, milestones[0].id())
            .unwrap()
            .is_none()
    );
    set_continuous_product_drive(&tx, product.id(), true, 106).unwrap();
    assert!(
        product_intent_for_handoff(&tx, &product, milestones[0].id())
            .unwrap()
            .is_some()
    );
}
