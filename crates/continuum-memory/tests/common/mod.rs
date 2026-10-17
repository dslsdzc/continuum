//! 记忆块集成测试的共用夹具。
//!
//! 本文件被多个 `tests/*.rs` 以 `mod common;` 收进；每个测试 crate 只用到其中一部分，
//! 故整文件允许 dead_code。
#![allow(dead_code)]

use continuum_persist::Db;

/// 起一个只带记忆迁移（190）的真库并应用迁移。
pub fn new_db() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("memory.db");
    let db = Db::open_with(&path, continuum_memory::p6c_memory_migrations()).unwrap();
    db.migrate().unwrap();
    (dir, db)
}

/// `MemorySource` 夹具。
pub fn source(
    id: &str,
    kind: continuum_memory::SourceKind,
    external_ref: &str,
    observed_at: i64,
) -> continuum_memory::MemorySource {
    continuum_memory::MemorySource::new(
        continuum_memory::MemorySourceId::new(id),
        kind,
        external_ref,
        observed_at,
    )
    .unwrap()
}

/// `SynthesisRequest` 夹具。
pub fn begin_request(
    job: &str,
    scope: continuum_memory::MemoryScope,
    anchor: Option<&str>,
    trigger: continuum_memory::SynthesisTrigger,
    sources: Vec<continuum_memory::MemorySourceId>,
    started_at: i64,
) -> continuum_memory::SynthesisRequest {
    continuum_memory::SynthesisRequest {
        job: continuum_memory::SynthesisJobId::new(job),
        scope,
        anchor: anchor.map(str::to_string),
        trigger,
        sources,
        started_at,
    }
}

/// 直接 SQL 写一条 claim、它的 source 边与（anchored 时）锚点侧表行。
///
/// 只给检索面夹具使用：绕过 `apply_synthesis` 是为了让 Task 7 不依赖 Task 8。
pub fn seed_claim(
    tx: &continuum_persist::Tx<'_>,
    id: &str,
    scope: continuum_memory::MemoryScope,
    statement: &str,
    validity: continuum_memory::Validity,
    last_confirmed: i64,
    anchor: Option<&str>,
    source_ids: &[&str],
) {
    for source_id in source_ids {
        let exists = tx
            .query(
                "SELECT 1 FROM memory_source WHERE source_id = ?",
                &[continuum_persist::Value::text(*source_id)],
            )
            .unwrap();
        if exists.is_empty() {
            let source = source(
                source_id,
                continuum_memory::SourceKind::Files,
                source_id,
                1,
            );
            continuum_memory::insert_source(tx, &source).unwrap();
        }
    }
    tx.execute(
        "INSERT INTO memory_claim
           (claim_id, scope, statement, validity, confidence, last_confirmed)
         VALUES (?, ?, ?, ?, ?, ?)",
        &[
            continuum_persist::Value::text(id),
            continuum_persist::Value::text(scope.as_str()),
            continuum_persist::Value::text(statement),
            continuum_persist::Value::text(validity.as_str()),
            continuum_persist::Value::text("null"),
            continuum_persist::Value::Int(last_confirmed),
        ],
    )
    .unwrap();
    for source_id in source_ids {
        tx.execute(
            "INSERT INTO memory_claim_source (claim_id, source_id) VALUES (?, ?)",
            &[
                continuum_persist::Value::text(id),
                continuum_persist::Value::text(*source_id),
            ],
        )
        .unwrap();
    }
    if let Some(anchor) = anchor {
        tx.execute(
            "INSERT INTO memory_claim_anchor (claim_id, anchor_id) VALUES (?, ?)",
            &[
                continuum_persist::Value::text(id),
                continuum_persist::Value::text(anchor),
            ],
        )
        .unwrap();
    }
}
