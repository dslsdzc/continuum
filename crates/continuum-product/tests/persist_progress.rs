//! J12／J13 的 Progress 落库面：完成态前置、证据 id 核对、写前判定、游标推进、
//! 真实策略序列与库内 delta 的一致性。

mod support;

use continuum_canonical::Opaque;
use continuum_persist::{Db, Tx, Value};
use continuum_product::{
    Milestone, MilestoneState, Product, ProductError, ProgressDelta, ProgressGateVerdict,
    load_milestone, load_product, load_progress, propose_next_batch, record_milestone_progress,
    record_milestone_state, save_milestones,
};
use support::{
    DeltaByDimension, boundary, evidence, milestone_generator, readiness_with_documentation,
    saved_product, saved_product_with_intent, seed_readiness,
};

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
    let product = load_product(&tx, product.id()).unwrap().unwrap();
    let milestones = continuum_product::load_milestones(&tx, product.id()).unwrap();
    tx.commit().unwrap();
    (dir, db, product, milestones)
}

fn count(tx: &Tx<'_>, table: &str) -> i64 {
    match tx
        .query(&format!("SELECT COUNT(*) FROM {table}"), &[])
        .unwrap()[0][0]
    {
        Value::Int(n) => n,
        ref other => panic!("COUNT 不是整数: {other:?}"),
    }
}

fn complete(db: &Db, milestone: &Milestone, at: i64) -> Milestone {
    let tx = db.begin().unwrap();
    record_milestone_state(
        &tx,
        milestone.id(),
        MilestoneState::Proposed,
        MilestoneState::Completed,
        at,
    )
    .unwrap();
    let loaded = load_milestone(&tx, milestone.id()).unwrap().unwrap();
    tx.commit().unwrap();
    loaded
}

#[tokio::test]
async fn record_milestone_progress_requires_a_completed_milestone_of_the_product() {
    let (_dir, db, product, milestones) = scenario(1).await;
    let other = saved_product_with_intent(&db, "p2", "intent/p2").await;
    let other_milestones = propose_next_batch(
        &other,
        &[],
        boundary(1, 8),
        &milestone_generator(&["other"]),
    )
    .await
    .unwrap();

    let tx = db.begin().unwrap();
    save_milestones(&tx, &other, &other_milestones, 100).unwrap();
    record_milestone_state(
        &tx,
        other_milestones[0].id(),
        MilestoneState::Proposed,
        MilestoneState::Completed,
        103,
    )
    .unwrap();
    record_milestone_state(
        &tx,
        milestones[0].id(),
        MilestoneState::Proposed,
        MilestoneState::Active,
        101,
    )
    .unwrap();
    let active = load_milestone(&tx, milestones[0].id()).unwrap().unwrap();
    let err = record_milestone_progress(
        &tx,
        &product,
        &active,
        product.readiness().clone(),
        &[],
        None,
        102,
    )
    .unwrap_err();
    assert!(matches!(
        err,
        ProductError::MilestoneNotCompleted { id } if id == "milestone/p1/0"
    ));

    let other_milestone = load_milestone(&tx, other_milestones[0].id())
        .unwrap()
        .unwrap();
    let err = record_milestone_progress(
        &tx,
        &product,
        &other_milestone,
        product.readiness().clone(),
        &[],
        None,
        104,
    )
    .unwrap_err();
    assert!(matches!(
        err,
        ProductError::MilestoneProductMismatch { .. }
    ));
    assert_eq!(count(&tx, "product_progress"), 0);
}

#[tokio::test]
async fn record_milestone_progress_rejects_a_second_record_for_the_same_milestone() {
    let (_dir, db, product, milestones) = scenario(1).await;
    let milestone = complete(&db, &milestones[0], 101);
    let tx = db.begin().unwrap();
    let product = load_product(&tx, product.id()).unwrap().unwrap();
    record_milestone_progress(
        &tx,
        &product,
        &milestone,
        product.readiness().clone(),
        &[],
        None,
        102,
    )
    .unwrap();
    let err = record_milestone_progress(
        &tx,
        &product,
        &milestone,
        product.readiness().clone(),
        &[],
        None,
        103,
    )
    .unwrap_err();
    assert!(matches!(
        err,
        ProductError::DuplicateProgress { product: p, milestone: m }
            if p == "p1" && m == "milestone/p1/0"
    ));
    assert_eq!(count(&tx, "product_progress"), 1);
}

