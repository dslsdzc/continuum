//! J7／J8／J9 的 Milestone 落库面：seq／id 全局序号、写入前全部校验、七状态往返、
//! 乐观并发、游标推进与依赖顺序。

mod support;

use continuum_canonical::Opaque;
use continuum_persist::{Tx, Value};
use continuum_product::{
    BatchBoundary, MilestoneBatchDraft, MilestoneDependency, MilestoneDraft, MilestoneState,
    Product, ProductError, load_milestone, load_milestones, load_product, propose_next_batch,
    record_milestone_state, save_milestones, set_current_milestone,
};
use continuum_semantics::ProductIdRef;
use support::{
    FixedMilestoneGenerator, milestone_generator, saved_product, saved_product_with_intent,
};

fn draft(
    objective: &str,
    dependencies: Vec<MilestoneDependency>,
    entry: Vec<&str>,
    exit: Vec<&str>,
) -> MilestoneDraft {
    MilestoneDraft {
        objective: Opaque::from_str(objective),
        dependencies,
        entry_conditions: entry.into_iter().map(Opaque::from_str).collect(),
        exit_conditions: exit.into_iter().map(Opaque::from_str).collect(),
    }
}

fn plain(objective: &str) -> MilestoneDraft {
    draft(objective, vec![], vec![], vec![])
}

fn batch(milestones: Vec<MilestoneDraft>) -> MilestoneBatchDraft {
    MilestoneBatchDraft { milestones }
}

fn boundary(max_milestones: usize, max_depth: u32) -> BatchBoundary {
    BatchBoundary::new(max_milestones, max_depth).unwrap()
}

fn count(tx: &Tx<'_>, table: &str) -> i64 {
    let rows = tx
        .query(&format!("SELECT COUNT(*) FROM {table}"), &[])
        .unwrap();
    match rows[0][0] {
        Value::Int(n) => n,
        ref other => panic!("COUNT 不是整数: {other:?}"),
    }
}

fn seqs(tx: &Tx<'_>, product_id: &str) -> Vec<i64> {
    tx.query(
        "SELECT seq FROM milestone WHERE product_id = ?1 ORDER BY seq",
        &[Value::text(product_id)],
    )
    .unwrap()
    .into_iter()
    .map(|row| match row[0] {
        Value::Int(n) => n,
        ref other => panic!("seq 不是整数: {other:?}"),
    })
    .collect()
}

