//! Task 9：删除 Source 的软删除与派生 claim 的无条件失效（设计 §4.4）。
//!
//! 照片：R3（失效与不误伤两侧）、R4（追溯可见／上下文不可见）、R9（另一侧）、
//! R33／R34。

use continuum_memory::{
    Confidence, MemoryClaimId, MemoryError, MemoryScope, MemorySourceId, ProposedClaim, Validity,
    apply_synthesis, begin_synthesis, insert_source, load_current_summary, load_sources,
    remove_source, retrieve_claims,
};
use continuum_persist::{Tx, Value};

mod common;

use common::{apply_request, begin_request, new_db, source};

fn proposed(id: &str, statement: &str, sources: &[&str]) -> ProposedClaim {
    ProposedClaim {
        id: MemoryClaimId::new(id),
        statement: statement.to_string(),
        sources: sources
            .iter()
            .map(|id| MemorySourceId::new(*id))
            .collect(),
        confidence: Confidence::from_value(serde_json::json!(null)),
        supersedes: vec![],
    }
}

fn run_job(
    tx: &Tx<'_>,
    job: &str,
    scope: MemoryScope,
    anchor: Option<&str>,
    sources: &[&str],
    claims: Vec<ProposedClaim>,
    occurred_at: i64,
) {
    let request = begin_request(
        job,
        scope,
        anchor,
        continuum_memory::SynthesisTrigger::UserRequested,
        sources
            .iter()
            .map(|id| MemorySourceId::new(*id))
            .collect(),
        0,
    );
    begin_synthesis(tx, &request).unwrap();
    apply_synthesis(
        tx,
        &continuum_memory::SynthesisJobId::new(job),
        &apply_request(claims, None),
        occurred_at,
    )
    .unwrap();
}

fn query(include_terminal: bool) -> continuum_memory::RetrievalQuery {
    continuum_memory::RetrievalQuery::new(
        vec![MemoryScope::Personal],
        None,
        None,
        100,
        include_terminal,
    )
    .unwrap()
}

fn insert_summary(tx: &Tx<'_>, job: &str, id: &str, scope: MemoryScope, anchor: Option<&str>) {
    tx.execute(
        "INSERT INTO memory_synthesis_job
           (job_id, scope, anchor_id, trigger_kind, trigger_ref, status,
            started_at, finished_at, report)
         VALUES (?, ?, ?, 'user_requested', 'null', 'applied', 0, 0, '{}')",
        &[
            Value::text(job),
            Value::text(scope.as_str()),
            match anchor {
                Some(a) => Value::text(a),
                None => Value::Null,
            },
        ],
    )
    .unwrap();
    tx.execute(
        "INSERT INTO memory_summary
           (summary_id, scope, anchor_id, body, job_id, generated_at, stale)
         VALUES (?, ?, ?, '摘要', ?, 100, 0)",
        &[
            Value::text(id),
            Value::text(scope.as_str()),
            match anchor {
                Some(a) => Value::text(a),
                None => Value::Null,
            },
            Value::text(job),
        ],
    )
    .unwrap();
}

fn seed_three_claims(tx: &Tx<'_>) {
    insert_source(tx, &source("A", continuum_memory::SourceKind::Files, "A", 1)).unwrap();
    insert_source(tx, &source("B", continuum_memory::SourceKind::Files, "B", 2)).unwrap();
    run_job(
        tx,
        "seed",
        MemoryScope::Personal,
        None,
        &["A", "B"],
        vec![
            proposed("X", "只引用 A", &["A"]),
            proposed("Y", "引用 A B", &["A", "B"]),
            proposed("Z", "只引用 B", &["B"]),
        ],
        100,
    );
}

#[test]
fn removing_a_source_invalidates_every_claim_that_references_it_and_no_other() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    seed_three_claims(&tx);

    let removed = remove_source(&tx, &MemorySourceId::new("A"), 200).unwrap();
    assert_eq!(
        removed.iter().map(MemoryClaimId::as_str).collect::<Vec<_>>(),
        ["X", "Y"]
    );

    let claims = retrieve_claims(&tx, &query(true)).unwrap();
    let by_id = |id: &str| claims.iter().find(|c| c.id().as_str() == id).unwrap();
    assert_eq!(by_id("X").validity(), Validity::Invalidated);
    assert_eq!(by_id("Y").validity(), Validity::Invalidated);
    assert_eq!(by_id("Z").validity(), Validity::Active);
}

