//! J10：Milestone → Product Intent 的确定 lower、唯一写路径与 READY 防线。

mod support;

use continuum_canonical::Opaque;
use continuum_persist::{Db, Tx, Value};
use continuum_product::{
    Milestone, MilestoneBatchDraft, MilestoneDraft, Product, ProductError, lower_milestone,
    propose_next_batch, save_lowered_intent, save_milestones,
};
use continuum_semantics::{
    ContractId, ContractRef, Intent, IntentId, IntentKind, IntentStatus, Priority, ProductIdRef,
    save_intent,
};
use support::{FixedMilestoneGenerator, saved_product, saved_product_with_intent};

fn contract() -> ContractRef {
    ContractRef {
        id: ContractId::from_str("c1"),
        version: 1,
    }
}

fn priority() -> Priority {
    Priority::from_str("normal")
}

async fn scenario(exit: Vec<&str>) -> (tempfile::TempDir, Db, Product, Milestone) {
    let (dir, db) = support::new_db();
    let product = saved_product(&db, "p1").await;
    let drafts = MilestoneBatchDraft {
        milestones: vec![MilestoneDraft {
            objective: Opaque::from_str("m0"),
            dependencies: vec![],
            entry_conditions: vec![],
            exit_conditions: exit.into_iter().map(Opaque::from_str).collect(),
        }],
    };
    let milestones = propose_next_batch(
        &product,
        &[],
        continuum_product::BatchBoundary::new(1, 0).unwrap(),
        &FixedMilestoneGenerator::new(vec![Ok(drafts)]),
    )
    .await
    .unwrap();
    let tx = db.begin().unwrap();
    save_milestones(&tx, &product, &milestones, 100).unwrap();
    tx.commit().unwrap();
    let tx = db.begin().unwrap();
    let milestone = continuum_product::load_milestone(&tx, milestones[0].id())
        .unwrap()
        .unwrap();
    tx.commit().unwrap();
    (dir, db, product, milestone)
}

fn lowered_count(tx: &Tx<'_>) -> i64 {
    match tx
        .query(
            "SELECT COUNT(*) FROM intent WHERE id LIKE 'intent/milestone/%'",
            &[],
        )
        .unwrap()[0][0]
    {
        Value::Int(n) => n,
        ref other => panic!("COUNT 不是整数: {other:?}"),
    }
}

#[tokio::test]
async fn lower_milestone_builds_a_product_intent_in_proposed() {
    let (_dir, _db, product, milestone) = scenario(vec![]).await;
    let intent = lower_milestone(&product, &milestone, contract(), priority(), 10, 20).unwrap();
    assert_eq!(intent.id.as_str(), "intent/milestone/p1/0");
    assert_eq!(intent.version, 1);
    assert_eq!(intent.kind, IntentKind::Product);
    assert_eq!(intent.goal.as_str(), "m0");
    assert_eq!(intent.status, IntentStatus::Proposed);
    assert_eq!(
        intent.parent_intent.as_ref().map(IntentId::as_str),
        Some("intent/root")
    );
    assert_eq!(intent.contract_id.id.as_str(), "c1");
    assert_eq!(intent.contract_id.version, 1);
    assert!(intent.active_plan_id.is_none());
    assert!(intent.graph_id.is_none());
    assert_eq!(
        intent.product_id.as_ref().map(ProductIdRef::as_str),
        Some("p1")
    );
    assert_eq!(intent.completion_predicate.as_str(), "");
}

#[tokio::test]
async fn two_lowerings_of_the_same_milestone_are_field_identical() {
    let (_dir, _db, product, milestone) = scenario(vec!["a", "b"]).await;
    let first = lower_milestone(&product, &milestone, contract(), priority(), 10, 20).unwrap();
    let second = lower_milestone(&product, &milestone, contract(), priority(), 30, 40).unwrap();
    assert_eq!(first.id.as_str(), second.id.as_str());
    assert_eq!(first.kind, second.kind);
    assert_eq!(first.goal.as_str(), second.goal.as_str());
    assert_eq!(
        first.parent_intent.as_ref().map(IntentId::as_str),
        second.parent_intent.as_ref().map(IntentId::as_str)
    );
    assert_eq!(
        first.contract_id.id.as_str(),
        second.contract_id.id.as_str()
    );
    assert_eq!(first.contract_id.version, second.contract_id.version);
    assert_eq!(
        first.completion_predicate.as_str(),
        second.completion_predicate.as_str()
    );
}