async fn propose(
    product: &Product,
    history: &[continuum_product::Milestone],
    objectives: &[&str],
) -> Vec<continuum_product::Milestone> {
    propose_next_batch(
        product,
        history,
        boundary(objectives.len().max(1), 8),
        &milestone_generator(objectives),
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn save_milestones_writes_contiguous_seq_starting_at_zero() {
    let (_dir, db) = support::new_db();
    let product = saved_product(&db, "p1").await;
    let milestones = propose(&product, &[], &["m0", "m1"]).await;
    let tx = db.begin().unwrap();
    save_milestones(&tx, &product, &milestones, 100).unwrap();

    assert_eq!(seqs(&tx, "p1"), [0, 1]);
    let stored = load_product(&tx, product.id()).unwrap().unwrap();
    assert_eq!(
        stored.current_milestone().map(|id| id.as_str()),
        Some("milestone/p1/0")
    );
    assert_eq!(count(&tx, "milestone"), 2);
}

#[tokio::test]
async fn two_batches_get_global_ordinals_and_unique_ids() {
    let (_dir, db) = support::new_db();
    let product = saved_product(&db, "p1").await;
    let first = propose(&product, &[], &["m0", "m1"]).await;
    let tx = db.begin().unwrap();
    save_milestones(&tx, &product, &first, 100).unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let history = load_milestones(&tx, product.id()).unwrap();
    let product = load_product(&tx, product.id()).unwrap().unwrap();
    let second = propose(&product, &history, &["m2", "m3"]).await;
    save_milestones(&tx, &product, &second, 101).unwrap();
    assert_eq!(seqs(&tx, "p1"), [0, 1, 2, 3]);
    let loaded = load_milestones(&tx, product.id()).unwrap();
    let ids: Vec<&str> = loaded.iter().map(|m| m.id().as_str()).collect();
    assert_eq!(
        ids,
        [
            "milestone/p1/0",
            "milestone/p1/1",
            "milestone/p1/2",
            "milestone/p1/3"
        ]
    );
    let unique: std::collections::BTreeSet<&str> = ids.iter().copied().collect();
    assert_eq!(unique.len(), 4);
}

#[tokio::test]
async fn save_milestones_rejects_an_unsaved_product_without_writing() {
    let (_dir, db) = support::new_db();
    let (_, product) = support::product_with_intent("p1").await;
    let milestones = propose(&product, &[], &["m0"]).await;
    let tx = db.begin().unwrap();
    let err = save_milestones(&tx, &product, &milestones, 100).unwrap_err();
    assert!(matches!(err, ProductError::UnknownProduct { id } if id == "p1"));
    assert_eq!(count(&tx, "milestone"), 0);
}

#[tokio::test]
async fn save_milestones_rejects_a_completed_milestone_without_progress() {
    let (_dir, db) = support::new_db();
    let product = saved_product(&db, "p1").await;
    let first = propose(&product, &[], &["m0"]).await;
    let tx = db.begin().unwrap();
    save_milestones(&tx, &product, &first, 100).unwrap();
    record_milestone_state(
        &tx,
        first[0].id(),
        MilestoneState::Proposed,
        MilestoneState::Completed,
        101,
    )
    .unwrap();

    let product = load_product(&tx, product.id()).unwrap().unwrap();
    let history = load_milestones(&tx, product.id()).unwrap();
    let next = propose(&product, &history, &["m1"]).await;
    let err = save_milestones(&tx, &product, &next, 102).unwrap_err();
    assert!(matches!(
        err,
        ProductError::MissingProgressForCompleted { milestone }
            if milestone == "milestone/p1/0"
    ));
    assert_eq!(count(&tx, "milestone"), 1);
}

#[tokio::test]
async fn load_milestone_round_trips_all_seven_fields() {
    let (_dir, db) = support::new_db();
    let product = saved_product(&db, "p1").await;
    let drafts = batch(vec![
        draft("m0", vec![], vec!["ready"], vec!["done"]),
        draft(
            "m1",
            vec![MilestoneDependency::InBatch(0)],
            vec!["m0 done"],
            vec!["tests pass"],
        ),
    ]);
    let milestones = propose_next_batch(
        &product,
        &[],
        boundary(2, 8),
        &FixedMilestoneGenerator::new(vec![Ok(drafts)]),
    )
    .await
    .unwrap();
    let tx = db.begin().unwrap();
    save_milestones(&tx, &product, &milestones, 100).unwrap();

    let loaded = load_milestone(&tx, milestones[1].id()).unwrap().unwrap();
    assert_eq!(loaded.id(), milestones[1].id());
    assert_eq!(
        loaded.product_id().as_str(),
        milestones[1].product_id().as_str()
    );
    assert_eq!(loaded.objective().as_str(), "m1");
    assert_eq!(loaded.dependencies(), milestones[1].dependencies());
    assert_eq!(
        loaded
            .entry_conditions()
            .iter()
            .map(Opaque::as_str)
            .collect::<Vec<_>>(),
        ["m0 done"]
    );
    assert_eq!(
        loaded
            .exit_conditions()
            .iter()
            .map(Opaque::as_str)
            .collect::<Vec<_>>(),
        ["tests pass"]
    );
    assert_eq!(loaded.state(), MilestoneState::Proposed);
}

#[tokio::test]
async fn all_seven_states_round_trip_through_the_table() {
    let (_dir, db) = support::new_db();
    let product = saved_product(&db, "p1").await;
    let milestones = propose(&product, &[], &["m0"]).await;
    let tx = db.begin().unwrap();
    save_milestones(&tx, &product, &milestones, 100).unwrap();

    let mut current = MilestoneState::Proposed;
    for (i, to) in [
        MilestoneState::Approved,
        MilestoneState::Active,
        MilestoneState::Verifying,
        MilestoneState::Completed,
        MilestoneState::Failed,
        MilestoneState::Superseded,
    ]
    .into_iter()
    .enumerate()
    {
        record_milestone_state(&tx, milestones[0].id(), current, to, 110 + i as i64).unwrap();
        let loaded = load_milestone(&tx, milestones[0].id()).unwrap().unwrap();
        assert_eq!(loaded.state(), to);
        current = to;
    }
}

#[tokio::test]
async fn an_out_of_table_state_is_a_corrupt_row() {
    let (_dir, db) = support::new_db();
    let product = saved_product(&db, "p1").await;
    let milestones = propose(&product, &[], &["m0"]).await;
    let tx = db.begin().unwrap();
    save_milestones(&tx, &product, &milestones, 100).unwrap();
    tx.execute(
        "UPDATE milestone SET state = 'bogus' WHERE id = ?1",
        &[Value::text(milestones[0].id().as_str())],
    )
    .unwrap();
    let err = load_milestone(&tx, milestones[0].id()).unwrap_err();
    assert!(matches!(
        err,
        ProductError::CorruptRow {
            table: "milestone",
            ..
        }
    ));
}

#[tokio::test]
async fn an_empty_objective_or_condition_is_a_corrupt_row() {
    let cases = [
        ("objective = ''", "objective"),
        ("entry_conditions = '[\"\"]'", "entry"),
        ("exit_conditions = '[\"\"]'", "exit"),
    ];
    for (assignment, label) in cases {
        let (_dir, db) = support::new_db();
        let product = saved_product(&db, "p1").await;
        let milestones = propose(&product, &[], &["m0"]).await;
        let tx = db.begin().unwrap();
        save_milestones(&tx, &product, &milestones, 100).unwrap();
        tx.execute(
            &format!("UPDATE milestone SET {assignment} WHERE id = ?1"),
            &[Value::text(milestones[0].id().as_str())],
        )
        .unwrap();
        let err = load_milestone(&tx, milestones[0].id()).unwrap_err();
        assert!(
            matches!(
                err,
                ProductError::CorruptRow {
                    table: "milestone",
                    ..
                }
            ),
            "坏行 {label} 未报 CorruptRow"
        );
    }
}

#[tokio::test]
async fn record_milestone_state_checks_expected_state() {
    let (_dir, db) = support::new_db();
    let product = saved_product(&db, "p1").await;
    let milestones = propose(&product, &[], &["m0"]).await;
    let tx = db.begin().unwrap();
    save_milestones(&tx, &product, &milestones, 100).unwrap();
    let err = record_milestone_state(
        &tx,
        milestones[0].id(),
        MilestoneState::Active,
        MilestoneState::Completed,
        101,
    )
    .unwrap_err();
    assert!(matches!(
        err,
        ProductError::MilestoneStateMismatch { id, expected, actual }
            if id == "milestone/p1/0" && expected == "active" && actual == "proposed"
    ));
    let loaded = load_milestone(&tx, milestones[0].id()).unwrap().unwrap();
    assert_eq!(loaded.state(), MilestoneState::Proposed);
}

#[tokio::test]
async fn completing_the_current_milestone_advances_the_cursor() {
    let (_dir, db) = support::new_db();
    let product = saved_product(&db, "p1").await;
    let milestones = propose(&product, &[], &["m0", "m1", "m2"]).await;
    let tx = db.begin().unwrap();
    save_milestones(&tx, &product, &milestones, 100).unwrap();

    for (index, next) in [
        (0, Some("milestone/p1/1")),
        (1, Some("milestone/p1/2")),
        (2, None),
    ] {
        record_milestone_state(
            &tx,
            milestones[index].id(),
            MilestoneState::Proposed,
            MilestoneState::Completed,
            101,
        )
        .unwrap();
        let product = load_product(&tx, product.id()).unwrap().unwrap();
        assert_eq!(
            product.current_milestone().map(|id| id.as_str()),
            next,
            "完成第 {index} 枚后的游标"
        );
    }
}

#[tokio::test]
async fn set_current_milestone_rejects_an_unknown_product() {
    let (_dir, db) = support::new_db();
    let tx = db.begin().unwrap();
    let err = set_current_milestone(&tx, &ProductIdRef::from_str("ghost"), None, 100)
        .unwrap_err();
    assert!(matches!(err, ProductError::UnknownProduct { id } if id == "ghost"));
}

#[tokio::test]
async fn set_current_milestone_rejects_a_terminal_or_foreign_milestone() {
    let (_dir, db) = support::new_db();
    let product = saved_product(&db, "p1").await;
    let milestones = propose(&product, &[], &["m0"]).await;
    let other = saved_product_with_intent(&db, "p2", "intent/p2").await;
    let other_milestones = propose(&other, &[], &["other"]).await;
    let tx = db.begin().unwrap();
    save_milestones(&tx, &product, &milestones, 100).unwrap();
    save_milestones(&tx, &other, &other_milestones, 100).unwrap();
    let err = set_current_milestone(
        &tx,
        product.id(),
        Some(other_milestones[0].id()),
        101,
    )
    .unwrap_err();
    assert!(matches!(
        err,
        ProductError::MilestoneProductMismatch { .. }
    ));

    record_milestone_state(
        &tx,
        milestones[0].id(),
        MilestoneState::Proposed,
        MilestoneState::Completed,
        102,
    )
    .unwrap();
    let err = set_current_milestone(&tx, product.id(), Some(milestones[0].id()), 103)
        .unwrap_err();
    assert!(matches!(err, ProductError::TerminalMilestone { .. }));
}

#[tokio::test]
async fn dependencies_round_trip_in_order() {
    let (_dir, db) = support::new_db();
    let product = saved_product(&db, "p1").await;
    let drafts = batch(vec![
        plain("m0"),
        plain("m1"),
        plain("m2"),
        draft(
            "m3",
            vec![
                MilestoneDependency::InBatch(1),
                MilestoneDependency::InBatch(2),
            ],
            vec![],
            vec![],
        ),
    ]);
    let milestones = propose_next_batch(
        &product,
        &[],
        boundary(4, 8),
        &FixedMilestoneGenerator::new(vec![Ok(drafts)]),
    )
    .await
    .unwrap();
    let tx = db.begin().unwrap();
    save_milestones(&tx, &product, &milestones, 100).unwrap();

    let loaded = load_milestone(&tx, milestones[3].id()).unwrap().unwrap();
    let dependencies: Vec<&str> = loaded
        .dependencies()
        .iter()
        .map(|id| id.as_str())
        .collect();
    assert_eq!(
        dependencies,
        ["milestone/p1/1", "milestone/p1/2"]
    );
}
