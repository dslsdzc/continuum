//! J18：迁移 200 的四张产品循环表（C2=(b) 已批准）。
//!
//! 照片：版本恰 `[200]`、名称 `p7a_product`；`PRAGMA table_info` 的四张表列清单
//! 逐列照设计 §4.9；`product_drive` 存在（C2=(a) 时才要求它缺席）。

use continuum_persist::{Db, Value};
use continuum_product::p7a_product_migrations;

fn new_db() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("product.db");
    let mut migrations = continuum_persist::builtin_migrations();
    migrations.extend(p7a_product_migrations());
    migrations.extend(continuum_semantics::p4_semantics_migrations());
    let db = Db::open_with(&path, migrations).unwrap();
    db.migrate().unwrap();
    (dir, db)
}

fn text(value: &Value) -> &str {
    match value {
        Value::Text(s) => s,
        other => panic!("应为文本，实际 {other:?}"),
    }
}

fn columns(tx: &continuum_persist::Tx<'_>, table: &str) -> Vec<String> {
    tx.query(&format!("PRAGMA table_info({table})"), &[])
        .unwrap()
        .into_iter()
        .map(|row| text(&row[1]).to_owned())
        .collect()
}

fn table_exists(tx: &continuum_persist::Tx<'_>, table: &str) -> bool {
    !tx.query(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name = ?",
        &[Value::text(table)],
    )
    .unwrap()
    .is_empty()
}

#[test]
fn the_200_schema_has_the_four_product_tables() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    assert_eq!(
        columns(&tx, "product"),
        [
            "id",
            "intent_id",
            "baseline",
            "readiness",
            "current_milestone",
            "shippable_predicate",
            "created_at",
            "updated_at"
        ]
    );
    assert_eq!(
        columns(&tx, "milestone"),
        [
            "id",
            "product_id",
            "seq",
            "objective",
            "dependencies",
            "entry_conditions",
            "exit_conditions",
            "state",
            "created_at",
            "updated_at"
        ]
    );
    assert_eq!(
        columns(&tx, "product_progress"),
        [
            "product_id",
            "milestone_id",
            "previous_product_state",
            "current_product_state",
            "progress_delta",
            "recorded_at"
        ]
    );
    assert_eq!(
        columns(&tx, "product_drive"),
        [
            "product_id",
            "continuous_product_drive",
            "phase",
            "updated_at"
        ]
    );
    assert!(
        table_exists(&tx, "product_drive"),
        "C2=(b) 的第四张表必须存在"
    );
}

#[test]
fn the_200_migration_is_exactly_one_version_named_p7a_product() {
    let migrations = p7a_product_migrations();
    assert_eq!(migrations.len(), 1);
    assert_eq!(migrations[0].version, 200);
    assert_eq!(migrations[0].name, "p7a_product");

    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    let rows = tx
        .query(
            "SELECT version, name FROM schema_migrations WHERE version = 200",
            &[],
        )
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], Value::Int(200));
    assert_eq!(text(&rows[0][1]), "p7a_product");
}
