//! Task 5：合成作业的 `begin_synthesis` 与 `fail_synthesis`（设计 §8.1／§8.2）。
//!
//! 照片：R31／R32／R33／R35／R38／R39／R42／R43。

use continuum_memory::{
    MemoryError, MemoryScope, MemorySourceId, SourceKind, SynthesisJobId, SynthesisStatus,
    SynthesisTrigger, begin_synthesis, fail_synthesis,
};
use continuum_persist::Value;

mod common;

use common::{begin_request, new_db, source};

fn job_ids(tx: &continuum_persist::Tx<'_>) -> Vec<String> {
    tx.query(
        "SELECT source_id FROM memory_synthesis_input ORDER BY source_id",
        &[],
    )
    .unwrap()
    .iter()
    .map(|r| match &r[0] {
        Value::Text(s) => s.clone(),
        other => panic!("source_id 应为文本，实际 {other:?}"),
    })
    .collect()
}

#[test]
fn begin_writes_a_running_job_and_its_inputs() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    for id in ["s1", "s2"] {
        continuum_memory::insert_source(&tx, &source(id, SourceKind::Files, id, 10)).unwrap();
    }
    let request = begin_request(
        "j1",
        MemoryScope::Project,
        Some("p1"),
        SynthesisTrigger::UserRequested,
        vec![MemorySourceId::new("s1"), MemorySourceId::new("s2")],
        100,
    );
    begin_synthesis(&tx, &request).unwrap();

    let rows = tx
        .query(
            "SELECT status, scope, anchor_id, trigger_kind, trigger_ref, started_at,
                    finished_at, report
               FROM memory_synthesis_job WHERE job_id = ?",
            &[Value::text("j1")],
        )
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], Value::text("running"));
    assert_eq!(rows[0][1], Value::text("project"));
    assert_eq!(rows[0][2], Value::text("p1"));
    assert_eq!(rows[0][3], Value::text("user_requested"));
    assert_eq!(rows[0][4], Value::text("null"));
    assert_eq!(rows[0][5], Value::Int(100));
    assert_eq!(rows[0][6], Value::Null);
    assert_eq!(rows[0][7], Value::Null);
    assert_eq!(job_ids(&tx), ["s1", "s2"]);
}

#[test]
fn synthesis_status_parse_does_not_default() {
    assert_eq!(SynthesisStatus::parse("unknown"), None);
    assert_eq!(SynthesisStatus::parse("RUNNING"), None);
    assert_eq!(SynthesisStatus::parse(""), None);
    for status in [
        SynthesisStatus::Running,
        SynthesisStatus::Applied,
        SynthesisStatus::Failed,
    ] {
        assert_eq!(SynthesisStatus::parse(status.as_str()), Some(status));
    }
}

#[test]
fn begin_rejects_an_empty_source_set() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    let request = begin_request(
        "j1",
        MemoryScope::Personal,
        None,
        SynthesisTrigger::UserRequested,
        vec![],
        1,
    );
    assert_eq!(
        begin_synthesis(&tx, &request),
        Err(MemoryError::EmptySourceSet)
    );
}

#[test]
fn begin_rejects_duplicate_source_ids() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    continuum_memory::insert_source(&tx, &source("s1", SourceKind::Files, "s1", 1)).unwrap();
    let request = begin_request(
        "j1",
        MemoryScope::Personal,
        None,
        SynthesisTrigger::UserRequested,
        vec![MemorySourceId::new("s1"), MemorySourceId::new("s1")],
        1,
    );
    assert_eq!(
        begin_synthesis(&tx, &request),
        Err(MemoryError::DuplicateSourceId)
    );
}

#[test]
fn begin_rejects_unknown_and_removed_sources() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    let request = begin_request(
        "j1",
        MemoryScope::Personal,
        None,
        SynthesisTrigger::UserRequested,
        vec![MemorySourceId::new("nope")],
        1,
    );
    assert_eq!(begin_synthesis(&tx, &request), Err(MemoryError::UnknownSource));

    continuum_memory::insert_source(&tx, &source("s1", SourceKind::Files, "s1", 1)).unwrap();
    tx.execute(
        "UPDATE memory_source SET removed_at = 2 WHERE source_id = ?",
        &[Value::text("s1")],
    )
    .unwrap();
    let request = begin_request(
        "j2",
        MemoryScope::Personal,
        None,
        SynthesisTrigger::UserRequested,
        vec![MemorySourceId::new("s1")],
        1,
    );
    assert_eq!(begin_synthesis(&tx, &request), Err(MemoryError::SourceRemoved));
}

