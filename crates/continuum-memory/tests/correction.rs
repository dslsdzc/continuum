//! Task 10：纠正的九步与 `load_correction`（设计 §7.2／§7.3）。
//!
//! 照片：R5–R9、R24、R26、R29、R30。

use continuum_memory::{
    Confidence, Correction, CorrectionRecord, MemoryClaim, MemoryClaimId, MemoryError,
    MemoryScope, MemorySourceId, ProposedClaim, SourceKind, Validity, apply_synthesis,
    begin_synthesis, correct, insert_source, load_correction, load_current_summary, load_sources,
    retrieve_claims,
};
use continuum_persist::{Tx, Value};

mod common;

use common::{apply_request, begin_request, new_db, source};

fn run_job(
    tx: &Tx<'_>,
    job: &str,
    scope: MemoryScope,
    anchor: Option<&str>,
    sources: &[&str],
    claims: Vec<ProposedClaim>,
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
        100,
    )
    .unwrap();
}

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

fn seed_project_claim(tx: &Tx<'_>) {
    insert_source(tx, &source("s1", SourceKind::Files, "s1", 100)).unwrap();
    insert_source(tx, &source("s2", SourceKind::Files, "s2", 200)).unwrap();
    run_job(
        tx,
        "seed-job",
        MemoryScope::Project,
        Some("p1"),
        &["s1", "s2"],
        vec![proposed("X", "旧说法", &["s1", "s2"])],
    );
}

fn correction(statement: &str, correction_source: continuum_memory::MemorySource) -> Correction {
    Correction {
        claim: MemoryClaimId::new("X"),
        statement: statement.to_string(),
        source: correction_source,
        occurred_at: 500,
    }
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

/// 经公开检索面读一条 claim（含终态）；本文件所有 claim 都在 `(Project, p1)`。
fn read_claim(tx: &Tx<'_>, id: &str) -> MemoryClaim {
    let q = continuum_memory::RetrievalQuery::new(
        vec![MemoryScope::Project],
        Some("p1".to_string()),
        None,
        100,
        true,
    )
    .unwrap();
    retrieve_claims(tx, &q)
        .unwrap()
        .into_iter()
        .find(|c| c.id().as_str() == id)
        .expect("claim 应存在")
}

fn correction_table_count(tx: &Tx<'_>) -> i64 {
    let rows = tx.query("SELECT COUNT(*) FROM memory_correction", &[]).unwrap();
    match rows[0][0] {
        Value::Int(n) => n,
        ref other => panic!("COUNT 应为整数，实际 {other:?}"),
    }
}

fn claim_count(tx: &Tx<'_>) -> i64 {
    let rows = tx.query("SELECT COUNT(*) FROM memory_claim", &[]).unwrap();
    match rows[0][0] {
        Value::Int(n) => n,
        ref other => panic!("COUNT 应为整数，实际 {other:?}"),
    }
}

#[test]
fn a_correction_writes_a_new_source_and_supersedes_the_old_claim() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    seed_project_claim(&tx);
    let before = read_claim(&tx, "X");
    let claims_before = claim_count(&tx);

    let correction_source = source(
        "corr",
        SourceKind::ExplicitUserCorrections,
        "sess-corr",
        600,
    );
    let outcome = correct(&tx, &correction(" 新说法 ", correction_source.clone())).unwrap();

    assert_eq!(outcome.correction_source.as_str(), "corr");
    assert_eq!(outcome.superseded.as_str(), "X");
    let stored = load_sources(&tx, &[MemorySourceId::new("corr")])
        .unwrap();
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].kind(), SourceKind::ExplicitUserCorrections);

    let after = read_claim(&tx, "X");
    assert_eq!(after.validity(), Validity::Superseded);
    assert_eq!(after.sources(), before.sources(), "旧 claim 的 sources[] 不得变");
    assert_eq!(after.statement(), "旧说法");
    assert_eq!(claim_count(&tx), claims_before, "纠正不得创建 claim");

    let rows = tx
        .query(
            "SELECT statement FROM memory_correction WHERE claim_id = 'X'",
            &[],
        )
        .unwrap();
    assert_eq!(
        rows[0][0],
        Value::text(" 新说法 "),
        "correction.statement 原样存储（含首尾空白）"
    );
}