#[tokio::test]
async fn record_milestone_progress_requires_every_referenced_evidence_id() {
    let (_dir, db, product, milestones) = scenario(1).await;
    let milestone = complete(&db, &milestones[0], 101);
    let tx = db.begin().unwrap();
    let product = load_product(&tx, product.id()).unwrap().unwrap();
    let current = continuum_product::ProductReadiness::unassessed().with_dimension(
        continuum_product::ReadinessDimensionName::Documentation,
        continuum_product::ReadinessDimension::assessed(
            Opaque::from_str("0.8"),
            vec![continuum_verify::EvidenceId::from_str("e2")],
            100,
        )
        .unwrap(),
    );
    let err = record_milestone_progress(
        &tx,
        &product,
        &milestone,
        current,
        &[evidence("e1")],
        None,
        102,
    )
    .unwrap_err();
    assert!(matches!(
        err,
        ProductError::EvidenceNotSupplied { id } if id == "e2"
    ));
    assert_eq!(count(&tx, "product_progress"), 0);
}

#[tokio::test]
async fn a_record_updates_readiness_and_returns_the_pre_write_verdict() {
    let (_dir, db, product, milestones) = scenario(1).await;
    let milestone = complete(&db, &milestones[0], 101);
    let tx = db.begin().unwrap();
    seed_readiness(&tx, "p1", "2");
    let product = load_product(&tx, product.id()).unwrap().unwrap();
    let current = readiness_with_documentation("2");
    let (record, verdict) = record_milestone_progress(
        &tx,
        &product,
        &milestone,
        current,
        &[evidence("e1")],
        Some(&DeltaByDimension { threshold: 3 }),
        102,
    )
    .unwrap();
    assert_eq!(
        verdict,
        ProgressGateVerdict::NotTriggered {
            consecutive_no_progress: 1,
            threshold: 3,
        }
    );
    assert_eq!(record.progress_delta, ProgressDelta::NoProgress);
    assert_eq!(record.recorded_at, 102);

    let stored = load_product(&tx, product.id()).unwrap().unwrap();
    let dimension = stored
        .readiness()
        .dimension(continuum_product::ReadinessDimensionName::Documentation);
    assert_eq!(
        dimension.value().as_assessed().map(Opaque::as_str),
        Some("2")
    );
    assert_eq!(dimension.last_verified(), Some(100));
}

#[tokio::test]
async fn the_stored_readiness_only_contains_evidence_ids() {
    let (_dir, db, product, milestones) = scenario(1).await;
    let milestone = complete(&db, &milestones[0], 101);
    let tx = db.begin().unwrap();
    let product = load_product(&tx, product.id()).unwrap().unwrap();
    record_milestone_progress(
        &tx,
        &product,
        &milestone,
        readiness_with_documentation("2"),
        &[evidence("e1")],
        None,
        102,
    )
    .unwrap();
    let raw = tx
        .query("SELECT readiness FROM product WHERE id = 'p1'", &[])
        .unwrap()[0][0]
        .clone();
    let Value::Text(raw) = raw else {
        panic!("readiness 不是文本");
    };
    assert!(raw.contains("\"e1\""));
    for forbidden in ["claim", "tests pass", "strength", "\"type\"", "scope"] {
        assert!(!raw.contains(forbidden), "readiness 里出现了 {forbidden}");
    }
}

#[tokio::test]
async fn completing_the_current_milestone_advances_the_cursor() {
    let (_dir, db, product, milestones) = scenario(2).await;
    let tx = db.begin().unwrap();
    // 直接置 Completed，绕过 record_milestone_state 的推进，专拍 progress 自己的规则。
    tx.execute(
        "UPDATE milestone SET state = 'completed' WHERE id = ?1",
        &[Value::text(milestones[0].id().as_str())],
    )
    .unwrap();
    let milestone = load_milestone(&tx, milestones[0].id()).unwrap().unwrap();
    // 不能用 load_product 取快照：它会把「当前游标指向终态」判成坏行。
    assert_eq!(
        product.current_milestone().map(|id| id.as_str()),
        Some("milestone/p1/0")
    );
    record_milestone_progress(
        &tx,
        &product,
        &milestone,
        product.readiness().clone(),
        &[],
        None,
        102,
    )
    .unwrap();
    let stored = load_product(&tx, product.id()).unwrap().unwrap();
    assert_eq!(
        stored.current_milestone().map(|id| id.as_str()),
        Some("milestone/p1/1")
    );
}