#[test]
fn begin_rejects_a_trigger_source_outside_the_input() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    for id in ["s1", "s2"] {
        continuum_memory::insert_source(&tx, &source(id, SourceKind::Conversations, id, 1)).unwrap();
    }
    let request = begin_request(
        "j1",
        MemoryScope::Personal,
        None,
        SynthesisTrigger::SessionEnded {
            session: MemorySourceId::new("s2"),
        },
        vec![MemorySourceId::new("s1")],
        1,
    );
    assert_eq!(
        begin_synthesis(&tx, &request),
        Err(MemoryError::TriggerSourceOutsideInput)
    );
}

#[test]
fn begin_rejects_a_session_trigger_of_the_wrong_kind() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    continuum_memory::insert_source(&tx, &source("f1", SourceKind::Files, "f1", 1)).unwrap();
    let request = begin_request(
        "j1",
        MemoryScope::Personal,
        None,
        SynthesisTrigger::SessionEnded {
            session: MemorySourceId::new("f1"),
        },
        vec![MemorySourceId::new("f1")],
        1,
    );
    assert_eq!(
        begin_synthesis(&tx, &request),
        Err(MemoryError::WrongTriggerSourceKind)
    );
}

#[test]
fn begin_rejects_a_bulk_trigger_of_the_wrong_kind() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    continuum_memory::insert_source(&tx, &source("c1", SourceKind::Conversations, "c1", 1)).unwrap();
    let request = begin_request(
        "j1",
        MemoryScope::Personal,
        None,
        SynthesisTrigger::BulkProjectStateChanged {
            changed: vec![MemorySourceId::new("c1")],
        },
        vec![MemorySourceId::new("c1")],
        1,
    );
    assert_eq!(
        begin_synthesis(&tx, &request),
        Err(MemoryError::WrongTriggerSourceKind)
    );
}

#[test]
fn begin_rejects_an_empty_bulk_trigger() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    continuum_memory::insert_source(&tx, &source("s1", SourceKind::Files, "s1", 1)).unwrap();
    let request = begin_request(
        "j1",
        MemoryScope::Personal,
        None,
        SynthesisTrigger::BulkProjectStateChanged { changed: vec![] },
        vec![MemorySourceId::new("s1")],
        1,
    );
    assert_eq!(
        begin_synthesis(&tx, &request),
        Err(MemoryError::EmptyTriggerSet)
    );
}

#[test]
fn begin_rejects_an_idle_trigger_that_starts_after_the_job() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    continuum_memory::insert_source(&tx, &source("s1", SourceKind::Files, "s1", 1)).unwrap();
    let request = begin_request(
        "j1",
        MemoryScope::Personal,
        None,
        SynthesisTrigger::SystemIdle { since: 11 },
        vec![MemorySourceId::new("s1")],
        10,
    );
    assert_eq!(
        begin_synthesis(&tx, &request),
        Err(MemoryError::InvalidTriggerTime)
    );

    let request = begin_request(
        "j2",
        MemoryScope::Personal,
        None,
        SynthesisTrigger::SystemIdle { since: 10 },
        vec![MemorySourceId::new("s1")],
        10,
    );
    begin_synthesis(&tx, &request).unwrap();
}

#[test]
fn begin_rejects_a_duplicate_job_id() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    for id in ["s1", "s2"] {
        continuum_memory::insert_source(&tx, &source(id, SourceKind::Files, id, 1)).unwrap();
    }
    let first = begin_request(
        "j1",
        MemoryScope::Personal,
        None,
        SynthesisTrigger::UserRequested,
        vec![MemorySourceId::new("s1")],
        1,
    );
    begin_synthesis(&tx, &first).unwrap();
    let second = begin_request(
        "j1",
        MemoryScope::Personal,
        None,
        SynthesisTrigger::UserRequested,
        vec![MemorySourceId::new("s2")],
        2,
    );
    assert_eq!(
        begin_synthesis(&tx, &second),
        Err(MemoryError::DuplicateJobId)
    );
    assert_eq!(job_ids(&tx), ["s1"], "第一条 job 的输入集合不得变");
}

#[test]
fn begin_applies_scope_anchor_rules() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    continuum_memory::insert_source(&tx, &source("s1", SourceKind::Files, "s1", 1)).unwrap();
    let sources = vec![MemorySourceId::new("s1")];

    let request = begin_request(
        "j1",
        MemoryScope::Project,
        None,
        SynthesisTrigger::UserRequested,
        sources.clone(),
        1,
    );
    assert_eq!(
        begin_synthesis(&tx, &request),
        Err(MemoryError::AnchorRequired)
    );

    let request = begin_request(
        "j2",
        MemoryScope::Personal,
        Some("p"),
        SynthesisTrigger::UserRequested,
        sources.clone(),
        1,
    );
    assert_eq!(
        begin_synthesis(&tx, &request),
        Err(MemoryError::AnchorNotAllowed)
    );

    let request = begin_request(
        "j3",
        MemoryScope::Project,
        Some("p"),
        SynthesisTrigger::UserRequested,
        sources.clone(),
        1,
    );
    begin_synthesis(&tx, &request).unwrap();

    let request = begin_request(
        "j4",
        MemoryScope::Personal,
        None,
        SynthesisTrigger::UserRequested,
        sources,
        1,
    );
    begin_synthesis(&tx, &request).unwrap();
}