#[test]
fn a_second_correction_is_rejected() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    seed_project_claim(&tx);
    correct(
        &tx,
        &correction(
            "新说法",
            source("corr1", SourceKind::ExplicitUserCorrections, "c1", 1),
        ),
    )
    .unwrap();
    assert_eq!(
        correct(
            &tx,
            &correction(
                "又一次",
                source("corr2", SourceKind::ExplicitUserCorrections, "c2", 2),
            ),
        ),
        Err(MemoryError::ClaimNotActive {
            validity: Validity::Superseded
        })
    );
}

#[test]
fn a_correction_needs_the_correction_source_kind() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    seed_project_claim(&tx);
    let attempts = correct(
        &tx,
        &correction("新说法", source("f1", SourceKind::Files, "f1", 1)),
    );
    assert_eq!(
        attempts,
        Err(MemoryError::WrongCorrectionSourceKind {
            kind: SourceKind::Files
        })
    );
    assert_eq!(correction_table_count(&tx), 0);
    assert_eq!(
        read_claim(&tx, "X").validity(),
        Validity::Active
    );
}

#[test]
fn the_database_keeps_one_correction_per_claim() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    seed_project_claim(&tx);
    correct(
        &tx,
        &correction(
            "第一次",
            source("corr1", SourceKind::ExplicitUserCorrections, "c1", 1),
        ),
    )
    .unwrap();

    insert_source(
        &tx,
        &source("corr2", SourceKind::ExplicitUserCorrections, "c2", 2),
    )
    .unwrap();
    let result = tx.execute(
        "INSERT INTO memory_correction (source_id, claim_id, statement, occurred_at)
         VALUES ('corr2', 'X', '第二次', 3)",
        &[],
    );
    assert!(result.is_err(), "claim_id UNIQUE 应由数据库挡住第二次纠正");
    let rows = tx
        .query(
            "SELECT statement FROM memory_correction WHERE claim_id = 'X'",
            &[],
        )
        .unwrap();
    assert_eq!(rows[0][0], Value::text("第一次"));
}

#[test]
fn a_correction_stales_only_its_scope_summary() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    seed_project_claim(&tx);
    insert_summary(&tx, "p-job", "p1-summary", MemoryScope::Project, Some("p1"));
    insert_summary(&tx, "me-job", "personal-summary", MemoryScope::Personal, None);

    let outcome = correct(
        &tx,
        &correction(
            "新说法",
            source("corr", SourceKind::ExplicitUserCorrections, "c", 1),
        ),
    )
    .unwrap();

    assert_eq!(outcome.staled_summaries, 1);
    assert_eq!(
        load_current_summary(&tx, MemoryScope::Project, Some("p1")).unwrap(),
        None
    );
    assert!(
        load_current_summary(&tx, MemoryScope::Personal, None)
            .unwrap()
            .is_some()
    );
}

#[test]
fn a_correction_does_not_create_a_claim() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    seed_project_claim(&tx);
    let before = claim_count(&tx);
    correct(
        &tx,
        &correction(
            "新说法",
            source("corr", SourceKind::ExplicitUserCorrections, "c", 1),
        ),
    )
    .unwrap();

    assert_eq!(claim_count(&tx), before);
    let rows = tx
        .query(
            "SELECT COUNT(*) FROM memory_claim WHERE statement = '新说法'",
            &[],
        )
        .unwrap();
    assert_eq!(rows[0][0], Value::Int(0), "新说法不得变成 claim");
    let q = continuum_memory::RetrievalQuery::new(
        vec![MemoryScope::Project],
        Some("p1".to_string()),
        None,
        10,
        false,
    )
    .unwrap();
    assert_eq!(retrieve_claims(&tx, &q).unwrap(), vec![]);
}

