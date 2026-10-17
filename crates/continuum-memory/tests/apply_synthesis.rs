//! Task 8：`apply_synthesis` 的新建／合并／退场、摘要失效与作业收尾（设计 §8.4）。
//!
//! 照片：R10–R19b、R31／R33／R35／R36／R37／R39／R40／R41，以及 I5。

use continuum_memory::{
    Confidence, MemoryClaimId, MemoryError, MemoryScope, MemorySourceId, ProposedClaim,
    SynthesisJobId, Validity, apply_synthesis, begin_synthesis, fail_synthesis, insert_source,
    load_current_summary, load_summary, retrieve_claims,
};
use continuum_persist::{Tx, Value};

mod common;

use common::{apply_request, begin_and_apply, begin_request, new_db, source};

fn proposed(
    id: &str,
    statement: &str,
    sources: &[&str],
    confidence: serde_json::Value,
    supersedes: &[&str],
) -> ProposedClaim {
    ProposedClaim {
        id: MemoryClaimId::new(id),
        statement: statement.to_string(),
        sources: sources
            .iter()
            .map(|id| MemorySourceId::new(*id))
            .collect(),
        confidence: Confidence::from_value(confidence),
        supersedes: supersedes.iter().map(|id| MemoryClaimId::new(*id)).collect(),
    }
}

fn personal_query(include_terminal: bool) -> continuum_memory::RetrievalQuery {
    continuum_memory::RetrievalQuery::new(
        vec![MemoryScope::Personal],
        None,
        None,
        100,
        include_terminal,
    )
    .unwrap()
}

fn count(tx: &Tx<'_>, table: &str) -> i64 {
    let rows = tx.query(&format!("SELECT COUNT(*) FROM {table}"), &[]).unwrap();
    match rows[0][0] {
        Value::Int(n) => n,
        ref other => panic!("COUNT 应为整数，实际 {other:?}"),
    }
}

fn job_status(tx: &Tx<'_>, job: &str) -> String {
    let rows = tx
        .query(
            "SELECT status FROM memory_synthesis_job WHERE job_id = ?",
            &[Value::text(job)],
        )
        .unwrap();
    match &rows[0][0] {
        Value::Text(s) => s.clone(),
        other => panic!("status 应为文本，实际 {other:?}"),
    }
}

fn seed_summary(tx: &Tx<'_>, job: &str, id: &str, anchor: &str, generated_at: i64) {
    tx.execute(
        "INSERT INTO memory_synthesis_job
           (job_id, scope, anchor_id, trigger_kind, trigger_ref, status,
            started_at, finished_at, report)
         VALUES (?, 'project', ?, 'user_requested', 'null', 'applied', 0, 0, '{}')",
        &[Value::text(job), Value::text(anchor)],
    )
    .unwrap();
    tx.execute(
        "INSERT INTO memory_summary
           (summary_id, scope, anchor_id, body, job_id, generated_at, stale)
         VALUES (?, 'project', ?, '旧摘要', ?, ?, 0)",
        &[
            Value::text(id),
            Value::text(anchor),
            Value::text(job),
            Value::Int(generated_at),
        ],
    )
    .unwrap();
}

fn claim_rows(tx: &Tx<'_>) -> i64 {
    count(tx, "memory_claim")
}