#[tokio::test]
async fn an_empty_exit_condition_list_encodes_as_the_empty_string() {
    let (_dir, _db, product, milestone) = scenario(vec![]).await;
    let intent = lower_milestone(&product, &milestone, contract(), priority(), 10, 20).unwrap();
    assert_eq!(intent.completion_predicate.as_str(), "");
}

#[tokio::test]
async fn exit_conditions_are_joined_in_order() {
    let (_dir, _db, product, milestone) = scenario(vec!["a", "b"]).await;
    let intent = lower_milestone(&product, &milestone, contract(), priority(), 10, 20).unwrap();
    assert_eq!(intent.completion_predicate.as_str(), "a\nb");
}

#[tokio::test]
async fn a_milestone_from_another_product_is_rejected() {
    let (_dir, db, product, _milestone) = scenario(vec![]).await;
    let other = saved_product_with_intent(&db, "p2", "intent/p2").await;
    let drafts = MilestoneBatchDraft {
        milestones: vec![MilestoneDraft {
            objective: Opaque::from_str("other"),
            dependencies: vec![],
            entry_conditions: vec![],
            exit_conditions: vec![],
        }],
    };
    let other_milestones = propose_next_batch(
        &other,
        &[],
        continuum_product::BatchBoundary::new(1, 0).unwrap(),
        &FixedMilestoneGenerator::new(vec![Ok(drafts)]),
    )
    .await
    .unwrap();
    let tx = db.begin().unwrap();
    save_milestones(&tx, &other, &other_milestones, 100).unwrap();
    let other_milestone = continuum_product::load_milestone(&tx, other_milestones[0].id())
        .unwrap()
        .unwrap();
    let err = save_lowered_intent(
        &tx,
        &product,
        &other_milestone,
        contract(),
        priority(),
        10,
        20,
    )
    .unwrap_err();
    assert!(matches!(
        err,
        ProductError::MilestoneProductMismatch { .. }
    ));
    assert_eq!(lowered_count(&tx), 0);
}

#[tokio::test]
async fn an_invalid_contract_is_rejected() {
    let (_dir, db, product, milestone) = scenario(vec![]).await;
    let tx = db.begin().unwrap();
    for invalid in [
        ContractRef {
            id: ContractId::from_str(""),
            version: 1,
        },
        ContractRef {
            id: ContractId::from_str("c1"),
            version: 0,
        },
    ] {
        let err = save_lowered_intent(
            &tx,
            &product,
            &milestone,
            invalid,
            priority(),
            10,
            20,
        )
        .unwrap_err();
        assert!(matches!(err, ProductError::InvalidContract { .. }));
    }
    assert_eq!(lowered_count(&tx), 0);
}

#[tokio::test]
async fn save_lowered_intent_writes_once_and_does_not_overwrite_a_ready_intent() {
    let (_dir, db, product, milestone) = scenario(vec![]).await;
    let tx = db.begin().unwrap();
    let first = save_lowered_intent(
        &tx,
        &product,
        &milestone,
        contract(),
        priority(),
        10,
        20,
    )
    .unwrap();
    assert_eq!(lowered_count(&tx), 1);
    tx.execute(
        "UPDATE intent SET status = 'ready' WHERE id = ?1",
        &[Value::text(first.id.as_str())],
    )
    .unwrap();

    let second = save_lowered_intent(
        &tx,
        &product,
        &milestone,
        contract(),
        priority(),
        10,
        20,
    )
    .unwrap();
    assert_eq!(second.status, IntentStatus::Ready);
    assert_eq!(second.version, first.version);
    let rows = tx
        .query(
            "SELECT status, version FROM intent WHERE id = ?1",
            &[Value::text(first.id.as_str())],
        )
        .unwrap();
    assert_eq!(rows[0][0], Value::text("ready"));
    assert_eq!(rows[0][1], Value::Int(1));
}

