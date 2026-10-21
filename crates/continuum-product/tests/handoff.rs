//! J11：只交接 READY 的 Product Intent；其余十二态一律 `None`；retire 走状态迁移。

mod support;

use continuum_persist::{Db, Tx, Value};
use continuum_product::{
    Milestone, Product, ProductError, product_intent_for_handoff, propose_next_batch,
    retire_completed_product_intent, save_lowered_intent, save_milestones,
};
use continuum_semantics::{
    ContractId, ContractRef, Intent, IntentId, IntentStatus, Priority, apply_intent_transition,
    load_intent,
};
use support::{boundary, milestone_generator, saved_product};

fn contract() -> ContractRef {
    ContractRef {
        id: ContractId::from_str("c1"),
        version: 1,
    }
}

fn priority() -> Priority {
    Priority::from_str("normal")
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

fn lower(tx: &Tx<'_>, product: &Product, milestone: &Milestone) -> Intent {
    save_lowered_intent(tx, product, milestone, contract(), priority(), 10, 20).unwrap()
}

fn set_status(tx: &Tx<'_>, id: &IntentId, status: IntentStatus) {
    let name = format!("{status:?}").to_lowercase();
    tx.execute(
        "UPDATE intent SET status = ?2 WHERE id = ?1",
        &[Value::text(id.as_str()), Value::text(&name)],
    )
    .unwrap();
}

const ALL_STATUSES: [IntentStatus; 13] = [
    IntentStatus::Proposed,
    IntentStatus::Specifying,
    IntentStatus::Planning,
    IntentStatus::Review,
    IntentStatus::Ready,
    IntentStatus::Running,
    IntentStatus::Waiting,
    IntentStatus::Suspended,
    IntentStatus::Verifying,
    IntentStatus::Completed,
    IntentStatus::Failed,
    IntentStatus::Cancelled,
    IntentStatus::Evolving,
];

#[tokio::test]
async fn only_ready_intents_are_handed_off() {
    let (_dir, db, product, milestones) = scenario(13).await;
    let tx = db.begin().unwrap();
    for (milestone, status) in milestones.iter().zip(ALL_STATUSES) {
        let intent = lower(&tx, &product, milestone);
        set_status(&tx, &intent.id, status);
    }
    for (milestone, status) in milestones.iter().zip(ALL_STATUSES) {
        let result = product_intent_for_handoff(&tx, &product, milestone.id()).unwrap();
        if status == IntentStatus::Ready {
            assert!(result.is_some(), "READY 应交接");
            assert_eq!(result.unwrap().status, IntentStatus::Ready);
        } else {
            assert!(result.is_none(), "{status:?} 不应交接");
        }
    }
}

#[tokio::test]
async fn two_ready_intents_are_both_handed_off_by_milestone_id() {
    let (_dir, db, product, milestones) = scenario(2).await;
    let tx = db.begin().unwrap();
    for milestone in &milestones {
        let intent = lower(&tx, &product, milestone);
        set_status(&tx, &intent.id, IntentStatus::Ready);
    }
    for milestone in &milestones {
        assert!(
            product_intent_for_handoff(&tx, &product, milestone.id())
                .unwrap()
                .is_some()
        );
    }
}

#[tokio::test]
async fn a_running_intent_that_never_passed_review_is_not_handed_off() {
    let (_dir, db, product, milestones) = scenario(1).await;
    let tx = db.begin().unwrap();
    let intent = lower(&tx, &product, &milestones[0]);
    set_status(&tx, &intent.id, IntentStatus::Planning);
    apply_intent_transition(
        &tx,
        &intent.id,
        IntentStatus::Suspended,
        30,
        "intent/lowered/suspended",
    )
    .unwrap();
    apply_intent_transition(
        &tx,
        &intent.id,
        IntentStatus::Running,
        31,
        "intent/lowered/running",
    )
    .unwrap();
    let result = product_intent_for_handoff(&tx, &product, milestones[0].id()).unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn a_missing_lowered_intent_is_not_an_error() {
    let (_dir, db, product, milestones) = scenario(1).await;
    let tx = db.begin().unwrap();
    let result = product_intent_for_handoff(&tx, &product, milestones[0].id()).unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn a_malformed_lowered_intent_is_rejected() {
    let (_dir, db, product, milestones) = scenario(3).await;
    let tx = db.begin().unwrap();
    for (index, milestone) in milestones.iter().enumerate() {
        let intent = lower(&tx, &product, milestone);
        match index {
            0 => {
                tx.execute(
                    "UPDATE intent SET kind = 'task' WHERE id = ?1",
                    &[Value::text(intent.id.as_str())],
                )
                .unwrap();
            }
            1 => {
                tx.execute(
                    "UPDATE intent SET product_id = 'p2' WHERE id = ?1",
                    &[Value::text(intent.id.as_str())],
                )
                .unwrap();
            }
            _ => {
                tx.execute(
                    "UPDATE intent SET contract_version = 0 WHERE id = ?1",
                    &[Value::text(intent.id.as_str())],
                )
                .unwrap();
            }
        }
        let err = product_intent_for_handoff(&tx, &product, milestone.id()).unwrap_err();
        assert!(
            matches!(err, ProductError::InvalidLoweredIntent { .. }),
            "第 {index} 个畸形 Intent 未报 InvalidLoweredIntent"
        );
    }
}

#[tokio::test]
async fn retire_moves_completed_to_evolving_and_stops_handoff() {
    let (_dir, db, product, milestones) = scenario(1).await;
    let tx = db.begin().unwrap();
    let intent = lower(&tx, &product, &milestones[0]);
    set_status(&tx, &intent.id, IntentStatus::Completed);
    retire_completed_product_intent(
        &tx,
        &intent.id,
        30,
        "intent/milestone/p1/0/evolving",
    )
    .unwrap();
    let loaded = load_intent(&tx, &intent.id).unwrap().unwrap();
    assert_eq!(loaded.status, IntentStatus::Evolving);
    assert!(
        product_intent_for_handoff(&tx, &product, milestones[0].id())
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn handoff_requires_a_drive_state() {
    let (_dir, db, product, milestones) = scenario(1).await;
    let tx = db.begin().unwrap();
    let intent = lower(&tx, &product, &milestones[0]);
    set_status(&tx, &intent.id, IntentStatus::Ready);
    tx.execute("DELETE FROM product_drive WHERE product_id = 'p1'", &[])
        .unwrap();
    let err = product_intent_for_handoff(&tx, &product, milestones[0].id()).unwrap_err();
    assert!(matches!(
        err,
        ProductError::MissingDriveState { product } if product == "p1"
    ));
}