#[test]
fn a_new_claim_has_seven_fields_and_max_observed_at() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    insert_source(&tx, &source("s1", continuum_memory::SourceKind::Files, "s1", 100)).unwrap();
    insert_source(&tx, &source("s2", continuum_memory::SourceKind::Files, "s2", 300)).unwrap();
    let proposal = apply_request(
        vec![proposed(
            "c1",
            "  Core 不再使用 move  ",
            &["s2", "s1"],
            serde_json::json!({"grade": "high"}),
            &[],
        )],
        None,
    );
    begin_and_apply(
        &tx,
        "j1",
        MemoryScope::Project,
        Some("p1"),
        &["s1", "s2"],
        &proposal,
        400,
    )
    .unwrap();

    let claims = retrieve_claims(
        &tx,
        &continuum_memory::RetrievalQuery::new(
            vec![MemoryScope::Project],
            Some("p1".to_string()),
            None,
            10,
            false,
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(claims.len(), 1);
    let claim = &claims[0];
    assert_eq!(claim.id().as_str(), "c1");
    assert_eq!(claim.scope(), MemoryScope::Project);
    assert_eq!(claim.statement(), "Core 不再使用 move");
    let sources: Vec<&str> = claim.sources().iter().map(MemorySourceId::as_str).collect();
    assert_eq!(sources, ["s1", "s2"]);
    assert_eq!(claim.validity(), Validity::Active);
    assert_eq!(
        claim.confidence().as_value(),
        &serde_json::json!({"grade": "high"})
    );
    assert_eq!(claim.last_confirmed(), 300);
}

#[test]
fn a_duplicate_statement_merges_into_one_claim() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    insert_source(&tx, &source("s1", continuum_memory::SourceKind::Files, "s1", 100)).unwrap();
    insert_source(&tx, &source("s2", continuum_memory::SourceKind::Files, "s2", 300)).unwrap();
    let proposal = apply_request(
        vec![
            proposed("c1", "同一说法", &["s1"], serde_json::json!("low"), &[]),
            proposed(
                "c2",
                "同一说法",
                &["s1", "s2"],
                serde_json::json!("high"),
                &[],
            ),
        ],
        None,
    );
    let report = begin_and_apply(
        &tx,
        "j1",
        MemoryScope::Personal,
        None,
        &["s1", "s2"],
        &proposal,
        400,
    )
    .unwrap();

    assert_eq!(claim_rows(&tx), 1, "同 statement 只能留一行 claim");
    assert_eq!(report.created.len(), 1);
    assert_eq!(report.created[0].as_str(), "c1");
    assert_eq!(report.merged.len(), 1);
    assert_eq!(report.merged[0].as_str(), "c1", "合并目标是第一条的 id");
    let row = tx
        .query(
            "SELECT last_confirmed, confidence FROM memory_claim WHERE claim_id = 'c1'",
            &[],
        )
        .unwrap();
    assert_eq!(row[0][0], Value::Int(300));
    let confidence: serde_json::Value =
        serde_json::from_str(match &row[0][1] {
            Value::Text(s) => s,
            other => panic!("confidence 应为文本，实际 {other:?}"),
        })
        .unwrap();
    assert_eq!(confidence, serde_json::json!("high"), "后一条覆盖 confidence");
    assert_eq!(
        count(&tx, "memory_claim_source"),
        2,
        "两条 source 边应存在"
    );
}

#[test]
fn different_statements_create_two_claims() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    insert_source(&tx, &source("s1", continuum_memory::SourceKind::Files, "s1", 100)).unwrap();
    let proposal = apply_request(
        vec![
            proposed("c1", "说法一", &["s1"], serde_json::json!(null), &[]),
            proposed("c2", "说法二", &["s1"], serde_json::json!(null), &[]),
        ],
        None,
    );
    let report = begin_and_apply(
        &tx,
        "j1",
        MemoryScope::Personal,
        None,
        &["s1"],
        &proposal,
        400,
    )
    .unwrap();
    assert_eq!(report.created.len(), 2);
    assert_eq!(claim_rows(&tx), 2);
}

