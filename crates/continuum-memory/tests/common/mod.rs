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