#[test]
fn a_removed_source_stays_traceable_but_hides_its_claims() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    seed_three_claims(&tx);
    remove_source(&tx, &MemorySourceId::new("A"), 200).unwrap();

    let sources = load_sources(&tx, &[MemorySourceId::new("A")]).unwrap();
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0].removed_at(), Some(200));

    let current = retrieve_claims(&tx, &query(false)).unwrap();
    assert_eq!(
        current.iter().map(|c| c.id().as_str()).collect::<Vec<_>>(),
        ["Z"],
        "默认检索不得返回失效 claim"
    );

    let terminal = retrieve_claims(&tx, &query(true)).unwrap();
    for id in ["X", "Y"] {
        let claim = terminal.iter().find(|c| c.id().as_str() == id).unwrap();
        assert!(
            claim.sources().iter().any(|s| s.as_str() == "A"),
            "{id} 的 sources[] 应仍能追溯已删 source"
        );
    }
}

#[test]
fn remove_is_not_idempotent_by_silence() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    seed_three_claims(&tx);
    remove_source(&tx, &MemorySourceId::new("A"), 200).unwrap();
    assert_eq!(
        remove_source(&tx, &MemorySourceId::new("A"), 300),
        Err(MemoryError::SourceAlreadyRemoved)
    );
}

#[test]
fn remove_rejects_an_unknown_source() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    assert_eq!(
        remove_source(&tx, &MemorySourceId::new("nope"), 200),
        Err(MemoryError::UnknownSource)
    );
}

#[test]
fn remove_stales_the_summaries_of_the_affected_pairs() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    insert_source(&tx, &source("A", continuum_memory::SourceKind::Files, "A", 1)).unwrap();
    run_job(
        &tx,
        "project-job",
        MemoryScope::Project,
        Some("p1"),
        &["A"],
        vec![proposed("X", "p1 的 claim", &["A"])],
        100,
    );
    insert_summary(&tx, "s1-job", "p1-summary", MemoryScope::Project, Some("p1"));
    insert_summary(&tx, "s2-job", "personal-summary", MemoryScope::Personal, None);

    remove_source(&tx, &MemorySourceId::new("A"), 200).unwrap();

    assert_eq!(
        load_current_summary(&tx, MemoryScope::Project, Some("p1")).unwrap(),
        None,
        "p1 的摘要应被置 stale"
    );
    assert!(
        load_current_summary(&tx, MemoryScope::Personal, None)
            .unwrap()
            .is_some(),
        "无关 pair 的摘要不得被误伤"
    );
}

#[test]
fn remove_does_not_touch_terminal_claims() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    insert_source(&tx, &source("A", continuum_memory::SourceKind::Files, "A", 1)).unwrap();
    run_job(
        &tx,
        "j1",
        MemoryScope::Personal,
        None,
        &["A"],
        vec![proposed("X", "旧", &["A"])],
        100,
    );
    let mut second = proposed("Y", "新", &["A"]);
    second.supersedes.push(MemoryClaimId::new("X"));
    run_job(
        &tx,
        "j2",
        MemoryScope::Personal,
        None,
        &["A"],
        vec![second],
        200,
    );

    let removed = remove_source(&tx, &MemorySourceId::new("A"), 300).unwrap();
    assert_eq!(
        removed.iter().map(MemoryClaimId::as_str).collect::<Vec<_>>(),
        ["Y"],
        "返回值不得含已终态的 X"
    );
    let claims = retrieve_claims(&tx, &query(true)).unwrap();
    let x = claims.iter().find(|c| c.id().as_str() == "X").unwrap();
    assert_eq!(x.validity(), Validity::Superseded, "终态 claim 不得被改成 Invalidated");
}

#[test]
fn remove_returns_only_the_claims_it_invalidated() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    insert_source(&tx, &source("A", continuum_memory::SourceKind::Files, "A", 1)).unwrap();
    run_job(
        &tx,
        "j1",
        MemoryScope::Personal,
        None,
        &["A"],
        vec![
            proposed("X", "将被退场", &["A"]),
            proposed("W", "仍 Active", &["A"]),
        ],
        100,
    );
    let mut second = proposed("Y", "替代 X", &["A"]);
    second.supersedes.push(MemoryClaimId::new("X"));
    run_job(
        &tx,
        "j2",
        MemoryScope::Personal,
        None,
        &["A"],
        vec![second],
        200,
    );

    let removed = remove_source(&tx, &MemorySourceId::new("A"), 300).unwrap();
    assert_eq!(
        removed.iter().map(MemoryClaimId::as_str).collect::<Vec<_>>(),
        ["W", "Y"],
        "只返回本次从 Active 改成 Invalidated 的 claim（升序）"
    );
}