#[test]
fn load_correction_returns_the_new_statement() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    seed_project_claim(&tx);
    correct(
        &tx,
        &correction(
            "新说法",
            source("corr", SourceKind::ExplicitUserCorrections, "c", 1),
        ),
    )
    .unwrap();

    assert_eq!(
        load_correction(&tx, &MemoryClaimId::new("X")).unwrap(),
        Some(CorrectionRecord {
            correction_source: MemorySourceId::new("corr"),
            claim: MemoryClaimId::new("X"),
            statement: "新说法".to_string(),
            occurred_at: 500,
        })
    );
    assert_eq!(
        load_correction(&tx, &MemoryClaimId::new("nope")).unwrap(),
        None
    );
}

#[test]
fn an_unknown_claim_is_rejected() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    assert_eq!(
        correct(
            &tx,
            &Correction {
                claim: MemoryClaimId::new("nope"),
                statement: "新说法".to_string(),
                source: source("corr", SourceKind::ExplicitUserCorrections, "c", 1),
                occurred_at: 1,
            }
        ),
        Err(MemoryError::UnknownClaim)
    );
}

#[test]
fn an_empty_statement_is_rejected() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    seed_project_claim(&tx);
    assert_eq!(
        correct(
            &tx,
            &correction(
                "   ",
                source("corr", SourceKind::ExplicitUserCorrections, "c", 1),
            )
        ),
        Err(MemoryError::EmptyStatement)
    );
    assert_eq!(correction_table_count(&tx), 0);
    assert_eq!(
        read_claim(&tx, "X").validity(),
        Validity::Active
    );
}

#[test]
fn a_duplicate_correction_source_is_rejected_without_side_effects() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    seed_project_claim(&tx);
    insert_source(
        &tx,
        &source("corr", SourceKind::ExplicitUserCorrections, "c", 1),
    )
    .unwrap();

    assert_eq!(
        correct(
            &tx,
            &correction(
                "新说法",
                source("corr", SourceKind::ExplicitUserCorrections, "c", 2),
            )
        ),
        Err(MemoryError::DuplicateSourceId)
    );
    assert_eq!(correction_table_count(&tx), 0);
    assert_eq!(
        read_claim(&tx, "X").validity(),
        Validity::Active
    );
}

#[test]
fn the_correction_statement_can_be_proposed_in_the_next_synthesis() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    seed_project_claim(&tx);
    correct(
        &tx,
        &correction(
            "  新说法  ",
            source("corr", SourceKind::ExplicitUserCorrections, "sess", 1),
        ),
    )
    .unwrap();

    let record = load_correction(&tx, &MemoryClaimId::new("X"))
        .unwrap()
        .unwrap();
    let proposal = ProposedClaim {
        id: MemoryClaimId::new("X2"),
        statement: record.statement.clone(),
        sources: vec![record.correction_source.clone()],
        confidence: Confidence::from_value(serde_json::json!("high")),
        supersedes: vec![],
    };
    let request = begin_request(
        "correction-job",
        MemoryScope::Project,
        Some("p1"),
        continuum_memory::SynthesisTrigger::UserRequested,
        vec![record.correction_source.clone()],
        0,
    );
    begin_synthesis(&tx, &request).unwrap();
    apply_synthesis(
        &tx,
        &continuum_memory::SynthesisJobId::new("correction-job"),
        &apply_request(vec![proposal], None),
        600,
    )
    .unwrap();

    let claim = read_claim(&tx, "X2");
    assert_eq!(claim.statement(), "新说法", "下一次合成用 trim 后的纠正文本");
    assert_eq!(claim.sources(), &[record.correction_source]);
    assert_eq!(claim.validity(), Validity::Active);
}

#[test]
fn a_correction_does_not_append_to_the_old_claim_sources() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    seed_project_claim(&tx);
    let before = read_claim(&tx, "X");
    correct(
        &tx,
        &correction(
            "新说法",
            source("corr", SourceKind::ExplicitUserCorrections, "c", 1),
        ),
    )
    .unwrap();
    let after = read_claim(&tx, "X");
    assert_eq!(after.sources(), before.sources());
    assert!(
        !after.sources().iter().any(|s| s.as_str() == "corr"),
        "correction source 不得挂到旧 claim 的 sources[]"
    );
    let rows = tx
        .query(
            "SELECT source_id FROM memory_correction WHERE claim_id = 'X'",
            &[],
        )
        .unwrap();
    assert_eq!(rows[0][0], Value::text("corr"));
}
