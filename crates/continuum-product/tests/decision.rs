//! J14：Progress Gate 只能从写前 history 重算并**只出** replan Decision；
//! 不执行 replan、不接受调用方伪造的 verdict。

mod support;

use continuum_canonical::Opaque;
use continuum_persist::{Db, Tx, Value};
use continuum_product::{
    Milestone, Product, ProductError, ProgressDelta, ProgressRecord, ReplanDraft,
    emit_replan_decision, propose_next_batch, save_milestones,
};
use continuum_semantics::{Decision, DecisionOption};
use support::{
    DeltaByDimension, boundary, milestone_generator, readiness_with_documentation, saved_product,
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
    let product = continuum_product::load_product(&tx, product.id())
        .unwrap()
        .unwrap();
    let milestones = continuum_product::load_milestones(&tx, product.id()).unwrap();
    tx.commit().unwrap();
    (dir, db, product, milestones)
}

fn progress_record(
    product: &Product,
    milestone: &Milestone,
    delta: ProgressDelta,
    score: &str,
    at: i64,
) -> ProgressRecord {
    ProgressRecord {
        product_id: product.id().clone(),
        milestone_id: milestone.id().clone(),
        previous_product_state: readiness_with_documentation(score),
        current_product_state: readiness_with_documentation(score),
        progress_delta: delta,
        recorded_at: at,
    }
}

fn draft(issue: &str, options: usize) -> ReplanDraft {
    ReplanDraft {
        issue: Opaque::from_str(issue),
        options: (0..options)
            .map(|i| {
                DecisionOption::new(
                    Opaque::from_str(&format!("option {i}")),
                    Opaque::from_str(&format!("consequence {i}")),
                )
            })
            .collect(),
        blocking: true,
    }
}

