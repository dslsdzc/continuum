//! Task 6：`MemorySummary` 的读面（设计 §6.2／§8.4）。
//!
//! 照片：R28（最新／current 的 stale 语义）、R32（摘要入口的 scope/anchor 规则）。
//! `write_summary`／`mark_summaries_stale` 是 `pub(crate)`，其用例在 `src/summary.rs`。

use continuum_memory::{
    MemoryError, MemoryScope, load_current_summary, load_summary,
};
use continuum_persist::{Tx, Value};

mod common;

use common::new_db;

fn insert_job(tx: &Tx<'_>, id: &str) {
    tx.execute(
        "INSERT INTO memory_synthesis_job
           (job_id, scope, anchor_id, trigger_kind, trigger_ref, status,
            started_at, finished_at, report)
         VALUES (?, 'personal', NULL, 'user_requested', 'null', 'running', 0, NULL, NULL)",
        &[Value::text(id)],
    )
    .unwrap();
}

fn insert_summary(
    tx: &Tx<'_>,
    id: &str,
    scope: MemoryScope,
    anchor: Option<&str>,
    generated_at: i64,
    stale: i64,
) {
    insert_job(tx, &format!("job-{id}"));
    tx.execute(
        "INSERT INTO memory_summary
           (summary_id, scope, anchor_id, body, job_id, generated_at, stale)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
        &[
            Value::text(id),
            Value::text(scope.as_str()),
            match anchor {
                Some(a) => Value::text(a),
                None => Value::Null,
            },
            Value::text(format!("正文-{id}")),
            Value::text(format!("job-{id}")),
            Value::Int(generated_at),
            Value::Int(stale),
        ],
    )
    .unwrap();
}

#[test]
fn load_summary_returns_the_latest_by_generated_at_then_id() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    insert_summary(
        &tx,
        "old",
        MemoryScope::Project,
        Some("p1"),
        100,
        0,
    );
    insert_summary(
        &tx,
        "new",
        MemoryScope::Project,
        Some("p1"),
        200,
        0,
    );
    let latest = load_summary(&tx, MemoryScope::Project, Some("p1"))
        .unwrap()
        .expect("应有摘要");
    assert_eq!(latest.id.as_str(), "new");

    // 同 generated_at 时取 id 字典序更小者。
    insert_summary(
        &tx,
        "aaa",
        MemoryScope::Project,
        Some("p1"),
        200,
        0,
    );
    let latest = load_summary(&tx, MemoryScope::Project, Some("p1"))
        .unwrap()
        .unwrap();
    assert_eq!(latest.id.as_str(), "aaa");
}

#[test]
fn load_current_summary_skips_stale_rows() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    insert_summary(
        &tx,
        "fresh",
        MemoryScope::Project,
        Some("p1"),
        100,
        0,
    );
    insert_summary(
        &tx,
        "stale",
        MemoryScope::Project,
        Some("p1"),
        200,
        1,
    );

    let current = load_current_summary(&tx, MemoryScope::Project, Some("p1"))
        .unwrap()
        .expect("应返回最新的 fresh 摘要");
    assert_eq!(current.id.as_str(), "fresh");

    let any = load_summary(&tx, MemoryScope::Project, Some("p1"))
        .unwrap()
        .unwrap();
    assert_eq!(any.id.as_str(), "stale", "load_summary 可返回 stale 行");
    assert!(any.stale);
}

#[test]
fn all_stale_yields_none() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    insert_summary(&tx, "a", MemoryScope::Personal, None, 100, 1);
    insert_summary(&tx, "b", MemoryScope::Personal, None, 200, 1);
    assert_eq!(
        load_current_summary(&tx, MemoryScope::Personal, None).unwrap(),
        None
    );
}

#[test]
fn empty_table_reads_as_none() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    assert_eq!(
        load_summary(&tx, MemoryScope::Project, Some("p1")).unwrap(),
        None
    );
    assert_eq!(
        load_current_summary(&tx, MemoryScope::Personal, None).unwrap(),
        None
    );
}

#[test]
fn summary_scope_anchor_rules_are_enforced() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    assert_eq!(
        load_summary(&tx, MemoryScope::Project, None),
        Err(MemoryError::AnchorRequired)
    );
    assert_eq!(
        load_current_summary(&tx, MemoryScope::Project, None),
        Err(MemoryError::AnchorRequired)
    );
    assert_eq!(
        load_summary(&tx, MemoryScope::Personal, Some("p")),
        Err(MemoryError::AnchorNotAllowed)
    );
    assert_eq!(
        load_current_summary(&tx, MemoryScope::Personal, Some("p")),
        Err(MemoryError::AnchorNotAllowed)
    );
    assert_eq!(load_summary(&tx, MemoryScope::Project, Some("p")), Ok(None));
    assert_eq!(load_current_summary(&tx, MemoryScope::Personal, None), Ok(None));
}