#[test]
fn begin_records_the_four_triggers() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    for id in ["s1", "p1", "p2"] {
        let kind = if id == "s1" {
            SourceKind::Conversations
        } else {
            SourceKind::Project
        };
        continuum_memory::insert_source(&tx, &source(id, kind, id, 1)).unwrap();
    }

    let cases = [
        (
            "j-session",
            SynthesisTrigger::SessionEnded {
                session: MemorySourceId::new("s1"),
            },
            vec![MemorySourceId::new("s1")],
            "session_ended",
            r#"{"session":"s1"}"#,
        ),
        (
            "j-idle",
            SynthesisTrigger::SystemIdle { since: 5 },
            vec![MemorySourceId::new("s1")],
            "system_idle",
            r#"{"since":5}"#,
        ),
        (
            "j-bulk",
            SynthesisTrigger::BulkProjectStateChanged {
                changed: vec![
                    MemorySourceId::new("p2"),
                    MemorySourceId::new("p1"),
                ],
            },
            vec![
                MemorySourceId::new("p2"),
                MemorySourceId::new("p1"),
            ],
            "bulk_project_state_changed",
            r#"{"changed":["p2","p1"]}"#,
        ),
        (
            "j-user",
            SynthesisTrigger::UserRequested,
            vec![MemorySourceId::new("s1")],
            "user_requested",
            "null",
        ),
    ];

    for (job, trigger, sources, kind, reference) in cases {
        let request = begin_request(
            job,
            MemoryScope::Personal,
            None,
            trigger,
            sources,
            10,
        );
        begin_synthesis(&tx, &request).unwrap();
        let rows = tx
            .query(
                "SELECT trigger_kind, trigger_ref FROM memory_synthesis_job WHERE job_id = ?",
                &[Value::text(job)],
            )
            .unwrap();
        assert_eq!(rows[0][0], Value::text(kind), "trigger_kind 不符：{job}");
        assert_eq!(
            rows[0][1],
            Value::text(reference),
            "trigger_ref 不符：{job}"
        );
    }
}

#[test]
fn fail_marks_a_running_job_failed() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    continuum_memory::insert_source(&tx, &source("s1", SourceKind::Files, "s1", 1)).unwrap();
    let request = begin_request(
        "j1",
        MemoryScope::Personal,
        None,
        SynthesisTrigger::UserRequested,
        vec![MemorySourceId::new("s1")],
        1,
    );
    begin_synthesis(&tx, &request).unwrap();
    fail_synthesis(&tx, &SynthesisJobId::new("j1"), "proposal 无效", 99).unwrap();

    let rows = tx
        .query(
            "SELECT status, finished_at, report FROM memory_synthesis_job WHERE job_id = ?",
            &[Value::text("j1")],
        )
        .unwrap();
    assert_eq!(rows[0][0], Value::text("failed"));
    assert_eq!(rows[0][1], Value::Int(99));
    let report = match &rows[0][2] {
        Value::Text(s) => serde_json::from_str::<serde_json::Value>(s).unwrap(),
        other => panic!("report 应为文本，实际 {other:?}"),
    };
    assert_eq!(report, serde_json::json!({"error": "proposal 无效"}));
}

#[test]
fn fail_rejects_unknown_and_finished_jobs() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    assert_eq!(
        fail_synthesis(&tx, &SynthesisJobId::new("nope"), "x", 1),
        Err(MemoryError::UnknownJob)
    );

    continuum_memory::insert_source(&tx, &source("s1", SourceKind::Files, "s1", 1)).unwrap();
    for (job, terminal) in [("j-applied", "applied"), ("j-failed", "failed")] {
        let request = begin_request(
            job,
            MemoryScope::Personal,
            None,
            SynthesisTrigger::UserRequested,
            vec![MemorySourceId::new("s1")],
            1,
        );
        begin_synthesis(&tx, &request).unwrap();
        tx.execute(
            "UPDATE memory_synthesis_job SET status = ? WHERE job_id = ?",
            &[Value::text(terminal), Value::text(job)],
        )
        .unwrap();
        assert_eq!(
            fail_synthesis(&tx, &SynthesisJobId::new(job), "x", 1),
            Err(MemoryError::JobNotRunning)
        );
    }
}