#[tokio::test]
async fn load_progress_returns_records_in_recorded_at_order() {
    let (_dir, db, product, milestones) = scenario(3).await;
    let tx = db.begin().unwrap();
    for (milestone, at) in milestones.iter().zip([300, 100, 200]) {
        tx.execute(
            "UPDATE milestone SET state = 'completed' WHERE id = ?1",
            &[Value::text(milestone.id().as_str())],
        )
        .unwrap();
        let milestone = load_milestone(&tx, milestone.id()).unwrap().unwrap();
        record_milestone_progress(
            &tx,
            &product,
            &milestone,
            product.readiness().clone(),
            &[],
            None,
            at,
        )
        .unwrap();
    }
    let records = load_progress(&tx, product.id()).unwrap();
    let recorded_at: Vec<i64> = records.iter().map(|r| r.recorded_at).collect();
    assert_eq!(recorded_at, [100, 200, 300]);
}

#[tokio::test]
async fn a_real_policy_progress_sequence_counts_to_the_threshold() {
    let (_dir, db, product, milestones) = scenario(3).await;
    let tx = db.begin().unwrap();
    seed_readiness(&tx, "p1", "2");
    let policy = DeltaByDimension { threshold: 3 };
    let mut verdicts = Vec::new();
    for milestone in &milestones {
        record_milestone_state(
            &tx,
            milestone.id(),
            MilestoneState::Proposed,
            MilestoneState::Completed,
            101,
        )
        .unwrap();
        let completed = load_milestone(&tx, milestone.id()).unwrap().unwrap();
        let product = load_product(&tx, product.id()).unwrap().unwrap();
        let (_, verdict) = record_milestone_progress(
            &tx,
            &product,
            &completed,
            readiness_with_documentation("2"),
            &[evidence("e1")],
            Some(&policy),
            102,
        )
        .unwrap();
        verdicts.push(verdict);
    }
    assert_eq!(
        verdicts,
        [
            ProgressGateVerdict::NotTriggered {
                consecutive_no_progress: 1,
                threshold: 3
            },
            ProgressGateVerdict::NotTriggered {
                consecutive_no_progress: 2,
                threshold: 3
            },
            ProgressGateVerdict::Triggered {
                consecutive_no_progress: 3,
                threshold: 3
            }
        ]
    );
    let records = load_progress(&tx, product.id()).unwrap();
    assert_eq!(records.len(), 3);
    for record in &records {
        assert_eq!(record.progress_delta, ProgressDelta::NoProgress);
    }
}

#[tokio::test]
async fn an_improved_record_in_a_real_sequence_resets_the_run() {
    let (_dir, db, product, milestones) = scenario(4).await;
    let tx = db.begin().unwrap();
    seed_readiness(&tx, "p1", "2");
    let policy = DeltaByDimension { threshold: 3 };
    let mut verdicts = Vec::new();
    for (milestone, score) in milestones.iter().zip(["2", "2", "3", "3"]) {
        record_milestone_state(
            &tx,
            milestone.id(),
            MilestoneState::Proposed,
            MilestoneState::Completed,
            101,
        )
        .unwrap();
        let completed = load_milestone(&tx, milestone.id()).unwrap().unwrap();
        let product = load_product(&tx, product.id()).unwrap().unwrap();
        let (_, verdict) = record_milestone_progress(
            &tx,
            &product,
            &completed,
            readiness_with_documentation(score),
            &[evidence("e1")],
            Some(&policy),
            102,
        )
        .unwrap();
        verdicts.push(verdict);
    }
    assert_eq!(
        verdicts,
        [
            ProgressGateVerdict::NotTriggered {
                consecutive_no_progress: 1,
                threshold: 3
            },
            ProgressGateVerdict::NotTriggered {
                consecutive_no_progress: 2,
                threshold: 3
            },
            ProgressGateVerdict::NotTriggered {
                consecutive_no_progress: 0,
                threshold: 3
            },
            ProgressGateVerdict::NotTriggered {
                consecutive_no_progress: 1,
                threshold: 3
            }
        ]
    );
    let deltas: Vec<ProgressDelta> = load_progress(&tx, product.id())
        .unwrap()
        .iter()
        .map(|record| record.progress_delta)
        .collect();
    assert_eq!(
        deltas,
        [
            ProgressDelta::NoProgress,
            ProgressDelta::NoProgress,
            ProgressDelta::Improved,
            ProgressDelta::NoProgress
        ]
    );
}