#[test]
fn a_foreign_source_is_rejected_and_nothing_is_written() {
    let (_dir, db) = new_db();
    {
        let tx = db.begin().unwrap();
        insert_source(&tx, &source("s1", continuum_memory::SourceKind::Files, "s1", 100)).unwrap();
        insert_source(&tx, &source("s2", continuum_memory::SourceKind::Files, "s2", 100)).unwrap();
        let request = begin_request(
            "j1",
            MemoryScope::Personal,
            None,
            continuum_memory::SynthesisTrigger::UserRequested,
            vec![MemorySourceId::new("s1")],
            1,
        );
        begin_synthesis(&tx, &request).unwrap();
        tx.commit().unwrap();
    }
    {
        let tx = db.begin().unwrap();
        let proposal = apply_request(
            vec![proposed("c1", "说法", &["s2"], serde_json::json!(null), &[])],
            None,
        );
        let result = apply_synthesis(&tx, &SynthesisJobId::new("j1"), &proposal, 400);
        assert_eq!(result, Err(MemoryError::ForeignSource));
    } // 丢弃句柄回滚

    let tx = db.begin().unwrap();
    assert_eq!(claim_rows(&tx), 0);
    assert_eq!(count(&tx, "memory_claim_source"), 0);
    assert_eq!(count(&tx, "memory_summary"), 0);
    assert_eq!(job_status(&tx, "j1"), "running");
}

#[test]
fn a_source_removed_between_begin_and_apply_is_rejected() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    insert_source(&tx, &source("s1", continuum_memory::SourceKind::Files, "s1", 100)).unwrap();
    let request = begin_request(
        "j1",
        MemoryScope::Personal,
        None,
        continuum_memory::SynthesisTrigger::UserRequested,
        vec![MemorySourceId::new("s1")],
        1,
    );
    begin_synthesis(&tx, &request).unwrap();
    tx.execute(
        "UPDATE memory_source SET removed_at = 2 WHERE source_id = ?",
        &[Value::text("s1")],
    )
    .unwrap();
    let proposal = apply_request(
        vec![proposed("c1", "说法", &["s1"], serde_json::json!(null), &[])],
        None,
    );
    assert_eq!(
        apply_synthesis(&tx, &SynthesisJobId::new("j1"), &proposal, 400),
        Err(MemoryError::SourceRemoved)
    );
    assert_eq!(job_status(&tx, "j1"), "running");
}

#[test]
fn an_empty_proposal_source_set_is_rejected() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    insert_source(&tx, &source("s1", continuum_memory::SourceKind::Files, "s1", 100)).unwrap();
    let proposal = apply_request(
        vec![proposed("c1", "说法", &[], serde_json::json!(null), &[])],
        None,
    );
    let result = begin_and_apply(
        &tx,
        "j1",
        MemoryScope::Personal,
        None,
        &["s1"],
        &proposal,
        400,
    );
    assert_eq!(result, Err(MemoryError::EmptySourceSet));
}

#[test]
fn a_second_apply_of_the_same_job_is_rejected() {
    let (_dir, db) = new_db();
    {
        let tx = db.begin().unwrap();
        insert_source(&tx, &source("s1", continuum_memory::SourceKind::Files, "s1", 100)).unwrap();
        let proposal = apply_request(
            vec![proposed("c1", "说法", &["s1"], serde_json::json!(null), &[])],
            None,
        );
        begin_and_apply(
            &tx,
            "j1",
            MemoryScope::Personal,
            None,
            &["s1"],
            &proposal,
            400,
        )
        .unwrap();
        tx.commit().unwrap();
    }

    let tx = db.begin().unwrap();
    let proposal = apply_request(vec![], None);
    assert_eq!(
        apply_synthesis(&tx, &SynthesisJobId::new("j1"), &proposal, 500),
        Err(MemoryError::JobNotRunning)
    );

    insert_source(&tx, &source("s2", continuum_memory::SourceKind::Files, "s2", 100)).unwrap();
    let request = begin_request(
        "j2",
        MemoryScope::Personal,
        None,
        continuum_memory::SynthesisTrigger::UserRequested,
        vec![MemorySourceId::new("s2")],
        600,
    );
    begin_synthesis(&tx, &request).unwrap();
    fail_synthesis(&tx, &SynthesisJobId::new("j2"), "failed", 700).unwrap();
    assert_eq!(
        apply_synthesis(&tx, &SynthesisJobId::new("j2"), &proposal, 800),
        Err(MemoryError::JobNotRunning)
    );
}

