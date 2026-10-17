//! Task 2：迁移 190 的八表六索引（设计 §9.2）。
//!
//! 照片：R1（版本恰 `[190]`）、S5（`Migration::new(190` 只在 `persist.rs` 一次）、
//! S7（`memory_claim` 恰六列）、S9（八表各恰一个 `CREATE TABLE`）。

use continuum_memory::p6c_memory_migrations;
use continuum_persist::Value;

mod common;

use common::new_db;

const TABLES: [&str; 8] = [
    "memory_source",
    "memory_claim",
    "memory_claim_source",
    "memory_claim_anchor",
    "memory_correction",
    "memory_synthesis_job",
    "memory_synthesis_input",
    "memory_summary",
];

const INDEXES: [&str; 6] = [
    "idx_memory_source_kind",
    "idx_memory_source_ref",
    "idx_memory_claim_source_source",
    "idx_memory_claim_scope_validity_time",
    "idx_memory_claim_anchor",
    "idx_memory_summary_scope_time",
];

fn text(v: &Value) -> &str {
    match v {
        Value::Text(s) => s,
        other => panic!("应为文本，实际 {other:?}"),
    }
}

#[test]
fn the_migration_is_the_only_producer_of_version_190() {
    let migrations = p6c_memory_migrations();
    let versions: Vec<i64> = migrations.iter().map(|m| m.version).collect();
    assert_eq!(versions, [190], "版本必须恰为 [190]");
    assert_eq!(migrations[0].name, "p6c_memory");

    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    let rows = tx
        .query(
            "SELECT version, name FROM schema_migrations WHERE version = 190",
            &[],
        )
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], Value::Int(190));
    assert_eq!(text(&rows[0][1]), "p6c_memory");
}

#[test]
fn the_migration_creates_eight_tables_and_six_indexes() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    for table in TABLES {
        let rows = tx
            .query(
                "SELECT name FROM sqlite_master WHERE type = 'table' AND name = ?",
                &[Value::text(table)],
            )
            .unwrap();
        assert_eq!(rows.len(), 1, "缺表 {table}");
    }
    for index in INDEXES {
        let rows = tx
            .query(
                "SELECT name FROM sqlite_master WHERE type = 'index' AND name = ?",
                &[Value::text(index)],
            )
            .unwrap();
        assert_eq!(rows.len(), 1, "缺索引 {index}");
    }
}

#[test]
fn memory_claim_has_exactly_six_scalar_columns() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    let rows = tx.query("PRAGMA table_info(memory_claim)", &[]).unwrap();
    let columns: Vec<String> = rows
        .iter()
        .map(|r| text(&r[1]).to_string())
        .collect();
    assert_eq!(
        columns,
        [
            "claim_id",
            "scope",
            "statement",
            "validity",
            "confidence",
            "last_confirmed"
        ],
        "memory_claim 的标量列必须恰为六列"
    );
    for forbidden in ["status", "created_at", "anchor_id"] {
        assert!(
            !columns.iter().any(|c| c == forbidden),
            "memory_claim 不得有 {forbidden} 列"
        );
    }
}

#[test]
fn each_of_the_eight_tables_has_exactly_one_create_table() {
    let sql = include_str!("../src/persist.rs");
    for table in TABLES {
        // 边界取「表名 + 空格 + 左括号」：`memory_claim` 是 `memory_claim_source`
        // 的前缀，裸表名计数会把后者的 `CREATE TABLE` 也算进来。
        let needle = format!("CREATE TABLE {table} (");
        assert_eq!(
            sql.matches(&needle).count(),
            1,
            "{table} 在迁移 SQL 里应恰有一个 `{needle}`"
        );
    }
}

#[test]
fn migration_190_is_declared_once_in_persist_rs() {
    let sql = include_str!("../src/persist.rs");
    assert_eq!(
        sql.matches("Migration::new(190").count(),
        1,
        "`Migration::new(190` 应恰出现一次"
    );
    assert_eq!(
        sql.matches("pub fn p6c_memory_migrations").count(),
        1,
        "`p6c_memory_migrations` 应恰定义一个 `pub fn`"
    );
}

#[test]
fn the_reference_tables_declare_their_foreign_keys() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    let expected = [
        ("memory_claim_source", 2),
        ("memory_claim_anchor", 1),
        ("memory_correction", 2),
        ("memory_synthesis_input", 2),
        ("memory_summary", 1),
    ];
    for (table, count) in expected {
        let rows = tx
            .query(&format!("PRAGMA foreign_key_list({table})"), &[])
            .unwrap();
        assert_eq!(rows.len(), count, "{table} 的外键数不符");
    }
}