#[tokio::test]
async fn save_lowered_intent_rejects_a_field_collision() {
    let (_dir, db, product, milestone) = scenario(vec![]).await;
    let tx = db.begin().unwrap();
    let colliding = Intent {
        id: IntentId::from_str("intent/milestone/p1/0"),
        version: 1,
        kind: IntentKind::Product,
        goal: continuum_canonical::EntityId::from_str("other"),
        status: IntentStatus::Proposed,
        priority: priority(),
        created_at: 1,
        updated_at: 1,
        parent_intent: Some(IntentId::from_str("intent/root")),
        contract_id: contract(),
        active_plan_id: None,
        graph_id: None,
        product_id: Some(ProductIdRef::from_str("p1")),
        completion_predicate: Opaque::from_str(""),
    };
    save_intent(&tx, &colliding).unwrap();
    let err = save_lowered_intent(
        &tx,
        &product,
        &milestone,
        contract(),
        priority(),
        10,
        20,
    )
    .unwrap_err();
    assert!(matches!(
        err,
        ProductError::IntentIdCollision { id } if id == "intent/milestone/p1/0"
    ));
    let rows = tx
        .query(
            "SELECT goal FROM intent WHERE id = 'intent/milestone/p1/0'",
            &[],
        )
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], Value::text("other"));
}

#[tokio::test]
async fn save_lowered_intent_writes_exactly_one_intent_created_event() {
    let (_dir, db, product, milestone) = scenario(vec![]).await;
    let tx = db.begin().unwrap();
    let before = tx
        .query(
            "SELECT COUNT(*) FROM events
             WHERE event_type = 'intent.created'
               AND payload LIKE '%intent/milestone/p1/0%'",
            &[],
        )
        .unwrap()[0][0]
        .clone();
    let _ = save_lowered_intent(
        &tx,
        &product,
        &milestone,
        contract(),
        priority(),
        10,
        20,
    )
    .unwrap();
    let after_first = tx
        .query(
            "SELECT COUNT(*) FROM events
             WHERE event_type = 'intent.created'
               AND payload LIKE '%intent/milestone/p1/0%'",
            &[],
        )
        .unwrap()[0][0]
        .clone();
    let _ = save_lowered_intent(
        &tx,
        &product,
        &milestone,
        contract(),
        priority(),
        10,
        20,
    )
    .unwrap();
    let after_second = tx
        .query(
            "SELECT COUNT(*) FROM events
             WHERE event_type = 'intent.created'
               AND payload LIKE '%intent/milestone/p1/0%'",
            &[],
        )
        .unwrap()[0][0]
        .clone();
    let before = match before {
        Value::Int(n) => n,
        other => panic!("COUNT 不是整数: {other:?}"),
    };
    let after_first = match after_first {
        Value::Int(n) => n,
        other => panic!("COUNT 不是整数: {other:?}"),
    };
    let after_second = match after_second {
        Value::Int(n) => n,
        other => panic!("COUNT 不是整数: {other:?}"),
    };
    assert_eq!(after_first, before + 1);
    assert_eq!(after_second, after_first);
}

#[test]
fn lower_does_not_write_intent_tables_directly() {
    let src = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
    for entry in std::fs::read_dir(src).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let source = std::fs::read_to_string(&path).unwrap();
        for (index, line) in source.lines().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") || trimmed.starts_with("use ") {
                continue;
            }
            assert!(
                !trimmed.contains("UPDATE intent") && !trimmed.contains("INSERT INTO intent"),
                "{}:{} 直接写 intent 表：{line}",
                path.display(),
                index + 1
            );
        }
    }
}