#[test]
fn supersedes_another_anchor_is_rejected() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    insert_source(&tx, &source("s1", continuum_memory::SourceKind::Files, "s1", 100)).unwrap();
    let first = apply_request(
        vec![proposed(
            "c-other",
            "p2 的说法",
            &["s1"],
            serde_json::json!(null),
            &[],
        )],
        None,
    );
    begin_and_apply(
        &tx,
        "j1",
        MemoryScope::Project,
        Some("p2"),
        &["s1"],
        &first,
        400,
    )
    .unwrap();

    let second = apply_request(
        vec![proposed(
            "c-new",
            "p1 的说法",
            &["s1"],
            serde_json::json!(null),
            &["c-other"],
        )],
        None,
    );
    let result = begin_and_apply(
        &tx,
        "j2",
        MemoryScope::Project,
        Some("p1"),
        &["s1"],
        &second,
        400,
    );
    assert_eq!(result, Err(MemoryError::ScopeMismatch));
}

#[test]
fn superseding_then_reproposing_creates_a_new_active_claim() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    insert_source(&tx, &source("s1", continuum_memory::SourceKind::Files, "s1", 100)).unwrap();

    let first = apply_request(
        vec![proposed("X", "旧说法", &["s1"], serde_json::json!(null), &[])],
        None,
    );
    begin_and_apply(
        &tx,
        "j1",
        MemoryScope::Personal,
        None,
        &["s1"],
        &first,
        100,
    )
    .unwrap();

    let second = apply_request(
        vec![proposed("Y", "新说法", &["s1"], serde_json::json!(null), &["X"])],
        None,
    );
    begin_and_apply(
        &tx,
        "j2",
        MemoryScope::Personal,
        None,
        &["s1"],
        &second,
        200,
    )
    .unwrap();

    let third = apply_request(
        vec![proposed("Z", "旧说法", &["s1"], serde_json::json!(null), &[])],
        None,
    );
    begin_and_apply(
        &tx,
        "j3",
        MemoryScope::Personal,
        None,
        &["s1"],
        &third,
        300,
    )
    .unwrap();

    assert_eq!(claim_rows(&tx), 3);
    let claims = retrieve_claims(&tx, &personal_query(true)).unwrap();
    let by_id = |id: &str| claims.iter().find(|c| c.id().as_str() == id).unwrap();
    assert_eq!(by_id("X").validity(), Validity::Superseded);
    assert_eq!(by_id("Y").validity(), Validity::Active);
    assert_eq!(by_id("Z").validity(), Validity::Active);
    let active = retrieve_claims(&tx, &personal_query(false)).unwrap();
    assert_eq!(active.len(), 2);
    assert!(active.iter().any(|c| c.id().as_str() == "Z"));
}

#[test]
fn a_summary_is_written_and_old_ones_are_staled() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    insert_source(&tx, &source("s1", continuum_memory::SourceKind::Files, "s1", 100)).unwrap();
    seed_summary(&tx, "old-job", "old-summary", "p1", 100);

    let proposal = apply_request(
        vec![proposed("c1", "新说法", &["s1"], serde_json::json!(null), &[])],
        Some("新摘要"),
    );
    let report = begin_and_apply(
        &tx,
        "j1",
        MemoryScope::Project,
        Some("p1"),
        &["s1"],
        &proposal,
        400,
    )
    .unwrap();

    let latest = load_summary(&tx, MemoryScope::Project, Some("p1"))
        .unwrap()
        .unwrap();
    assert_eq!(latest.id.as_str(), "j1:400");
    assert!(!latest.stale);
    assert_eq!(latest.job.as_str(), "j1");
    assert_eq!(latest.body, "新摘要");
    let old = tx
        .query(
            "SELECT stale FROM memory_summary WHERE summary_id = 'old-summary'",
            &[],
        )
        .unwrap();
    assert_eq!(old[0][0], Value::Int(1), "旧摘要应被置 stale");
    assert_eq!(report.staled_summaries, 1);
    assert!(report.summary_written);
}

