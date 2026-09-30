use continuum_persist::{builtin_migrations, Db, Migration, Value};

#[test]
fn builtin_migrations_create_event_and_audit_tables() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let db = Db::open(&path).unwrap();
    let applied = db.migrate().unwrap();
    assert_eq!(applied, builtin_migrations().len() as u32);

    let tx = db.begin().unwrap();
    // sqlite_sequence 由 AUTOINCREMENT 自动创建，不属于本项目的表
    let rows = tx
        .query(
            "SELECT name FROM sqlite_master
             WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
            &[],
        )
        .unwrap();
    let names: Vec<String> = rows
        .into_iter()
        .map(|r| match r[0].clone() {
            Value::Text(s) => s,
            other => panic!("表名应为文本，实际 {other:?}"),
        })
        .collect();
    assert_eq!(names, vec!["audit_log", "events", "schema_migrations"]);
}

#[test]
fn migrate_is_idempotent() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let db = Db::open(&path).unwrap();
    assert_eq!(db.migrate().unwrap(), builtin_migrations().len() as u32);
    assert_eq!(db.migrate().unwrap(), 0, "重复迁移不得重复应用");
}

#[test]
fn layer_supplied_migration_is_applied_after_builtin() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let mut all = builtin_migrations();
    all.push(Migration::new(
        100,
        "layer_table",
        "CREATE TABLE layer_table (id TEXT PRIMARY KEY);",
    ));
    let db = Db::open_with(&path, all).unwrap();
    let applied = db.migrate().unwrap();
    assert_eq!(applied, builtin_migrations().len() as u32 + 1);

    let tx = db.begin().unwrap();
    let rows = tx
        .query(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='layer_table'",
            &[],
        )
        .unwrap();
    assert_eq!(rows.len(), 1);
}

#[test]
fn failed_migration_leaves_later_migrations_unapplied() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let mut all = builtin_migrations();
    all.push(Migration::new(101, "broken", "THIS IS NOT SQL;"));
    all.push(Migration::new(102, "never", "CREATE TABLE never (id TEXT);"));
    let db = Db::open_with(&path, all).unwrap();
    let err = db.migrate().expect_err("坏迁移必须返回错误");
    assert!(
        err.to_string().contains("101"),
        "错误信息应含失败版本号，实际: {err}"
    );

    let tx = db.begin().unwrap();
    let rows = tx
        .query(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='never'",
            &[],
        )
        .unwrap();
    assert!(rows.is_empty(), "迁移 102 不得被应用");
}

#[test]
fn wal_mode_is_enabled() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let db = Db::open(&path).unwrap();
    let tx = db.begin().unwrap();
    let rows = tx.query("PRAGMA journal_mode", &[]).unwrap();
    match &rows[0][0] {
        Value::Text(s) => assert_eq!(s, "wal"),
        other => panic!("journal_mode 应为文本，实际 {other:?}"),
    }
}
