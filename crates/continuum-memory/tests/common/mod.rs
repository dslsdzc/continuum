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