#[test]
fn no_summary_stales_the_old_one_and_current_summary_is_none() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    insert_source(&tx, &source("s1", continuum_memory::SourceKind::Files, "s1", 100)).unwrap();
    seed_summary(&tx, "old-job", "old-summary", "p1", 100);

    let proposal = apply_request(
        vec![proposed("c1", "新说法", &["s1"], serde_json::json!(null), &[])],
        None,
    );
    let report = begin_and_apply(
        &tx,
        "j1",
        MemoryScope::Project,
        Some("p1"),
        &["s1"],
        &proposal,
        400,
    )
    .unwrap();

    assert_eq!(
        load_current_summary(&tx, MemoryScope::Project, Some("p1")).unwrap(),
        None
    );
    let old = load_summary(&tx, MemoryScope::Project, Some("p1"))
        .unwrap()
        .unwrap();
    assert!(old.stale);
    assert_eq!(report.staled_summaries, 1);
    assert!(!report.summary_written);
}

#[test]
fn no_summary_without_claim_changes_leaves_the_summary_fresh() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    insert_source(&tx, &source("s1", continuum_memory::SourceKind::Files, "s1", 100)).unwrap();
    seed_summary(&tx, "old-job", "old-summary", "p1", 100);

    let proposal = apply_request(vec![], None);
    let report = begin_and_apply(
        &tx,
        "j1",
        MemoryScope::Project,
        Some("p1"),
        &["s1"],
        &proposal,
        400,
    )
    .unwrap();

    assert!(report.created.is_empty());
    assert!(report.merged.is_empty());
    assert!(report.superseded.is_empty());
    assert_eq!(report.staled_summaries, 0);
    assert!(!report.summary_written);
    let current = load_current_summary(&tx, MemoryScope::Project, Some("p1"))
        .unwrap()
        .unwrap();
    assert_eq!(current.id.as_str(), "old-summary");
    assert_eq!(job_status(&tx, "j1"), "applied");
}

#[test]
fn duplicate_source_ids_in_a_proposal_are_rejected() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    insert_source(&tx, &source("s1", continuum_memory::SourceKind::Files, "s1", 100)).unwrap();
    let proposal = apply_request(
        vec![proposed("c1", "说法", &["s1", "s1"], serde_json::json!(null), &[])],
        None,
    );
    let result = begin_and_apply(
        &tx,
        "j1",
        MemoryScope::Personal,
        None,
        &["s1"],
        &proposal,
        400,
    );
    assert_eq!(result, Err(MemoryError::DuplicateSourceId));
}

#[test]
fn an_unknown_source_in_a_proposal_is_rejected() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    insert_source(&tx, &source("s1", continuum_memory::SourceKind::Files, "s1", 100)).unwrap();
    let proposal = apply_request(
        vec![proposed("c1", "说法", &["nope"], serde_json::json!(null), &[])],
        None,
    );
    let result = begin_and_apply(
        &tx,
        "j1",
        MemoryScope::Personal,
        None,
        &["s1"],
        &proposal,
        400,
    );
    assert_eq!(result, Err(MemoryError::UnknownSource));
}

#[test]
fn an_empty_statement_is_rejected() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    insert_source(&tx, &source("s1", continuum_memory::SourceKind::Files, "s1", 100)).unwrap();
    let proposal = apply_request(
        vec![proposed("c1", "   ", &["s1"], serde_json::json!(null), &[])],
        None,
    );
    let result = begin_and_apply(
        &tx,
        "j1",
        MemoryScope::Personal,
        None,
        &["s1"],
        &proposal,
        400,
    );
    assert_eq!(result, Err(MemoryError::EmptyStatement));
}

