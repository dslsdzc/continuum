//! 三张表的建出与列结构（设计 §3.1、§3.3）。
//!
//! 本文件的夹具单独成立：它只依赖 `p3d_model_migrations` 与 `Db`，建库后必须
//! `migrate()` —— 漏掉那一行时各用例会一起挂在 `no such table`，而报错位置指向
//! 被测函数，容易误判成实现的问题（同 P3A 的 `continuum-capability/tests/persist.rs`）。

use continuum_model_registry::p3d_model_migrations;
use continuum_persist::{Db, Migration, PersistError, Tx, Value, builtin_migrations};

fn db() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let mut migrations: Vec<Migration> = builtin_migrations();
    migrations.extend(p3d_model_migrations());
    let db = Db::open_with(&path, migrations).unwrap();
    db.migrate().unwrap();
    (dir, db)
}

fn text_of(v: &Value) -> String {
    match v {
        Value::Text(s) => s.clone(),
        other => panic!("列应为文本，实际 {other:?}"),
    }
}

/// `model_profile` 的列清单，**逐列**钉住（设计 §2.3 第 2 条的结构性事实在库侧的落点）。
///
/// 十一条而不是 §247 的十二字段：**`skill_vector` 不在这张表里**，它是
/// `model_skill_score` 那张键控时间序列表（同一维度的多个版本），一个列装不下
/// （设计 §3.1 的 SQL 是权威取值）。故「十二字段」与「十一列」都对，
/// 说的是两件事——**本条钉的是列清单**。
///
/// 逐列的判据是「加一列即红」：清单用 `assert_eq!` 全量比对，不是「包含」。
#[test]
fn the_model_profile_columns_are_exactly_the_eleven_columns() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();
    let names = column_names(&tx, "model_profile");
    assert_eq!(
        names,
        vec![
            "id",
            "version",
            "provider",
            "model_revision",
            "modalities",
            "tools",
            "failure_modes",
            "cost_profile",
            "latency_profile",
            "evidence_count",
            "confidence",
        ],
        "model_profile 的列清单应与设计 §3.1 逐列相同（加一列、改名或换序都在此变红）"
    );

    tx.commit().unwrap();
}

/// 三张表各查得到。
///
/// 对照臂：一个没建过的表名**查不到**——否则「查得到」这一侧只是「SQL 语法没写错」，
/// 把 `sqlite_master` 的查询条件写松（比如漏掉 `type = 'table'`）也无从发现。
#[test]
fn the_three_tables_exist() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    for table in ["model_registry", "model_profile", "model_skill_score"] {
        assert_eq!(
            table_exists(&tx, table),
            1,
            "{table} 应由迁移 80 建出"
        );
    }
    assert_eq!(
        table_exists(&tx, "no_such_table"),
        0,
        "对照臂：未建过的表名不应查得到"
    );

    tx.commit().unwrap();
}

/// §248 的 `overall_score` 禁令在库侧的落点：`model_profile` 没有这一列。
///
/// 断言的是**具体** `Err`（哪个变体、信息里点了哪个列名），不是「返回了 Err」——
/// 表不存在、SQL 语法错都会给 `Err`，那两种红的理由与本条要守的不是一回事。
/// 红的条件：谁给 `model_profile` 加上 `overall_score` 这一列即红。
#[test]
fn the_model_profile_has_no_total_score_column() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    let err = tx
        .query("SELECT overall_score FROM model_profile", &[])
        .unwrap_err();
    match &err {
        PersistError::Database(m) => {
            assert!(
                m.contains("no such column") && m.contains("overall_score"),
                "应为「没有 overall_score 这一列」，实际 {m}"
            );
        }
        other => panic!("应为 PersistError::Database 变体，实际 {other:?}"),
    }

    tx.commit().unwrap();
}

/// 表名是否落在 `sqlite_master` 的 `table` 类型里（视图/索引不算建出表）。
fn table_exists(tx: &Tx<'_>, name: &str) -> i64 {
    let rows = tx
        .query(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
            &[Value::text(name)],
        )
        .unwrap();
    match &rows[0][0] {
        Value::Int(n) => *n,
        other => panic!("计数应为整数，实际 {other:?}"),
    }
}

/// 经 `PRAGMA table_info` 取列名（声明序，即建表语句里的顺序）。
fn column_names(tx: &Tx<'_>, table: &str) -> Vec<String> {
    tx.query(&format!("PRAGMA table_info({table})"), &[])
        .unwrap()
        .iter()
        .map(|row| text_of(&row[1]))
        .collect()
}
