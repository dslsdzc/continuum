//! 三张表的建出、列清单与主键结构（设计 §3.1、§3.3）。
//!
//! **覆盖到哪就说到哪**：三张表的**列名、声明类型、主键（含 `model_skill_score` 的复合主键
//! 及其列序）**逐列钉住——清单用 `assert_eq!` 全量比对，不是「包含」。**`NOT NULL` 没有照片**，
//! 理由见下节。三张表**都有**列断言（早期版本只钉了 `model_profile` 一张，文件头却写着「列结构」，
//! 措辞大于实际覆盖面）。
//!
//! # 为什么没有 `NOT NULL` 的照片
//!
//! `NOT NULL` 在本仓**没有可观察的落点**。写入一律经类型化接口（`ModelId`、`Ratio`、
//! `LifecycleState::as_str` 等），Rust 侧递不出 `NULL`，故行为层写不出「写 `NULL` 被拒」的用例；
//! 而在结构层钉 `PRAGMA table_info` 的 `notnull`，会顺带把 `id TEXT PRIMARY KEY` 这类
//! **设计本就没写 `NOT NULL`** 的列钉成 `notnull = 0`——那正是 SQLite「非 INTEGER 主键容许
//! `NULL`」的历史遗留，钉住它等于把遗留读成规格。故此处不钉，并明写在此，免得被读成「已覆盖」。
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

fn int_of(v: &Value) -> i64 {
    match v {
        Value::Int(n) => *n,
        other => panic!("应为整数，实际 {other:?}"),
    }
}

/// 一列的结构：`(列名, 声明类型, 主键序)`。
///
/// 主键序取自 `PRAGMA table_info` 的 `pk` 列：`0` = 不在主键里，`1`、`2`、`3` = 主键里的位置
/// （即 `model_skill_score` 的复合主键**连列序**一起钉住，设计 §3.1 第 2 条）。
type Col = (String, String, i64);

/// `model_registry` 的列清单，**逐列**钉住（设计 §3.1 的建表 SQL 是权威取值）。
///
/// 两列而不是画像那样的长清单：§21 的登记与 §247 的画像**是两个对象**，故两张表——
/// 一张表会让「一个模型被登记了」与「一个模型有画像了」不可分辨（设计 §3.1 开头）。
#[test]
fn the_model_registry_columns_are_exactly_the_two_columns() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    assert_eq!(
        columns(&tx, "model_registry"),
        vec![
            ("id".to_string(), "TEXT".to_string(), 1),
            ("lifecycle_state".to_string(), "TEXT".to_string(), 0),
        ],
        "model_registry 的列清单应与设计 §3.1 逐列相同（加一列、改名、换序、改类型都在此变红）"
    );

    tx.commit().unwrap();
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

    assert_eq!(
        columns(&tx, "model_profile"),
        vec![
            ("id".to_string(), "TEXT".to_string(), 1),
            ("version".to_string(), "TEXT".to_string(), 0),
            ("provider".to_string(), "TEXT".to_string(), 0),
            ("model_revision".to_string(), "TEXT".to_string(), 0),
            ("modalities".to_string(), "TEXT".to_string(), 0),
            ("tools".to_string(), "TEXT".to_string(), 0),
            ("failure_modes".to_string(), "TEXT".to_string(), 0),
            ("cost_profile".to_string(), "TEXT".to_string(), 0),
            ("latency_profile".to_string(), "TEXT".to_string(), 0),
            ("evidence_count".to_string(), "INTEGER".to_string(), 0),
            ("confidence".to_string(), "TEXT".to_string(), 0),
        ],
        "model_profile 的列清单应与设计 §3.1 逐列相同（加一列、改名、换序、改类型都在此变红）"
    );

    tx.commit().unwrap();
}

/// `model_skill_score` 的列清单与**复合主键**，逐列钉住（设计 §3.1 的建表 SQL 是权威取值）。
///
/// 主键序 `1/2/3` 落在 `model_id`、`dimension`、`score_version` 上，就是设计 §3.1 第 2 条
/// 「同一维度的同一版本只有一次观测」的**结构性落点**：三列缺一列、多一列或**换序**
/// （`1/2/3` 对不上）都在此变红。这条保证的行为侧照片（裸 `INSERT` 第二次被主键拒）
/// 属写入函数，不在本 task。
#[test]
fn the_model_skill_score_columns_are_exactly_the_eight_columns() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    assert_eq!(
        columns(&tx, "model_skill_score"),
        vec![
            ("model_id".to_string(), "TEXT".to_string(), 1),
            ("dimension".to_string(), "TEXT".to_string(), 2),
            ("score_version".to_string(), "INTEGER".to_string(), 3),
            ("score".to_string(), "TEXT".to_string(), 0),
            ("confidence".to_string(), "TEXT".to_string(), 0),
            ("sample_count".to_string(), "INTEGER".to_string(), 0),
            ("time_range_start".to_string(), "INTEGER".to_string(), 0),
            ("time_range_end".to_string(), "INTEGER".to_string(), 0),
        ],
        "model_skill_score 的列清单与复合主键列序应与设计 §3.1 逐列相同"
    );

    tx.commit().unwrap();
}

/// 三张表各查得到。
///
/// 对照臂：一个没建过的表名**查不到**。它守的是**查询条件对名字有区分力**——若把名字的比较
/// 写松（比如漏掉 `name = ?1`、或整条 `WHERE` 只留 `type = 'table'`），`no_such_table` 也会
/// 查出数来（`sqlite_master` 里已建出的表全被数进去）。
///
/// **这条臂不守 `type = 'table'` 本身**：漏掉它时 `no_such_table` 照样是 0，本臂无从发现
/// （要守它得另找一个与已建表同名的视图或索引，本套件没有）。
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
    int_of(&rows[0][0])
}

/// 经 `PRAGMA table_info` 取列结构（声明序，即建表语句里的顺序）。
fn columns(tx: &Tx<'_>, table: &str) -> Vec<Col> {
    tx.query(&format!("PRAGMA table_info({table})"), &[])
        .unwrap()
        .iter()
        .map(|row| (text_of(&row[1]), text_of(&row[2]), int_of(&row[5])))
        .collect()
}