#[test]
fn duplicate_claim_ids_are_rejected() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    insert_source(&tx, &source("s1", continuum_memory::SourceKind::Files, "s1", 100)).unwrap();
    let proposal = apply_request(
        vec![
            proposed("c1", "说法一", &["s1"], serde_json::json!(null), &[]),
            proposed("c1", "说法二", &["s1"], serde_json::json!(null), &[]),
        ],
        None,
    );
    let result = begin_and_apply(
        &tx,
        "j1",
        MemoryScope::Personal,
        None,
        &["s1"],
        &proposal,
        400,
    );
    assert_eq!(result, Err(MemoryError::DuplicateClaimId));

    // 与库里已有 claim 同 id。
    let first = apply_request(
        vec![proposed("existing", "已有", &["s1"], serde_json::json!(null), &[])],
        None,
    );
    begin_and_apply(
        &tx,
        "j2",
        MemoryScope::Personal,
        None,
        &["s1"],
        &first,
        500,
    )
    .unwrap();
    let second = apply_request(
        vec![proposed("existing", "另一条", &["s1"], serde_json::json!(null), &[])],
        None,
    );
    let result = begin_and_apply(
        &tx,
        "j3",
        MemoryScope::Personal,
        None,
        &["s1"],
        &second,
        600,
    );
    assert_eq!(result, Err(MemoryError::DuplicateClaimId));
}

#[test]
fn supersedes_an_unknown_claim_is_rejected() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    insert_source(&tx, &source("s1", continuum_memory::SourceKind::Files, "s1", 100)).unwrap();
    let proposal = apply_request(
        vec![proposed(
            "c1",
            "说法",
            &["s1"],
            serde_json::json!(null),
            &["nope"],
        )],
        None,
    );
    let result = begin_and_apply(
        &tx,
        "j1",
        MemoryScope::Personal,
        None,
        &["s1"],
        &proposal,
        400,
    );
    assert_eq!(result, Err(MemoryError::UnknownClaim));
}

#[test]
fn supersedes_a_terminal_claim_is_rejected() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    insert_source(&tx, &source("s1", continuum_memory::SourceKind::Files, "s1", 100)).unwrap();
    let first = apply_request(
        vec![proposed("X", "旧", &["s1"], serde_json::json!(null), &[])],
        None,
    );
    begin_and_apply(
        &tx,
        "j1",
        MemoryScope::Personal,
        None,
        &["s1"],
        &first,
        100,
    )
    .unwrap();
    let second = apply_request(
        vec![proposed("Y", "新", &["s1"], serde_json::json!(null), &["X"])],
        None,
    );
    begin_and_apply(
        &tx,
        "j2",
        MemoryScope::Personal,
        None,
        &["s1"],
        &second,
        200,
    )
    .unwrap();
    let third = apply_request(
        vec![proposed("Z", "更新", &["s1"], serde_json::json!(null), &["X"])],
        None,
    );
    let result = begin_and_apply(
        &tx,
        "j3",
        MemoryScope::Personal,
        None,
        &["s1"],
        &third,
        300,
    );
    assert_eq!(
        result,
        Err(MemoryError::ClaimNotActive {
            validity: Validity::Superseded
        })
    );
}

#[test]
fn an_empty_summary_is_rejected() {
    let (_dir, db) = new_db();
    {
        let tx = db.begin().unwrap();
        insert_source(&tx, &source("s1", continuum_memory::SourceKind::Files, "s1", 100)).unwrap();
        let request = begin_request(
            "j1",
            MemoryScope::Personal,
            None,
            continuum_memory::SynthesisTrigger::UserRequested,
            vec![MemorySourceId::new("s1")],
            1,
        );
        begin_synthesis(&tx, &request).unwrap();
        tx.commit().unwrap();
    }
    {
        let tx = db.begin().unwrap();
        let proposal = apply_request(
            vec![proposed("c1", "说法", &["s1"], serde_json::json!(null), &[])],
            Some("   "),
        );
        let result = apply_synthesis(&tx, &SynthesisJobId::new("j1"), &proposal, 400);
        assert_eq!(result, Err(MemoryError::EmptySummary));
    }

    let tx = db.begin().unwrap();
    assert_eq!(job_status(&tx, "j1"), "running");
    assert_eq!(claim_rows(&tx), 0, "整笔回滚后 claim 不得落库");
}