fn decision_events(tx: &Tx<'_>) -> i64 {
    match tx
        .query(
            "SELECT COUNT(*) FROM events WHERE event_type = 'decision.required'",
            &[],
        )
        .unwrap()[0][0]
    {
        Value::Int(n) => n,
        ref other => panic!("COUNT 不是整数: {other:?}"),
    }
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

#[tokio::test]
async fn a_triggered_recomputation_writes_one_decision_and_one_event() {
    let (_dir, db, product, milestones) = scenario(2).await;
    let tx = db.begin().unwrap();
    let no_progress = DeltaByDimension { threshold: 2 };
    let first = progress_record(
        &product,
        &milestones[0],
        ProgressDelta::NoProgress,
        "2",
        101,
    );
    let history = [progress_record(
        &product,
        &milestones[1],
        ProgressDelta::NoProgress,
        "2",
        100,
    )];
    let decision = emit_replan_decision(
        &tx,
        &product,
        &first,
        &history,
        Some(&no_progress),
        draft("scope changed", 2),
        102,
    )
    .unwrap()
    .expect("应产出 Decision");
    assert_eq!(decision.issue.as_str(), "scope changed");
    assert_eq!(decision.options.len(), 2);
    assert!(decision.blocking);
    assert_eq!(decision.intent_id.as_str(), product.intent_id().as_str());
    assert_eq!(count(&tx, "decision"), 1);
    let events = tx
        .query(
            "SELECT event_type FROM events WHERE event_type = 'decision.required'",
            &[],
        )
        .unwrap();
    assert_eq!(events.len(), 1);
}

#[tokio::test]
async fn a_not_triggered_recomputation_writes_nothing() {
    let (_dir, db, product, milestones) = scenario(1).await;
    let tx = db.begin().unwrap();
    let record = progress_record(
        &product,
        &milestones[0],
        ProgressDelta::NoProgress,
        "2",
        101,
    );
    let decision = emit_replan_decision(
        &tx,
        &product,
        &record,
        &[],
        Some(&DeltaByDimension { threshold: 3 }),
        draft("scope changed", 1),
        102,
    )
    .unwrap();
    assert!(decision.is_none());
    assert_eq!(count(&tx, "decision"), 0);
    assert_eq!(decision_events(&tx), 0);
}

#[tokio::test]
async fn an_undetermined_recomputation_writes_nothing() {
    let (_dir, db, product, milestones) = scenario(1).await;
    let tx = db.begin().unwrap();
    let record = progress_record(
        &product,
        &milestones[0],
        ProgressDelta::NoProgress,
        "2",
        101,
    );
    let decision =
        emit_replan_decision(&tx, &product, &record, &[], None, draft("x", 1), 102)
            .unwrap();
    assert!(decision.is_none());
    assert_eq!(count(&tx, "decision"), 0);
    assert_eq!(decision_events(&tx), 0);
}

#[tokio::test]
async fn a_record_repeated_in_history_is_counted_once() {
    let (_dir, db, product, milestones) = scenario(3).await;
    let tx = db.begin().unwrap();
    let record = progress_record(
        &product,
        &milestones[0],
        ProgressDelta::NoProgress,
        "2",
        104,
    );
    let history = [
        progress_record(
            &product,
            &milestones[1],
            ProgressDelta::NoProgress,
            "2",
            101,
        ),
        progress_record(
            &product,
            &milestones[1],
            ProgressDelta::Improved,
            "2",
            102,
        ),
        progress_record(
            &product,
            &milestones[2],
            ProgressDelta::NoProgress,
            "2",
            103,
        ),
        record.clone(),
    ];
    let decision = emit_replan_decision(
        &tx,
        &product,
        &record,
        &history,
        Some(&DeltaByDimension { threshold: 3 }),
        draft("x", 1),
        105,
    )
    .unwrap();
    assert!(
        decision.is_none(),
        "去重后连续数为 2，未达阈值 3，不得 Triggered"
    );
    assert_eq!(count(&tx, "decision"), 0);
    assert_eq!(decision_events(&tx), 0);
}

#[tokio::test]
async fn an_incorrect_history_count_cannot_trigger() {
    let (_dir, db, product, milestones) = scenario(3).await;
    let tx = db.begin().unwrap();
    let record = progress_record(
        &product,
        &milestones[0],
        ProgressDelta::NoProgress,
        "2",
        104,
    );
    let history = [
        progress_record(
            &product,
            &milestones[1],
            ProgressDelta::Improved,
            "2",
            101,
        ),
        progress_record(
            &product,
            &milestones[2],
            ProgressDelta::NoProgress,
            "2",
            102,
        ),
    ];
    let decision = emit_replan_decision(
        &tx,
        &product,
        &record,
        &history,
        Some(&DeltaByDimension { threshold: 3 }),
        draft("x", 1),
        105,
    )
    .unwrap();
    assert!(decision.is_none());
    assert_eq!(count(&tx, "decision"), 0);
}

#[tokio::test]
async fn a_second_emit_hits_the_decision_primary_key() {
    let (_dir, db, product, milestones) = scenario(2).await;
    let tx = db.begin().unwrap();
    let policy = DeltaByDimension { threshold: 2 };
    let record = progress_record(
        &product,
        &milestones[0],
        ProgressDelta::NoProgress,
        "2",
        101,
    );
    let history = [progress_record(
        &product,
        &milestones[1],
        ProgressDelta::NoProgress,
        "2",
        100,
    )];
    emit_replan_decision(
        &tx,
        &product,
        &record,
        &history,
        Some(&policy),
        draft("x", 1),
        102,
    )
    .unwrap()
    .unwrap();
    let err = emit_replan_decision(
        &tx,
        &product,
        &record,
        &history,
        Some(&policy),
        draft("x", 1),
        103,
    )
    .unwrap_err();
    assert!(matches!(err, ProductError::Decision(_)));
    assert_eq!(count(&tx, "decision"), 1);
}

#[tokio::test]
async fn empty_issue_or_options_are_rejected() {
    let (_dir, db, product, milestones) = scenario(2).await;
    let tx = db.begin().unwrap();
    let policy = DeltaByDimension { threshold: 2 };
    let record = progress_record(
        &product,
        &milestones[0],
        ProgressDelta::NoProgress,
        "2",
        101,
    );
    let history = [progress_record(
        &product,
        &milestones[1],
        ProgressDelta::NoProgress,
        "2",
        100,
    )];
    let err = emit_replan_decision(
        &tx,
        &product,
        &record,
        &history,
        Some(&policy),
        draft("", 1),
        102,
    )
    .unwrap_err();
    assert!(matches!(
        err,
        ProductError::EmptyReplanField { field: "issue" }
    ));
    let err = emit_replan_decision(
        &tx,
        &product,
        &record,
        &history,
        Some(&policy),
        draft("x", 0),
        103,
    )
    .unwrap_err();
    assert!(matches!(
        err,
        ProductError::EmptyReplanField { field: "options" }
    ));
    assert_eq!(count(&tx, "decision"), 0);
}

#[tokio::test]
async fn the_decision_id_is_derived_from_product_and_milestone() {
    let (_dir, db, product, milestones) = scenario(2).await;
    let tx = db.begin().unwrap();
    let record = progress_record(
        &product,
        &milestones[0],
        ProgressDelta::NoProgress,
        "2",
        101,
    );
    let history = [progress_record(
        &product,
        &milestones[1],
        ProgressDelta::NoProgress,
        "2",
        100,
    )];
    let decision: Decision = emit_replan_decision(
        &tx,
        &product,
        &record,
        &history,
        Some(&DeltaByDimension { threshold: 2 }),
        draft("x", 1),
        102,
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        decision.id.as_str(),
        "decision/p1/replan/milestone/p1/0"
    );
    assert_eq!(decision.intent_id.as_str(), product.intent_id().as_str());
}

#[test]
fn there_is_no_planner_or_replan_execution() {
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
                !trimmed.contains("Planner") && !trimmed.contains("replan("),
                "{}:{} 出现 Planner／replan(：{line}",
                path.display(),
                index + 1
            );
        }
    }
}