#[test]
fn supersedes_the_merge_target_is_rejected() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    insert_source(&tx, &source("s1", continuum_memory::SourceKind::Files, "s1", 100)).unwrap();
    let first = apply_request(
        vec![proposed("X", "同一说法", &["s1"], serde_json::json!(null), &[])],
        None,
    );
    begin_and_apply(
        &tx,
        "j1",
        MemoryScope::Personal,
        None,
        &["s1"],
        &first,
        100,
    )
    .unwrap();

    let second = apply_request(
        vec![proposed(
            "Y",
            "同一说法",
            &["s1"],
            serde_json::json!(null),
            &["X"],
        )],
        None,
    );
    let result = begin_and_apply(
        &tx,
        "j2",
        MemoryScope::Personal,
        None,
        &["s1"],
        &second,
        200,
    );
    assert_eq!(result, Err(MemoryError::SupersedeMergeConflict));
}

#[test]
fn apply_to_an_unknown_or_failed_job_is_rejected() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    let proposal = apply_request(vec![], None);
    assert_eq!(
        apply_synthesis(&tx, &SynthesisJobId::new("nope"), &proposal, 1),
        Err(MemoryError::UnknownJob)
    );

    insert_source(&tx, &source("s1", continuum_memory::SourceKind::Files, "s1", 100)).unwrap();
    let request = begin_request(
        "j1",
        MemoryScope::Personal,
        None,
        continuum_memory::SynthesisTrigger::UserRequested,
        vec![MemorySourceId::new("s1")],
        1,
    );
    begin_synthesis(&tx, &request).unwrap();
    fail_synthesis(&tx, &SynthesisJobId::new("j1"), "x", 2).unwrap();
    assert_eq!(
        apply_synthesis(&tx, &SynthesisJobId::new("j1"), &proposal, 3),
        Err(MemoryError::JobNotRunning)
    );
}

#[test]
fn the_report_counts_created_merged_superseded_and_staled() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    insert_source(&tx, &source("s1", continuum_memory::SourceKind::Files, "s1", 100)).unwrap();

    // 先建两个旧 claim：一个将被合并，一个将被退场。
    let seed = apply_request(
        vec![
            proposed("M", "待合并", &["s1"], serde_json::json!(null), &[]),
            proposed("X", "待退场", &["s1"], serde_json::json!(null), &[]),
        ],
        None,
    );
    begin_and_apply(
        &tx,
        "j0",
        MemoryScope::Project,
        Some("p1"),
        &["s1"],
        &seed,
        100,
    )
    .unwrap();
    seed_summary(&tx, "old-job-1", "old-1", "p1", 10);
    seed_summary(&tx, "old-job-2", "old-2", "p1", 20);

    let proposal = apply_request(
        vec![
            proposed("NEW", "全新", &["s1"], serde_json::json!(null), &[]),
            proposed(
                "M2",
                "待合并",
                &["s1"],
                serde_json::json!(null),
                &["X"],
            ),
        ],
        Some("新摘要"),
    );
    let report = begin_and_apply(
        &tx,
        "j1",
        MemoryScope::Project,
        Some("p1"),
        &["s1"],
        &proposal,
        400,
    )
    .unwrap();

    assert_eq!(report.created.len(), 1);
    assert_eq!(report.created[0].as_str(), "NEW");
    assert_eq!(report.merged.len(), 1);
    assert_eq!(report.merged[0].as_str(), "M");
    assert_eq!(report.superseded.len(), 1);
    assert_eq!(report.superseded[0].as_str(), "X");
    assert!(report.summary_written);
    assert_eq!(report.staled_summaries, 2);

    let row = tx
        .query(
            "SELECT status, finished_at, report FROM memory_synthesis_job WHERE job_id = 'j1'",
            &[],
        )
        .unwrap();
    assert_eq!(row[0][0], Value::text("applied"));
    assert_eq!(row[0][1], Value::Int(400));
    let stored: serde_json::Value = serde_json::from_str(match &row[0][2] {
        Value::Text(s) => s,
        other => panic!("report 应为文本，实际 {other:?}"),
    })
    .unwrap();
    assert_eq!(stored["created"], serde_json::json!(["NEW"]));
    assert_eq!(stored["merged"], serde_json::json!(["M"]));
    assert_eq!(stored["superseded"], serde_json::json!(["X"]));
    assert_eq!(stored["summary_written"], serde_json::json!(true));
    assert_eq!(stored["staled_summaries"], serde_json::json!(2));
}

#[test]
fn a_failed_apply_leaves_the_job_running() {
    let (_dir, db) = new_db();
    {
        let tx = db.begin().unwrap();
        insert_source(&tx, &source("s1", continuum_memory::SourceKind::Files, "s1", 100)).unwrap();
        let request = begin_request(
            "j1",
            MemoryScope::Personal,
            None,
            continuum_memory::SynthesisTrigger::UserRequested,
            vec![MemorySourceId::new("s1")],
            1,
        );
        begin_synthesis(&tx, &request).unwrap();
        tx.commit().unwrap();
    }
    {
        let tx = db.begin().unwrap();
        let proposal = apply_request(
            vec![
                proposed("c1", "合法", &["s1"], serde_json::json!(null), &[]),
                proposed("c2", "   ", &["s1"], serde_json::json!(null), &[]),
            ],
            None,
        );
        let result = apply_synthesis(&tx, &SynthesisJobId::new("j1"), &proposal, 400);
        assert_eq!(result, Err(MemoryError::EmptyStatement));
    }

    let tx = db.begin().unwrap();
    assert_eq!(job_status(&tx, "j1"), "running");
    assert_eq!(claim_rows(&tx), 0, "整笔回滚后第一条 claim 也不得存在");
}

#[test]
fn apply_does_not_create_or_delete_sources_or_rewrite_claims() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    insert_source(&tx, &source("s1", continuum_memory::SourceKind::Files, "s1", 100)).unwrap();
    insert_source(&tx, &source("s2", continuum_memory::SourceKind::Files, "s2", 200)).unwrap();
    let first = apply_request(
        vec![proposed("X", "合并目标", &["s1"], serde_json::json!(null), &[])],
        None,
    );
    begin_and_apply(
        &tx,
        "j1",
        MemoryScope::Personal,
        None,
        &["s1"],
        &first,
        100,
    )
    .unwrap();
    let before_sources = count(&tx, "memory_source");

    let second = apply_request(
        vec![proposed(
            "Y",
            "合并目标",
            &["s1", "s2"],
            serde_json::json!("new"),
            &[],
        )],
        None,
    );
    begin_and_apply(
        &tx,
        "j2",
        MemoryScope::Personal,
        None,
        &["s1", "s2"],
        &second,
        200,
    )
    .unwrap();

    assert_eq!(count(&tx, "memory_source"), before_sources);
    let row = tx
        .query(
            "SELECT claim_id, scope, statement FROM memory_claim WHERE claim_id = 'X'",
            &[],
        )
        .unwrap();
    assert_eq!(row[0][0], Value::text("X"));
    assert_eq!(row[0][1], Value::text("personal"));
    assert_eq!(row[0][2], Value::text("合并目标"));
}
