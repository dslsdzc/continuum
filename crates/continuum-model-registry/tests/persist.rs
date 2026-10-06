//! 三张表的建出、列清单、主键结构，以及 §21 §4.1 的登记项与生命周期读写
//! （设计 §3.1、§3.2、§3.3、§4.1）。
//!
//! **覆盖到哪就说到哪**：三张表的**列名、声明类型、主键（含 `model_skill_score` 的复合主键
//! 及其列序）**逐列钉住——清单用 `assert_eq!` 全量比对，不是「包含」。**`NOT NULL` 没有照片**，
//! 理由见下节。三张表**都有**列断言（早期版本只钉了 `model_profile` 一张，文件头却写着「列结构」，
//! 措辞大于实际覆盖面）。
//!
//! Task 6 起本文件另钉 `model_registry` 的**行级读写**：`register_model` / `load_lifecycle` /
//! `transition_in_tx`（设计 §3.2）；Task 7 起另钉 `model_profile` 的**行级读写**：
//! `save_profile` / `load_profile`（设计 §4.3）；Task 8 起另钉 `model_skill_score` 的**行级读写**：
//! `save_skill_observation` / `load_skill_vector` / `load_skill_series`（设计 §3.1、§24、§248）；
//! Task 14 起另钉 `model_registry` 的**全量读出**：`list_registered`（「列出已登记的模型」
//! 这个问题的入口，子项目 G 的候选集从它出发）。
//!
//! # 编码的格式在这三条链上被钉死（不是靠单条用例）
//!
//! 列里的字面量 ↔ `as_str`：`tests/lifecycle.rs` 的 `ALL_NAMES`（手写字面量）逐项比对；
//! `as_str` ↔ `parse`：同文件的往返用例与本文件的 `every_lifecycle_state_round_trips`；
//! `as_str` ↔ **列内容**：本文件的 `the_lifecycle_state_column_is_lowercase_with_underscores`
//! 走被测写入路径后读**原始列值**，与**手写字面量**逐字比对。三段接起来才是
//! 「列里存的是小写字面量」这条断言；任何一段单看都只是编码器与解码器互相对账，两者同错时不红。
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

use continuum_capability::{Cost, Latency};
use continuum_core::model::ModelId;
use continuum_core::tool::ToolId;
use continuum_model_registry::{
    LifecycleError, LifecycleState, ModelProfile, Ratio, SkillDimension, SkillObservation,
    SkillScore, list_registered, load_lifecycle, load_profile, load_skill_series,
    load_skill_vector, p3d_model_migrations, register_model, save_profile, save_skill_observation,
    transition_in_tx,
};
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

// ===== 登记项与生命周期的行级读写（设计 §3.2、§21、§4.1） =====

/// §249 的十个状态**逐项列出**（不抽代表），落库编码的**手写字面量**与之一一对应。
///
/// 与 `tests/lifecycle.rs` 的 `ALL_STATES` / `ALL_NAMES` 是**同一份手写表的两处副本**：
/// 集成测试之间共享不了 `const`，加第十一态时两处都要手工同步——这个点不设防。
/// 本文件需要它自己这一份，是因为格式断言要拿**手写字面量**与**列内容**比，
/// 而 `as_str()` 返回的串不能当这个字面量用（那是被测函数返回的，钉的是路径不是格式）。
const ALL_STATES: [(LifecycleState, &str); 10] = [
    (LifecycleState::Discovered, "discovered"),
    (LifecycleState::Unprofiled, "unprofiled"),
    (LifecycleState::Researched, "researched"),
    (LifecycleState::Probed, "probed"),
    (LifecycleState::Verified, "verified"),
    (LifecycleState::Active, "active"),
    (LifecycleState::Stale, "stale"),
    (LifecycleState::Degraded, "degraded"),
    (LifecycleState::Quarantined, "quarantined"),
    (LifecycleState::Disabled, "disabled"),
];

fn id(s: &str) -> ModelId {
    ModelId::new(s)
}

/// §21「发现即登记」：`register_model` 之后该模型的生命周期是 `Discovered`。
///
/// 对照臂：**未登记**的 id 读得 `None`——只断 `Some(Discovered)` 的话，
/// 「登记之后谁都有生命周期」这类坏法没有照片（它的专案是
/// `an_unknown_model_has_no_lifecycle`，此处留一条只为让本用例自足）。
#[test]
fn registering_a_model_leaves_it_discovered() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    register_model(&tx, &id("model-a")).unwrap();
    assert_eq!(
        load_lifecycle(&tx, &id("model-a")).unwrap(),
        Some(LifecycleState::Discovered),
        "§21 的「发现即登记」：新登记项的初值是 `discovered`"
    );
    assert_eq!(
        load_lifecycle(&tx, &id("model-b")).unwrap(),
        None,
        "对照臂：登记的是 `model-a`，`model-b` 仍无生命周期"
    );

    tx.commit().unwrap();
}

/// 同 id 再登记被拒，**且原行一点没动**（裸 `INSERT`，不是 `INSERT OR REPLACE`）。
///
/// **原行先被迁到 `Unprofiled` 再重登记**：`register_model` 写的初值是 `Discovered`，
/// 若原行停在 `Discovered`，「再登记被拒后仍是 `Discovered`」在 `OR REPLACE` 下**照样成立**
/// ——那个形状的断言区分不出裸 `INSERT` 与 `OR REPLACE`（等价变异体的判据：这两版在哪个
/// 入参上会给出不同结果）。把原行挪到别的状态，「未被改写」才成为可观察的事实。
///
/// 红的条件两条，各在一侧：`INSERT` 被改成 `OR REPLACE`（或先 `DELETE` 再写）→ 本用例红；
/// 写入口整个坏掉 → 末尾那条对照臂（换个 id 仍能登记）红。
#[test]
fn registering_the_same_id_twice_is_rejected_and_the_row_is_unchanged() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    register_model(&tx, &id("model-a")).unwrap();
    assert_eq!(
        transition_in_tx(&tx, &id("model-a"), LifecycleState::Unprofiled).unwrap(),
        LifecycleState::Discovered,
        "先把原行挪离初值，下面「未被改写」才有区分力"
    );

    let err = register_model(&tx, &id("model-a")).unwrap_err();
    // 断言是哪一种 `Err`：`Tx::execute` 把 rusqlite 的错误码抹成字符串，
    // 故只能在 `Database` 变体内按消息断言冲突来自主键（同 P3A 的 `saving_the_same_id_twice_is_rejected`）。
    match &err {
        PersistError::Database(m) => assert!(
            m.contains("UNIQUE constraint failed: model_registry.id"),
            "应为 model_registry.id 上的主键冲突，实际 {m}"
        ),
        other => panic!("应为 PersistError::Database 变体，实际 {other:?}"),
    }

    assert_eq!(
        load_lifecycle(&tx, &id("model-a")).unwrap(),
        Some(LifecycleState::Unprofiled),
        "被拒的登记不得改动原行"
    );
    assert_eq!(
        raw_state(&tx, "model-a"),
        "unprofiled",
        "原行的列值也须逐字原样"
    );
    assert_eq!(count_registry(&tx), 1, "被拒的登记不得新增行");

    // 对照臂：挡住的是同 id，不是所有登记。
    register_model(&tx, &id("model-c")).unwrap();
    assert_eq!(count_registry(&tx), 2);

    tx.commit().unwrap();
}

/// 十态**逐项**往返：把每一态经 `as_str` 写进库，`load_lifecycle` 读回得**同一个变体**。
///
/// 逐项而非抽代表：`parse` 的十条臂能各自漂移，抽一个代表时漏掉其中一条不会红
/// （`as_str` 侧有穷尽 match 兜住，`parse` 侧没有——见 `LifecycleState::parse` 的注释）。
///
/// **本用例钉的是路径**（`parse ∘ as_str` 经库往返恒等），**不钉格式**：写进去的串是
/// `as_str()` 返回的、不是手写字面量，故两边同错（例如都改成大写）时它不红——
/// 格式由 `the_lifecycle_state_column_is_lowercase_with_underscores` 用手写字面量钉。
#[test]
fn every_lifecycle_state_round_trips() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    for (i, (state, _literal)) in ALL_STATES.iter().enumerate() {
        let model = format!("model-{i}");
        // 裸 SQL 直写：本用例要的是「列里放着这一态的编码」这个初始条件，
        // 不是任何一条迁移边（`verified` / `stale` 等没有从 `discovered` 直通的边）。
        tx.execute(
            "INSERT INTO model_registry (id, lifecycle_state) VALUES (?1, ?2)",
            &[Value::text(model.clone()), Value::text(state.as_str())],
        )
        .unwrap();
        assert_eq!(
            load_lifecycle(&tx, &id(&model)).unwrap(),
            Some(*state),
            "{state:?} 经库往返应得同一个变体"
        );
    }

    tx.commit().unwrap();
}

/// 列里存的是**手写字面量**（小写、`_` 连接），**不是 `Debug` 表示**。
///
/// 走**被测写入路径**（`register_model` 的初值，以及 `transition_in_tx` 把该模型
/// 沿 §4.1 的表推过全部十态），每次读**原始列值**与手写字面量逐字比对，故十态一个不漏：
/// `discovered`（登记）→ `unprofiled` / `researched` / `probed` / `verified` / `active`
/// → `stale` →（§82 的重新 profiling 回到 `researched`）→ `degraded` → `quarantined` → `disabled`。
///
/// 反向也在本用例里：把手写字面量**直接写进库**，`load_lifecycle` 应解出对应的那一枚
/// ——这一侧不依赖 `as_str`，故「写侧与读侧同时漂移」不会让本用例静默。
///
/// 红的条件：任一处落库改成 `format!("{to:?}")`（`Discovered` / `Quarantined`）、大写、
/// 或多词加下划线即红（写侧读原始列值的十条断言）；`parse` 只认某一枚字面量即反向那侧红。
#[test]
fn the_lifecycle_state_column_is_lowercase_with_underscores() {
    use LifecycleState::*;

    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    register_model(&tx, &id("model-a")).unwrap();
    assert_eq!(
        raw_state(&tx, "model-a"),
        "discovered",
        "登记项的初值应是手写字面量 `discovered`，不是 `Debug` 的 `Discovered`"
    );

    // 沿 §4.1 的合法边把十态走一遍（`stale` 之后回到 `researched`，故有几态走两次）。
    for (to, literal) in [
        (Unprofiled, "unprofiled"),
        (Researched, "researched"),
        (Probed, "probed"),
        (Verified, "verified"),
        (Active, "active"),
        (Stale, "stale"),
        (Researched, "researched"),
        (Probed, "probed"),
        (Verified, "verified"),
        (Active, "active"),
        (Degraded, "degraded"),
        (Quarantined, "quarantined"),
        (Disabled, "disabled"),
    ] {
        transition_in_tx(&tx, &id("model-a"), to).unwrap();
        assert_eq!(
            raw_state(&tx, "model-a"),
            literal,
            "{to:?} 的落库编码应是手写字面量 {literal:?}"
        );
    }

    // 反向：手写字面量直接写进库 → `load_lifecycle` 解出对应变体。
    for (state, literal) in ALL_STATES {
        let model = format!("raw-{literal}");
        tx.execute(
            "INSERT INTO model_registry (id, lifecycle_state) VALUES (?1, ?2)",
            &[Value::text(model.clone()), Value::text(literal)],
        )
        .unwrap();
        assert_eq!(
            load_lifecycle(&tx, &id(&model)).unwrap(),
            Some(state),
            "手写字面量 {literal:?} 应解出 {state:?}"
        );
    }

    tx.commit().unwrap();
}

/// 列里的表外取值报**具体** `Err`，**不取默认值**。
///
/// 把串猜成另一枚状态（例如 `parse` 的 `_` 臂返回 `Discovered`）正是设计要拦的
/// （`LifecycleState::parse` 的注释）：本用例断言 `Err(PersistError::Database)` 且
/// **信息里点了那个表外串**——只写 `is_err()` 的话，「报了个错」与「说清了是哪个串」
/// 分不出来，而后者才是排查要用的事实。
///
/// 两个表外串各一条：未知词 `ascended`，以及大写 `ACTIVE`（后者是 `Debug` 表示那类
/// 写法的典型形状——它若真被写进库，本用例是发现它的那条链的终点）。
#[test]
fn an_unknown_lifecycle_state_in_the_column_is_rejected() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    for (i, raw) in ["ascended", "ACTIVE"].into_iter().enumerate() {
        let model = format!("model-{i}");
        tx.execute(
            "INSERT INTO model_registry (id, lifecycle_state) VALUES (?1, ?2)",
            &[Value::text(model.clone()), Value::text(raw)],
        )
        .unwrap();

        let err = load_lifecycle(&tx, &id(&model)).unwrap_err();
        match &err {
            PersistError::Database(m) => assert!(
                m.contains("未知 LifecycleState") && m.contains(raw),
                "应为「未知 LifecycleState: {raw}」，实际 {m}"
            ),
            other => panic!("应为 PersistError::Database 变体，实际 {other:?}"),
        }
    }

    tx.commit().unwrap();
}

/// 列里是**非文本**值时的照片：`load_lifecycle` 报 `ColumnType { index: 0, actual: "blob" }`。
///
/// 为什么要这一条：`lifecycle_state` 是 `TEXT NOT NULL`，`ColumnType` 这条臂在
/// 前三种取值形状下**都没有产生方**——那会是一条死臂。四种形状逐项枚举如下，
/// 故「非文本值只能是 BLOB」不是印象而是穷举后的结论：
///
/// 1. **整数 / 实数**：被 TEXT 列的亲和性**转换**成文本（`Value::Int(3)` 读回来是
///    `"3"`），走的是表外取值那条路（`PersistError::Database`），到不了 `ColumnType`；
/// 2. **`NULL`**：被 `NOT NULL` 挡在写入侧，列里不可能有它；
/// 3. **文本**：本臂的正例，不解释；
/// 4. **BLOB**：**不受亲和性转换**，是本库里唯一能喂出非文本值的形状。
///
/// 本条给第 4 种一张照片（本项目对死臂的处置是删或写明来历）。
#[test]
fn a_non_text_lifecycle_state_column_is_rejected() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    tx.execute(
        "INSERT INTO model_registry (id, lifecycle_state) VALUES (?1, ?2)",
        &[Value::text("model-a"), Value::Blob(b"discovered".to_vec())],
    )
    .unwrap();

    assert_eq!(
        load_lifecycle(&tx, &id("model-a")),
        Err(PersistError::ColumnType {
            index: 0,
            actual: "blob"
        }),
        "非文本的列值应报 ColumnType，不是当成表外状态串"
    );

    tx.commit().unwrap();
}

/// 库里没有的 id → `Ok(None)`（**不是** `Err`）。
///
/// `UnknownModel` 用在**转移**上（没有登记项可改），不在读上——读一个不存在的模型是
/// 「没有」，不是「出错」。
///
/// 两侧对钉：未登记的 `None` ＋ 已登记的 `Some(Discovered)`——只写 `None` 那侧时，
/// 「读什么都返回 `None`」这种坏法不会红。
#[test]
fn an_unknown_model_has_no_lifecycle() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    assert_eq!(
        load_lifecycle(&tx, &id("model-absent")).unwrap(),
        None,
        "未登记的 id 读得 `None`，不是 `Err`"
    );
    register_model(&tx, &id("model-absent")).unwrap();
    assert_eq!(
        load_lifecycle(&tx, &id("model-absent")).unwrap(),
        Some(LifecycleState::Discovered),
        "对照臂：登记之后同一 id 读得到状态"
    );

    tx.commit().unwrap();
}

/// §21 的登记表**全量**读出：**十态各一行、十行都在**，按 id 升序，每行的 `LifecycleState`
/// 是登记后的那个值（不是初值 `discovered`）。
///
/// # 为什么是**十行（十态各一行）**而不是两三行
///
/// 被钉的那句话是「**不过滤状态**」，而它是一句**枚举断言**（`list_registered` 的文档：
/// 「十态一个都不滤，四个不可路由的异常态与其余六态一视同仁」）。本仓纪律是
/// **枚举断言须逐项有照片**——**「一臂代表四类」不是覆盖**：夹具里只放 `unprofiled`
/// 与 `researched` 时，一个在 SQL 里加 `WHERE lifecycle_state NOT IN ('stale','quarantined','disabled')`
/// 的变异体**编得过、且什么都不红**（Task 14 评审实测的正是这一格）。
/// 故这里**十态逐项各登记一行**，被滤掉的任何一态都会落在期望值里而缺席于返回值。
///
/// # 四处都做了「可分辨」的准备，否则四种坏法不会红
///
/// - **漏行**：十行都登记，故「少列一行」与「列全」结果不同；
/// - **漏状态 / 状态取错来源**：十行的状态**两两不同**，且除 `model-01` 外**都不等于初值**
///   （`model-01` 就是 `discovered`——它代表「登记初值」这一态，故它留在初值是**有意的**：
///   「状态恒取初值」这条坏法由**另外九行**变红）；
/// - **按状态过滤**：四个不可路由态（`unprofiled` / `stale` / `quarantined` / `disabled`）
///   各有自己的一行——滤掉其中**任意一态**（或四态全滤）都在末尾那条断言上红；
/// - **夹带未登记的 id**：库里另有一个**只用于对照**的未登记 id，清单里不得出现它。
///
/// # 顺序那一半靠「登记序与 id 序相反」才可分辨
///
/// 下面按 `model-10 → model-01` 的**逆序**登记，而 id 升序以 `model-01` 打头：若把
/// `ORDER BY id ASC` 删掉（SQLite 按 rowid／插入序给行），返回的次序与期望值整段相反。
/// 若按同一序登记，这两版**等价**，那条断言就没有区分力。
///
/// # 空表也要覆盖
///
/// 首条断言是空库得**空清单**：少了它，「没有登记项时返回一个空元素／`None` 之类」不会被区分。
#[test]
fn list_registered_lists_every_registered_model() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    assert_eq!(
        list_registered(&tx).unwrap(),
        vec![],
        "还没有登记项时应给出空清单，不是 `Err`、也不是一项空的"
    );

    // 十态各一行的**走链路径**：每条都是 §4.1 迁移表里的合法前缀（空路径 = 登记初值
    // `discovered`）。`model-10` 先登记（登记序与 id 序**相反**，见文档头）。
    let ladder: [(&str, &[LifecycleState]); 10] = [
        ("model-01", &[]),
        ("model-02", &[LifecycleState::Unprofiled]),
        ("model-03", &[LifecycleState::Unprofiled, LifecycleState::Researched]),
        (
            "model-04",
            &[
                LifecycleState::Unprofiled,
                LifecycleState::Researched,
                LifecycleState::Probed,
            ],
        ),
        (
            "model-05",
            &[
                LifecycleState::Unprofiled,
                LifecycleState::Researched,
                LifecycleState::Probed,
                LifecycleState::Verified,
            ],
        ),
        (
            "model-06",
            &[
                LifecycleState::Unprofiled,
                LifecycleState::Researched,
                LifecycleState::Probed,
                LifecycleState::Verified,
                LifecycleState::Active,
            ],
        ),
        (
            "model-07",
            &[
                LifecycleState::Unprofiled,
                LifecycleState::Researched,
                LifecycleState::Probed,
                LifecycleState::Verified,
                LifecycleState::Active,
                LifecycleState::Stale,
            ],
        ),
        (
            "model-08",
            &[
                LifecycleState::Unprofiled,
                LifecycleState::Researched,
                LifecycleState::Probed,
                LifecycleState::Verified,
                LifecycleState::Active,
                LifecycleState::Degraded,
            ],
        ),
        ("model-09", &[LifecycleState::Quarantined]),
        ("model-10", &[LifecycleState::Disabled]),
    ];
    for (model, path) in ladder.iter().rev() {
        register_model(&tx, &id(model)).unwrap();
        for to in path.iter() {
            transition_in_tx(&tx, &id(model), *to).unwrap();
        }
    }

    // 对照臂：表里只有上面十行，`model-absent` 未登记——清单里不得出现它。
    assert_eq!(
        load_lifecycle(&tx, &id("model-absent")).unwrap(),
        None,
        "对照臂：`model-absent` 未登记"
    );

    // 期望值是**手写的十项**（不从 `ladder` 推）：走链的终点与这里写的是两份，
    // 任一处写错都对不上——若从 `ladder` 推，期望值会跟着错的路径一起错。
    assert_eq!(
        list_registered(&tx).unwrap(),
        vec![
            (id("model-01"), LifecycleState::Discovered),
            (id("model-02"), LifecycleState::Unprofiled),
            (id("model-03"), LifecycleState::Researched),
            (id("model-04"), LifecycleState::Probed),
            (id("model-05"), LifecycleState::Verified),
            (id("model-06"), LifecycleState::Active),
            (id("model-07"), LifecycleState::Stale),
            (id("model-08"), LifecycleState::Degraded),
            (id("model-09"), LifecycleState::Quarantined),
            (id("model-10"), LifecycleState::Disabled),
        ],
        "十行都在、按 id 升序，且每行的状态是登记后的那个值（含四个不可路由态，一个都不许被滤掉）"
    );
    assert_eq!(
        count_registry(&tx),
        10,
        "对照臂：清单长度应与表里的行数一致"
    );

    tx.commit().unwrap();
}

/// `transition_in_tx` 落库的是 `to`，**返回的是迁移前的 `from`**（设计 §3.2 的定稿）。
///
/// 两侧都断言：只看返回值或只看库里的值，都会漏掉「返回值写成 `to`」这类错——
/// 若 `Discovered → Unprofiled` 的返回值是 `Unprofiled`，返回值那侧红；若没写回库，库那侧红。
///
/// 走**两步**（再迁一次）使 `from` 不是恒定初值：`from` 若恒为 `Discovered` 这个初值，
/// 「返回的是 `from`」与「返回的是初值」就分不出来。
#[test]
fn a_transition_writes_the_new_state_and_returns_the_previous() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    register_model(&tx, &id("model-a")).unwrap();

    assert_eq!(
        transition_in_tx(&tx, &id("model-a"), LifecycleState::Unprofiled).unwrap(),
        LifecycleState::Discovered,
        "返回值是迁移**前**的旧态"
    );
    assert_eq!(
        load_lifecycle(&tx, &id("model-a")).unwrap(),
        Some(LifecycleState::Unprofiled),
        "库里落的是迁移**后**的新态"
    );

    assert_eq!(
        transition_in_tx(&tx, &id("model-a"), LifecycleState::Researched).unwrap(),
        LifecycleState::Unprofiled,
        "第二步的旧态是上一步写进去的那个，不是初值"
    );
    assert_eq!(
        load_lifecycle(&tx, &id("model-a")).unwrap(),
        Some(LifecycleState::Researched)
    );

    tx.commit().unwrap();
}

/// 对未登记的 id 做迁移 → `Err(UnknownModel { id })`，**且 `id` 正是给的那一个**。
///
/// **不静默创建**（设计 §3.2）：登记是 `register_model` 的活，故断言库里仍没有这一行——
/// 只断言错误种类的话，「先偷偷 `INSERT` 一行再报错」这种半写不会有照片。
///
/// 用两个不同的 id 各来一次：只用一个的话，把 `id` 字段写成常量（或写成别的 id）
/// 也有机会蒙对。
#[test]
fn a_transition_on_an_unknown_model_is_rejected_with_the_id() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    for model in ["model-missing-1", "model-missing-2"] {
        assert_eq!(
            transition_in_tx(&tx, &id(model), LifecycleState::Unprofiled),
            Err(LifecycleError::UnknownModel { id: id(model) }),
            "未登记的 id 应报 `UnknownModel` 且带着它自己"
        );
        assert_eq!(
            load_lifecycle(&tx, &id(model)).unwrap(),
            None,
            "被拒的迁移不得静默创建登记项"
        );
    }
    assert_eq!(count_registry(&tx), 0, "被拒的迁移不得新增行");

    tx.commit().unwrap();
}

/// 非法对：`Err(Illegal { from, to })`，**且库里的状态一点没动**。
///
/// 失败路径不只断言「是哪一种 `Err`」，还要断言**没有半写的副作用**：若实现先把 `to`
/// 写进库、再去查迁移表，返回值仍然是 `Illegal`——只断言 `Err` 的用例完全看不出这一笔，
/// 故这里读回库里的值。
///
/// **三个状态互不相同**（`from = Unprofiled`、`to = Verified`、初值 `Discovered`）：
/// 「没变」若等于「还是初值」，在「先写 `to` 后报错」下也可能蒙对（`to` 恰好是初值时）。
///
/// 末尾还有一条对照臂：被拒之后这个登记项**仍然可用**，合法迁移读回的旧态正是 `Unprofiled`
/// ——它同时钉住「失败那一步没有把行弄坏，也没有把旧态记错」。
#[test]
fn an_illegal_transition_leaves_the_row_unchanged() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    register_model(&tx, &id("model-a")).unwrap();
    transition_in_tx(&tx, &id("model-a"), LifecycleState::Unprofiled).unwrap();

    // `unprofiled → verified` 跨过整条画像流水线，不在 §4.1 的表里。
    assert_eq!(
        transition_in_tx(&tx, &id("model-a"), LifecycleState::Verified),
        Err(LifecycleError::Illegal {
            from: LifecycleState::Unprofiled,
            to: LifecycleState::Verified
        }),
        "跨过画像流水线的对不在表里"
    );

    assert_eq!(
        load_lifecycle(&tx, &id("model-a")).unwrap(),
        Some(LifecycleState::Unprofiled),
        "非法迁移不得改动库里的状态（`to` 不得先被写进去）"
    );
    assert_eq!(
        raw_state(&tx, "model-a"),
        "unprofiled",
        "列值也须逐字原样"
    );

    // 对照臂：被拒之后该登记项仍可用，且旧态仍是 `Unprofiled`。
    assert_eq!(
        transition_in_tx(&tx, &id("model-a"), LifecycleState::Researched).unwrap(),
        LifecycleState::Unprofiled,
        "失败那一步没有把行弄坏"
    );

    tx.commit().unwrap();
}

/// 落库失败经 `?` 升格成 [`LifecycleError::Persist`]，且**内层变体原样带出**。
///
/// # 为什么要自己开一个库
///
/// 正常读写不产生 `Persist`，而计划「三条纪律」第 3 条要求 `LifecycleError` **逐变体**至少
/// 一条用例——本 task 正是这枚变体的产生方，故这条照片的本分在这里。
/// 造法借自 `crates/continuum-workspace/src/gate.rs` 的
/// `a_failed_audit_write_reports_persist_and_leaves_the_change_in_the_base`：
/// **建一个不开迁移的库**，`model_registry` 表因此不存在，读写必然失败
/// （真实数据库错误，不是伪造一个失败的 `Tx`）。本文件的夹具 `db()` 必然 `migrate()`，
/// 故本条自建库而**不** `migrate()`。
///
/// ## 断言到内层变体
///
/// 不是「返回了某个 `Err`」：外层须是 `Persist`，内层须是 `PersistError::Database`
/// 且信息里点了缺失的表名。红的条件：把 `#[from]` 的转出吞掉（例如
/// `load_lifecycle(tx, id).unwrap_or(None)`，于是外层变成 `UnknownModel`）、
/// 或把内层换成另一枚，都在此变红。
#[test]
fn a_persist_failure_keeps_the_inner_persist_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("unmigrated.db");
    // 故意不 migrate()：三张表一张都不存在。
    let db = Db::open_with(&path, builtin_migrations()).unwrap();
    let tx = db.begin().unwrap();

    match transition_in_tx(&tx, &id("model-a"), LifecycleState::Unprofiled) {
        Err(LifecycleError::Persist(PersistError::Database(m))) => assert!(
            m.contains("no such table: model_registry"),
            "内层应是「目标表不存在」的 Database 错误，实际 {m}"
        ),
        Err(LifecycleError::Persist(other)) => {
            panic!("内层应为 PersistError::Database，实际 {other:?}")
        }
        other => panic!("落库失败应升格成 LifecycleError::Persist，实际 {other:?}"),
    }

    // 读写面的失败不止迁移一处：`register_model` 也是同一枚 `Persist`。
    match register_model(&tx, &id("model-a")) {
        Err(PersistError::Database(m)) => assert!(
            m.contains("no such table: model_registry"),
            "内层应是「目标表不存在」的 Database 错误，实际 {m}"
        ),
        other => panic!("登记失败应报 PersistError::Database，实际 {other:?}"),
    }

    // 不 commit：库里没有迁移，也没有要提交的写入；`Tx` 丢弃时自行 ROLLBACK。
    drop(tx);
}

// ===== 画像的行级读写（设计 §2.1、§3.1、§4.3） =====

/// # 本节夹具的形态：裸 SQL 播一行 → `load_profile` 读回
///
/// `ModelProfile` 在 crate 外**没有构造入口**（`try_new` 是 `pub(crate)`，设计 §2.1），
/// 故集成测试**拿不到**一份可以直接传给 `save_profile` 的画像。唯一通道是 `load_profile`，
/// 而它要先有行。于是本节每个用例都走同一条路：裸 SQL 播一行（**十一列的字面量全部手写**）
/// → `load_profile` 读回 → 拿它当被测输入。
///
/// 这不是测试技巧，是设计 §2.1 那条保证的直接后果（Task 11 的 router 夹具同形）。
/// 副作用是好的：`a_profile_round_trips_field_by_field` 因此能拿**手写字面量**与
/// **读回的字段值**逐项比，而不是拿被测函数的一侧与另一侧对账。

/// 播进 `model_profile` 的三个列表列的字面量（**手写** JSON 文本，非 `to_string` 的产物）。
const MODALITIES: &str = r#"["text","image"]"#;
const TOOLS: &str = r#"["tool-epsilon","tool-zeta"]"#;
const FAILURE_MODES: &str = r#"["loses constraints in very long tasks"]"#;

/// 往 `model_profile` 播一行。**十一列的字面量全部手写**，`version` / `provider` /
/// `model_revision` 三个同类型的文本列**两两互不相同**——理由就是各用例红的条件：
/// 全取空串时，「某个字段漏存」与「两个同类型字段写串」两类缺陷都会静默。
///
/// 三个列表列与两个存在性列由调用方给，故「空数组 / 非空数组」与「`NULL` / 非 `NULL`」
/// 两侧走同一入口。`cost_profile` / `latency_profile` 的 `Option` 是**库列**的 `NULL` 语义
/// （`None` = 尚未登记画像），与空串不是一回事。
fn insert_profile_row(
    tx: &Tx<'_>,
    model: &str,
    modalities: &str,
    tools: &str,
    failure_modes: &str,
    cost_profile: Option<&str>,
    latency_profile: Option<&str>,
) {
    tx.execute(
        "INSERT INTO model_profile
           (id, version, provider, model_revision, modalities, tools, failure_modes,
            cost_profile, latency_profile, evidence_count, confidence)
         VALUES (?1, 'version-beta', 'provider-gamma', 'revision-delta', ?2, ?3, ?4, ?5, ?6, 42, '0.94')",
        &[
            Value::text(model),
            Value::text(modalities),
            Value::text(tools),
            Value::text(failure_modes),
            match cost_profile {
                Some(s) => Value::text(s),
                None => Value::Null,
            },
            match latency_profile {
                Some(s) => Value::text(s),
                None => Value::Null,
            },
        ],
    )
    .unwrap();
}

/// 把一个刚登记的模型沿 §4.1 的合法边推到 `verified`：画像流水线走完了，`save_profile` 的闸门放行。
fn advance_to_verified(tx: &Tx<'_>, model: &str) {
    use LifecycleState::*;
    for to in [Unprofiled, Researched, Probed, Verified] {
        transition_in_tx(tx, &id(model), to).unwrap();
    }
}

/// 本节最常用的那一行：三个列表列非空，`cost` 已登记（非 `NULL`）、`latency` 未登记（`NULL`）。
///
/// 两个存在性列取**不同**的一侧，是为了让「`cost_profile` 与 `latency_profile` 两列写串」
/// 在本文件里可观察——两个列的字面量都是空串（`Cost::as_str` / `Latency::as_str` 的取值域
/// 为空），列内容比不出区别，能比出来的只有存在性。四种组合各有照片的见
/// `the_cost_and_latency_columns_distinguish_absent_from_registered`。
fn seed_profile_row(tx: &Tx<'_>, model: &str) {
    insert_profile_row(tx, model, MODALITIES, TOOLS, FAILURE_MODES, Some(""), None);
}

/// 逐字段比对本 crate 落库的那**十一列**，期望值全部由**调用点手写**（非被测函数返回值）。
///
/// 十二个字段里 `skill_vector` 不在这里：它落 `model_skill_score` 表（设计 §3.1），
/// `load_profile` 读回来的画像里它恒为空向量——那一条单独断言，不混进本函数（免得
/// 读者以为本函数覆盖了十二项）。
///
/// 十项各钉一次，理由与 `src/profile.rs` 的 `a_profile_carries_the_twelve_fields_of_247` 同：
/// **漏存**（某列没接上）与**写串**（同类型两列互换）都在此变红。`cost` / `latency` 各一条，
/// 两条都钉的是「存在性」，取值由 `Cost` / `Latency` 的空取值域决定。
fn assert_profile_fields(
    profile: &ModelProfile,
    version: &str,
    provider: &str,
    model_revision: &str,
    modalities: &[&str],
    tools: &[&str],
    failure_modes: &[&str],
    cost: Option<Cost>,
    latency: Option<Latency>,
    evidence_count: u64,
    confidence: f64,
) {
    assert_eq!(profile.version(), version, "§247 第 2 项 version");
    assert_eq!(profile.provider(), provider, "§247 第 3 项 provider");
    assert_eq!(
        profile.model_revision(),
        model_revision,
        "§247 第 4 项 model_revision"
    );
    let expected_modalities: Vec<String> = modalities.iter().map(|s| (*s).to_string()).collect();
    assert_eq!(
        profile.modalities(),
        expected_modalities.as_slice(),
        "§247 第 5 项 modalities[]"
    );
    let expected_tools: Vec<ToolId> = tools.iter().map(|s| ToolId::new(*s)).collect();
    assert_eq!(
        profile.tools(),
        expected_tools.as_slice(),
        "§247 第 6 项 tools[]（元素是 ToolId）"
    );
    let expected_failure_modes: Vec<String> =
        failure_modes.iter().map(|s| (*s).to_string()).collect();
    assert_eq!(
        profile.failure_modes(),
        expected_failure_modes.as_slice(),
        "§247 第 8 项 failure_modes[]"
    );
    assert_eq!(
        profile.cost_profile(),
        cost,
        "§247 第 10 项 cost_profile：`None` 与 `Some` 是两个不同的存在性"
    );
    assert_eq!(
        profile.latency_profile(),
        latency,
        "§247 第 9 项 latency_profile：语义同 cost_profile"
    );
    assert_eq!(
        profile.evidence_count(),
        evidence_count,
        "§247 第 11 项 evidence_count"
    );
    assert_eq!(
        profile.confidence(),
        Ratio::try_new(confidence).expect("期望值应是合法置信度"),
        "§247 第 12 项 confidence"
    );
}

/// `model_profile` 十一列的**原始值**（声明序，**不经任何解码**）。
///
/// 列清单在本函数里**手写一遍**，不从被测实现里取——这样「实现把某列写到了邻列」
/// 才会在原始值上暴露出来；若与实现共用一份列清单，两侧同错时不红。
fn raw_profile(tx: &Tx<'_>, model: &str) -> Vec<Value> {
    tx.query(
        "SELECT id, version, provider, model_revision, modalities, tools, failure_modes,
                cost_profile, latency_profile, evidence_count, confidence
         FROM model_profile WHERE id = ?1",
        &[Value::text(model)],
    )
    .unwrap()
    .into_iter()
    .next()
    .expect("该 id 应有一行")
}

/// `model_profile` 的行数。
fn count_profiles(tx: &Tx<'_>) -> i64 {
    int_of(&tx.query("SELECT COUNT(*) FROM model_profile", &[]).unwrap()[0][0])
}

/// 十一列的字段值**逐字段**往返：先（读侧）拿手写字面量与读回的字段比，
/// 再（写侧）删掉那一行、用被测写入路径写回、读**原始列值**与手写字面量逐列比。
///
/// # 为什么两侧都要
///
/// 只做读侧：`save_profile` 一列都不写、或写到邻列，本用例照样全绿。
/// 只做写侧：`load_profile` 把两列读串，本用例照样全绿（手写值 → 列 → 读回，两处错相互抵消）。
///
/// # 为什么读侧能用「手写字面量」当期望值
///
/// 因为播进去的行是裸 SQL 写的手写字面量，不是 `as_str()` 的返回值（计划「三条纪律」第 2 条：
/// 期望值是手写的才钉格式，是被测函数返回的只钉路径）。
///
/// 红的条件：某列漏写、与邻列写串、或两列的存在性写反，任一处即红。
#[test]
fn a_profile_round_trips_field_by_field() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    register_model(&tx, &id("model-alpha")).unwrap();
    advance_to_verified(&tx, "model-alpha");
    seed_profile_row(&tx, "model-alpha");

    // ===== 读侧 =====
    let loaded = load_profile(&tx, &id("model-alpha"))
        .unwrap()
        .expect("刚播的行应读得到");
    assert_eq!(loaded.id(), &id("model-alpha"), "§247 第 1 项 id");
    assert_profile_fields(
        &loaded,
        "version-beta",
        "provider-gamma",
        "revision-delta",
        &["text", "image"],
        &["tool-epsilon", "tool-zeta"],
        &["loses constraints in very long tasks"],
        Some(Cost),
        None,
        42,
        0.94,
    );
    // 第十二个字段不在这张表里（十一列 ≠ 十二字段），它由 `load_profile` **组合**
    // `load_skill_vector` 填上（Task 8）。本用例播的画像**一条观测都没有**，故读回来的是
    // **空向量**：九维全 `None`（「尚无观测」），不是九维全 0——后者的逐项照片在 Task 8 的
    // `every_dimension_without_an_observation_is_absent_not_zero`。
    for dimension in [
        SkillDimension::Reasoning,
        SkillDimension::Coding,
        SkillDimension::Media,
    ] {
        assert_eq!(
            loaded.skill_vector().get(dimension),
            None,
            "该模型尚无观测，{dimension:?} 应是 `None`（缺席不是 0）"
        );
    }

    // ===== 写侧 =====
    tx.execute(
        "DELETE FROM model_profile WHERE id = ?1",
        &[Value::text("model-alpha")],
    )
    .unwrap();
    assert_eq!(count_profiles(&tx), 0, "删干净，下面那一行才只可能来自被测写入路径");

    save_profile(&tx, &loaded).unwrap();

    assert_eq!(
        raw_profile(&tx, "model-alpha"),
        vec![
            Value::text("model-alpha"),
            Value::text("version-beta"),
            Value::text("provider-gamma"),
            Value::text("revision-delta"),
            Value::text(MODALITIES),
            Value::text(TOOLS),
            Value::text(FAILURE_MODES),
            Value::text(""),
            Value::Null,
            Value::Int(42),
            Value::text("0.94"),
        ],
        "十一列的原始值应与手写字面量逐列相同（写错列、漏写列、`cost`/`latency` 写反都在此变红）"
    );

    let reloaded = load_profile(&tx, &id("model-alpha"))
        .unwrap()
        .expect("写回后应读得到");
    assert_profile_fields(
        &reloaded,
        "version-beta",
        "provider-gamma",
        "revision-delta",
        &["text", "image"],
        &["tool-epsilon", "tool-zeta"],
        &["loses constraints in very long tasks"],
        Some(Cost),
        None,
        42,
        0.94,
    );

    tx.commit().unwrap();
}

/// 「画像早于 `verified` 被拒」：§249 的**十态逐项**——六态 `Ok`、四态
/// `Err(ProfileBeforeVerified { state })` 且**断言是哪一枚**（设计 §4.3）。
///
/// 逐项而不是抽一个代表：允许集与拒绝集的**每一格各自能漂移**（把允许集只留 `verified`
/// 会漏放另外五态、把拒绝集改宽会多拒允许集里的某几态），抽代表时漏掉的那几格不会红。
///
/// 两侧的判据在用例里**手写**（`matches!` 的六个名字与四个名字），**不引用实现的常量**——
/// 引用它就成了拿实现给自己的集合对账。红的条件两条各在一侧：实现把允许集只留 `verified`
/// → 那五态那几条红；实现把拒绝集改宽 → 允许集那几条红。
///
/// 被拒时另断言**库里不留半写的行**：若实现先 `INSERT` 再判状态，返回值仍然是
/// `ProfileBeforeVerified`，只断言 `Err` 的用例看不出这一笔。
#[test]
fn saving_a_profile_before_verified_is_rejected_with_the_state() {
    use LifecycleState::*;

    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    register_model(&tx, &id("model-a")).unwrap();
    // 画像只能经 `load_profile` 拿到（crate 外构造不出来），故先播一行读回来，
    // 再把那一行删掉——本用例要的初始条件是「登记项在某个状态、库里还没有画像行」。
    seed_profile_row(&tx, "model-a");
    let profile = load_profile(&tx, &id("model-a")).unwrap().unwrap();
    tx.execute(
        "DELETE FROM model_profile WHERE id = ?1",
        &[Value::text("model-a")],
    )
    .unwrap();

    for (state, literal) in ALL_STATES {
        tx.execute(
            "UPDATE model_registry SET lifecycle_state = ?1 WHERE id = ?2",
            &[Value::text(literal), Value::text("model-a")],
        )
        .unwrap();

        let expected_allowed = matches!(
            state,
            Verified | Active | Stale | Degraded | Quarantined | Disabled
        );

        match save_profile(&tx, &profile) {
            Ok(()) => {
                assert!(
                    expected_allowed,
                    "{state:?} 不在允许集里，画像却存下去了（`model_profile` 行只在 probed → verified 时产生）"
                );
                assert!(
                    load_profile(&tx, &id("model-a")).unwrap().is_some(),
                    "{state:?} 允许存，库里就该有这一行"
                );
            }
            Err(LifecycleError::ProfileBeforeVerified { state: got }) => {
                assert!(
                    !expected_allowed,
                    "{state:?} 在允许集里，却被拒了（画像已产出，异常态只改可用性）"
                );
                assert_eq!(got, state, "带出的应是**读到的那一枚**状态，不是某枚默认");
                assert_eq!(
                    count_profiles(&tx),
                    0,
                    "{state:?} 被拒时不得留下半写的行（先写后判会让本断言红）"
                );
            }
            Err(other) => panic!("{state:?} 应报 ProfileBeforeVerified，实际 {other:?}"),
        }

        tx.execute(
            "DELETE FROM model_profile WHERE id = ?1",
            &[Value::text("model-a")],
        )
        .unwrap();
    }

    tx.commit().unwrap();
}

/// 库里不存在的 id → `Ok(None)`（**不是** `Err`）。
///
/// 这是设计 §2.1「画像必须来自库」这条路的**反例照片**：只有从库读才拿得到画像，
/// 而库里没有的行读出来就是「没有」。
///
/// 两侧对钉：先断言不存在的 id 是 `None`，再断言同一个 id **播种之后**读得到——
/// 只写 `None` 那侧时，「`load_profile` 恒返回 `None`」这种坏法不会红。
#[test]
fn load_profile_of_an_unknown_id_is_none() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    assert!(
        load_profile(&tx, &id("model-absent")).unwrap().is_none(),
        "库里没有这一行，读到的是「没有」，不是 `Err`"
    );

    // 对照臂：登记 + 播种之后，同一个 id 读得到画像。
    register_model(&tx, &id("model-absent")).unwrap();
    seed_profile_row(&tx, "model-absent");
    assert!(
        load_profile(&tx, &id("model-absent")).unwrap().is_some(),
        "对照臂：播种之后应读得到"
    );

    tx.commit().unwrap();
}

/// 未登记的模型存画像被**外键**拒绝（`model_profile.id REFERENCES model_registry(id)`）。
///
/// # 这一格为什么非有不可
///
/// `ModelProfile` crate 外构造不出来，但**已经有值的**画像可以在内存里传——把登记项撤掉、
/// 手里那份画像还在，这一格就是这么来的。「画像必须来自库」拦不住它，拦住它的是外键。
///
/// # 断言到内层变体
///
/// 外层须是 `LifecycleError::Persist`，内层须是 `PersistError::Database` 且信息里点了
/// 外键约束。只写 `is_err()` 的话，「报了个错」与「说清了是哪一类拒绝」分不出来。
///
/// 末尾一条对照臂：重新登记 + 迁到 `verified` 之后，**同一份画像**存得进去——
/// 挡住的是「未登记」，不是这份画像本身有什么毛病。
#[test]
fn a_profile_for_an_unregistered_model_is_rejected() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    register_model(&tx, &id("model-a")).unwrap();
    seed_profile_row(&tx, "model-a");
    let profile = load_profile(&tx, &id("model-a")).unwrap().unwrap();

    // 撤掉画像行与登记行（先撤画像行：外键在 `PRAGMA foreign_keys=ON` 下不许留下悬空引用）。
    tx.execute(
        "DELETE FROM model_profile WHERE id = ?1",
        &[Value::text("model-a")],
    )
    .unwrap();
    tx.execute(
        "DELETE FROM model_registry WHERE id = ?1",
        &[Value::text("model-a")],
    )
    .unwrap();

    match save_profile(&tx, &profile) {
        Err(LifecycleError::Persist(PersistError::Database(m))) => assert!(
            m.contains("FOREIGN KEY constraint failed"),
            "应是 model_profile.id 上的外键拒绝这一行，实际 {m}"
        ),
        Err(other) => panic!("应为 LifecycleError::Persist，实际 {other:?}"),
        Ok(()) => panic!("未登记的模型的画像不应存得下去"),
    }
    assert_eq!(count_profiles(&tx), 0, "被拒的写入不得留下行");

    // 对照臂：登记 + 走完整条画像流水线到 verified 之后，同一份画像存得进去。
    register_model(&tx, &id("model-a")).unwrap();
    advance_to_verified(&tx, "model-a");
    save_profile(&tx, &profile).unwrap();
    assert_eq!(count_profiles(&tx), 1, "同一份画像在登记并 verified 之后应存得下去");

    tx.commit().unwrap();
}

/// 三个列表列**各**一条：空数组与非空数组两侧，且空数组落 `"[]"` 而**不是 `NULL`**。
///
/// 三个列都是 `NOT NULL`，而「这个模型没有登记任何模态」与「这一列没有值」是两件事——
/// 把空数组落成 `NULL` 会让两者在库里长得一样（设计 §3.1 第 3 条：列表列**取容器**，
/// 不建子表，因为元素没有逐元素属性）。
///
/// 逐列断言（`modalities` / `tools` / `failure_modes` 各一条）而不是「有一个是空就算」：
/// 三个列能各自漂移。
///
/// 红的条件：某列把空向量写成 `NULL`、或写侧漏写某一列（原始值那一侧）、
/// 或读侧把空数组当成「列坏了」报错。
#[test]
fn the_three_list_columns_round_trip_as_json_arrays() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    // ===== 空数组那一侧 =====
    register_model(&tx, &id("model-empty")).unwrap();
    advance_to_verified(&tx, "model-empty");
    insert_profile_row(&tx, "model-empty", "[]", "[]", "[]", Some(""), None);
    let empty = load_profile(&tx, &id("model-empty")).unwrap().unwrap();
    assert!(empty.modalities().is_empty(), "空数组应读成空列表");
    assert!(empty.tools().is_empty(), "空数组应读成空列表");
    assert!(empty.failure_modes().is_empty(), "空数组应读成空列表");

    tx.execute(
        "DELETE FROM model_profile WHERE id = ?1",
        &[Value::text("model-empty")],
    )
    .unwrap();
    save_profile(&tx, &empty).unwrap();
    let raw_empty = raw_profile(&tx, "model-empty");
    assert_eq!(
        raw_empty[4],
        Value::text("[]"),
        "modalities 的空数组应落 \"[]\"，不是 NULL"
    );
    assert_eq!(
        raw_empty[5],
        Value::text("[]"),
        "tools 的空数组应落 \"[]\"，不是 NULL"
    );
    assert_eq!(
        raw_empty[6],
        Value::text("[]"),
        "failure_modes 的空数组应落 \"[]\"，不是 NULL"
    );

    // ===== 非空数组那一侧 =====
    register_model(&tx, &id("model-lists")).unwrap();
    advance_to_verified(&tx, "model-lists");
    seed_profile_row(&tx, "model-lists");
    let lists = load_profile(&tx, &id("model-lists")).unwrap().unwrap();
    assert_eq!(
        lists.modalities(),
        ["text".to_string(), "image".to_string()].as_slice(),
        "非空数组应逐元素读回"
    );
    assert_eq!(
        lists.tools(),
        [ToolId::new("tool-epsilon"), ToolId::new("tool-zeta")].as_slice(),
        "tools 的元素是 ToolId"
    );
    assert_eq!(
        lists.failure_modes(),
        ["loses constraints in very long tasks".to_string()].as_slice(),
        "非空数组应逐元素读回"
    );

    tx.execute(
        "DELETE FROM model_profile WHERE id = ?1",
        &[Value::text("model-lists")],
    )
    .unwrap();
    save_profile(&tx, &lists).unwrap();
    let raw_lists = raw_profile(&tx, "model-lists");
    assert_eq!(raw_lists[4], Value::text(MODALITIES), "modalities 的列内容");
    assert_eq!(raw_lists[5], Value::text(TOOLS), "tools 的列内容");
    assert_eq!(
        raw_lists[6],
        Value::text(FAILURE_MODES),
        "failure_modes 的列内容"
    );

    tx.commit().unwrap();
}

/// 列里的 `confidence` 是表外取值 → `load_profile` 报**具体** `Err`，**不取默认值**。
///
/// `Ratio::parse` 对非数值、非有限、越界给 `None`，`persist` 把它转成具体 `Err`
/// （设计 §2.4：构造失败在构造期就被拒，不走默认值）。四个入参各一条，覆盖四类失败：
/// 越界（`1.5`）、非数值（`abc`）、下界之外（`-0.1`）、非有限（`NaN`）。
///
/// 把表外串当成 0 或当成区间端点，正是设计要拦的（`Ratio::parse` 的注释）。
#[test]
fn an_out_of_range_confidence_in_the_column_is_rejected() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    register_model(&tx, &id("model-a")).unwrap();
    seed_profile_row(&tx, "model-a");

    for raw in ["1.5", "abc", "-0.1", "NaN"] {
        tx.execute(
            "UPDATE model_profile SET confidence = ?1 WHERE id = ?2",
            &[Value::text(raw), Value::text("model-a")],
        )
        .unwrap();

        match load_profile(&tx, &id("model-a")) {
            Err(PersistError::Database(m)) => assert!(
                m.contains("未知 Ratio") && m.contains(raw),
                "应为「未知 Ratio: {raw}」，实际 {m}"
            ),
            Err(other) => panic!("应为 PersistError::Database，实际 {other:?}"),
            Ok(read) => panic!(
                "表外置信度 {raw:?} 应被拒，实际读回成功（is_some={}）",
                read.is_some()
            ),
        }
    }

    // 对照臂：同一列写回合法值后读得到——挡住的是那个取值，不是这一行。
    tx.execute(
        "UPDATE model_profile SET confidence = '0.94' WHERE id = ?1",
        &[Value::text("model-a")],
    )
    .unwrap();
    assert!(
        load_profile(&tx, &id("model-a")).unwrap().is_some(),
        "对照臂：合法置信度应读得到"
    );

    tx.commit().unwrap();
}

/// `cost_profile` / `latency_profile` 两列的**存在性**：`NULL` 读回 `None`、非 `NULL` 读回 `Some`
/// （P3A 的存在性编码，设计 §2.5）。
///
/// 四种组合**逐项**：`(Some, Some)` / `(Some, None)` / `(None, Some)` / `(None, None)`。
/// 逐项的理由是同一条：两列各自能漂移，而**两列写串**只有在两列的存在性不同的时候才可观察
/// （两个列的字面量都是空串——`Cost` / `Latency` 的取值域为空，这是设计 §3.1 的容器形状）。
/// `(Some, None)` 与 `(None, Some)` 两格正是为此而设。
///
/// 两侧都钉：读（列 → `Option`）与写（`Option` → 列）。只钉读侧时，写侧把
/// `None` 写成空串（而不是 `NULL`）不会红。
#[test]
fn the_cost_and_latency_columns_distinguish_absent_from_registered() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    let cases: [(Option<&str>, Option<&str>); 4] = [
        (Some(""), Some("")),
        (Some(""), None),
        (None, Some("")),
        (None, None),
    ];

    for (i, (cost, latency)) in cases.into_iter().enumerate() {
        let model = format!("model-{i}");
        register_model(&tx, &id(&model)).unwrap();
        advance_to_verified(&tx, &model);
        insert_profile_row(&tx, &model, MODALITIES, TOOLS, FAILURE_MODES, cost, latency);

        // 读侧：列 → `Option`。
        let loaded = load_profile(&tx, &id(&model)).unwrap().unwrap();
        assert_eq!(
            loaded.cost_profile(),
            cost.map(|_| Cost),
            "cost 列 {cost:?} 应读成对应的 `Option<Cost>`"
        );
        assert_eq!(
            loaded.latency_profile(),
            latency.map(|_| Latency),
            "latency 列 {latency:?} 应读成对应的 `Option<Latency>`"
        );

        // 写侧：`Option` → 列。
        tx.execute(
            "DELETE FROM model_profile WHERE id = ?1",
            &[Value::text(model.clone())],
        )
        .unwrap();
        save_profile(&tx, &loaded).unwrap();
        let raw = raw_profile(&tx, &model);
        assert_eq!(
            raw[7],
            match cost {
                Some(s) => Value::text(s),
                None => Value::Null,
            },
            "cost_profile 列的原始值应与给定的存在性一致（`None` 必须落 NULL，不是空串）"
        );
        assert_eq!(
            raw[8],
            match latency {
                Some(s) => Value::text(s),
                None => Value::Null,
            },
            "latency_profile 列的原始值应与给定的存在性一致"
        );
    }

    tx.commit().unwrap();
}

/// 列表列是**表外取值**（不是 JSON 字符串数组）→ `load_profile` 报**具体** `Err`，**不取默认值**。
///
/// # 三个列表列各一条，且**分属两个解码函数**
///
/// `modalities` / `failure_modes` 走 [`decode_string_list`]，`tools` 走 [`decode_tool_ids`]
/// ——**这不是一条分支能代表的一类**：前者收 `Vec<String>`，后者收 `Vec<String>` 再逐个
/// `ToolId::new`。三条各钉一次，故两个函数各自的「容器坏了即报错」都有照片。
///
/// # 不取默认值的判据
///
/// 把坏容器静默换成空列表，会让「这个模型没有登记任何模态」与「这一列的内容是垃圾」
/// 在库里长得一样（与 `Ratio::parse` 给 `None` 同一条理由）。
///
/// 红的条件：任一解码函数改成 `unwrap_or_default()`（吞掉容器错误）即红。
#[test]
fn a_list_column_that_is_not_a_json_string_array_is_rejected() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    register_model(&tx, &id("model-a")).unwrap();
    seed_profile_row(&tx, "model-a");

    // 三条各错一个列：非 JSON（`modalities`）、JSON 但不是数组（`tools`）、
    // 是数组但元素不是字符串（`failure_modes`）。
    //
    // **每条只留一个列坏、其余先写回合法值**：`row_to_profile` 按列序解码，前一个列坏着的话
    // 后面那条会先撞上前一个列的错误，断言就点不到本条要验的那个列。
    for (column, raw, legal) in [
        ("modalities", "not-json", MODALITIES),
        ("tools", r#"{"a":1}"#, TOOLS),
        ("failure_modes", "[1,2]", FAILURE_MODES),
    ] {
        tx.execute(
            &format!("UPDATE model_profile SET {column} = ?1 WHERE id = ?2"),
            &[Value::text(raw), Value::text("model-a")],
        )
        .unwrap();

        match load_profile(&tx, &id("model-a")) {
            Err(PersistError::Database(m)) => assert!(
                m.contains(column) && m.contains("不是字符串数组") && m.contains(raw),
                "{column} 的容器坏掉时应点名该列与该串，实际 {m}"
            ),
            Err(other) => panic!("应为 PersistError::Database，实际 {other:?}"),
            Ok(read) => panic!(
                "{column} = {raw:?} 应被拒，实际读回成功（is_some={}）",
                read.is_some()
            ),
        }

        tx.execute(
            &format!("UPDATE model_profile SET {column} = ?1 WHERE id = ?2"),
            &[Value::text(legal), Value::text("model-a")],
        )
        .unwrap();
    }

    // 对照臂：三个列写回合法容器后读得到——挡住的是那三个取值，不是这一行。
    tx.execute(
        "UPDATE model_profile SET modalities = ?1, tools = ?2, failure_modes = ?3 WHERE id = ?4",
        &[
            Value::text(MODALITIES),
            Value::text(TOOLS),
            Value::text(FAILURE_MODES),
            Value::text("model-a"),
        ],
    )
    .unwrap();
    assert!(
        load_profile(&tx, &id("model-a")).unwrap().is_some(),
        "对照臂：三个列都合法时应读得到"
    );

    tx.commit().unwrap();
}

/// `cost_profile` / `latency_profile` 是**表外字面量** → `load_profile` 报**具体** `Err`，**不取默认值**。
///
/// # 这一条与 `confidence` 那一条**共用同一个转换点**
///
/// 三列都经 [`decode_literal`]（`type_name` 与 `parse` 是参数）。**据实写明**：这两条用例钉的是
/// **两个不同的 `parse` 闭包**（`Cost::parse` / `Latency::parse` 只认空串），不是两条独立分支——
/// 把 `decode_literal` 的 `.ok_or_else` 改成取默认值，两条用例会**一起**红。
/// 故「四类表外取值互不相同」这句在代码分支上只说对了两类：**列表列那类**与**其余三类**。
///
/// 红的条件：`Cost::parse` / `Latency::parse` 被放宽成「什么都收」即红
/// （如 `|_| Some(Cost)`）——那会让「已登记」与「登记了个表外串」在库里长得一样。
#[test]
fn a_cost_or_latency_column_that_is_not_the_presence_literal_is_rejected() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    register_model(&tx, &id("model-a")).unwrap();
    seed_profile_row(&tx, "model-a");

    for (column, type_name, raw) in [
        ("cost_profile", "Cost", "default"),
        ("latency_profile", "Latency", "1ms"),
    ] {
        tx.execute(
            &format!("UPDATE model_profile SET {column} = ?1 WHERE id = ?2"),
            &[Value::text(raw), Value::text("model-a")],
        )
        .unwrap();

        match load_profile(&tx, &id("model-a")) {
            Err(PersistError::Database(m)) => assert!(
                m.contains(&format!("未知 {type_name}")) && m.contains(raw),
                "{column} 的表外字面量应报「未知 {type_name}: {raw}」，实际 {m}"
            ),
            Err(other) => panic!("应为 PersistError::Database，实际 {other:?}"),
            Ok(read) => panic!(
                "{column} = {raw:?} 应被拒，实际读回成功（is_some={}）",
                read.is_some()
            ),
        }

        // 写回合法值再进下一轮：`cost_profile` 在 `latency_profile` 之前解码，
        // 留着上一轮那个坏值会让第二条断言先撞上第一条的错误。
        tx.execute(
            &format!("UPDATE model_profile SET {column} = ?1 WHERE id = ?2"),
            &[Value::text(""), Value::text("model-a")],
        )
        .unwrap();
    }

    // 对照臂：两列写回**唯一合法的那个字面量**（空串）后读得到。
    tx.execute(
        "UPDATE model_profile SET cost_profile = ?1, latency_profile = ?2 WHERE id = ?3",
        &[
            Value::text(""),
            Value::text(""),
            Value::text("model-a"),
        ],
    )
    .unwrap();
    let read = load_profile(&tx, &id("model-a")).unwrap().expect("应读得到");
    assert_eq!(read.cost_profile(), Some(Cost), "对照臂：空串解出 `Some(Cost)`");
    assert_eq!(
        read.latency_profile(),
        Some(Latency),
        "对照臂：空串解出 `Some(Latency)`"
    );

    tx.commit().unwrap();
}

/// `evidence_count` 是**负数** → `load_profile` 报**具体** `Err`，**不取默认值**。
///
/// 列声明成 `INTEGER NOT NULL`，SQLite 不禁止负整数，故这一格构得出（裸 SQL 写 `-1`）；
/// 而 §247 的 `evidence_count` 是**证据条数**，负数不是条数。把负值折算成 0 或绝对值，
/// 就是替规范发明一个它没给的处置（同「不取默认值」这条一贯判据）。
///
/// 红的条件：`u64::try_from` 换成 `unwrap_or(0)` / `max(0)` 即红。
#[test]
fn a_negative_evidence_count_in_the_column_is_rejected() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    register_model(&tx, &id("model-a")).unwrap();
    seed_profile_row(&tx, "model-a");

    // 两个负值各一条：`-1` 与 `i64::MIN`（后者是「取绝对值」这类处置会溢出的那一格）。
    for raw in [-1_i64, i64::MIN] {
        tx.execute(
            "UPDATE model_profile SET evidence_count = ?1 WHERE id = ?2",
            &[Value::Int(raw), Value::text("model-a")],
        )
        .unwrap();

        match load_profile(&tx, &id("model-a")) {
            Err(PersistError::Database(m)) => assert!(
                m.contains("evidence_count 是负数") && m.contains(&raw.to_string()),
                "负的 evidence_count 应点名该值，实际 {m}"
            ),
            Err(other) => panic!("应为 PersistError::Database，实际 {other:?}"),
            Ok(read) => panic!(
                "evidence_count = {raw} 应被拒，实际读回成功（is_some={}）",
                read.is_some()
            ),
        }
    }

    // 对照臂：写回 `0` 与 `42` 都读得到——零条证据是合法条数。
    for raw in [0_i64, 42] {
        tx.execute(
            "UPDATE model_profile SET evidence_count = ?1 WHERE id = ?2",
            &[Value::Int(raw), Value::text("model-a")],
        )
        .unwrap();
        assert_eq!(
            load_profile(&tx, &id("model-a"))
                .unwrap()
                .expect("应读得到")
                .evidence_count(),
            raw as u64,
            "对照臂：{raw} 是合法条数"
        );
    }

    tx.commit().unwrap();
}

/// 登记项的行数（`registering_the_same_id_twice…` 与 `a_transition_on_an_unknown_model…`
/// 用它断言「不得新增行」）。
fn count_registry(tx: &Tx<'_>) -> i64 {
    int_of(&tx.query("SELECT COUNT(*) FROM model_registry", &[]).unwrap()[0][0])
}

/// 直接读 `lifecycle_state` 列的**原始文本**（不经 `parse`）。
///
/// 格式类断言必须走这条路径：经 `load_lifecycle` 读回来的值已经过解码，拿它去断言
/// 「列里存的是 `discovered`」只是把编码器与解码器对了一次账，两者同错时不红。
fn raw_state(tx: &Tx<'_>, model: &str) -> String {
    let rows = tx
        .query(
            "SELECT lifecycle_state FROM model_registry WHERE id = ?1",
            &[Value::text(model)],
        )
        .unwrap();
    text_of(&rows[0][0])
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

// ===== 技能观测的落库与「当前值」（设计 §3.1、§24、§248） =====
//
// 夹具形态与上一节同：观测的 `model_id` 外键挂在 `model_profile` 上（设计 §3.1 第 1 条），
// 故每个用例都要先有一份**已画像**的模型，而第一枚画像只能由裸 SQL 播下——`save_profile`
// 收的画像 crate 外造不出，链条不能自举（理由见上一节）。观测本身走类型化接口
// （`save_skill_observation`），只有「表外取值」那一类才由裸 SQL 直接写列。

/// §248 的九维**逐项**列出（不抽代表），顺序取 `SkillDimension` 的声明序。
///
/// 与 `src/profile.rs` 的 `as_str` / `parse` 是两处副本：集成测试拿不到 crate 内的枚举表，
/// 加第十维时两处都要手工同步——这个点不设防，与 `tests/lifecycle.rs` 的 `ALL_STATES` 同形。
const ALL_DIMENSIONS: [SkillDimension; 9] = [
    SkillDimension::Reasoning,
    SkillDimension::Coding,
    SkillDimension::Vision,
    SkillDimension::Planning,
    SkillDimension::ToolUse,
    SkillDimension::ConstraintFollowing,
    SkillDimension::Verification,
    SkillDimension::Spatial,
    SkillDimension::Media,
];

/// 一条观测，五个字段由调用点给。**各用例的取值两两互不相同**：全取默认值时，
/// 「某字段漏写」与「两个字段写串」两类缺陷都会静默（同上一节各夹具的理由）。
fn observation(
    score: f64,
    confidence: f64,
    sample_count: u64,
    version: u32,
    time_range: (i64, i64),
) -> SkillObservation {
    SkillObservation::try_new(
        SkillScore::try_new(score).expect("夹具的评分应是有限实数"),
        Ratio::try_new(confidence).expect("夹具的置信度应在 [0,1] 内"),
        sample_count,
        version,
        time_range,
    )
    .expect("夹具的时间窗应自洽")
}

/// 登记 → 走完整条画像流水线到 `verified` → 裸 SQL 播一行画像：该模型「已画像」。
fn seed_profiled_model(tx: &Tx<'_>, model: &str) {
    register_model(tx, &id(model)).unwrap();
    advance_to_verified(tx, model);
    seed_profile_row(tx, model);
}

/// 裸 SQL 往 `model_skill_score` 写一行，**八列的字面量全部由调用点给**。
///
/// 用于造表外取值——类型化写入路径（`save_skill_observation`）造不出这些行
/// （它收的是已构造好的 `SkillObservation` 与 `SkillDimension`）。
fn insert_skill_row(
    tx: &Tx<'_>,
    model: &str,
    dimension: &str,
    version: i64,
    score: &str,
    confidence: &str,
    sample_count: i64,
    start: i64,
    end: i64,
) {
    tx.execute(
        "INSERT INTO model_skill_score
           (model_id, dimension, score_version, score, confidence, sample_count,
            time_range_start, time_range_end)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        &[
            Value::text(model),
            Value::text(dimension),
            Value::Int(version),
            Value::text(score),
            Value::text(confidence),
            Value::Int(sample_count),
            Value::Int(start),
            Value::Int(end),
        ],
    )
    .unwrap();
}

/// `model_skill_score` 八列的**原始值**（声明序，**不经任何解码**）。
///
/// 列清单在本函数里**手写一遍**，不从被测实现里取——这样「实现把某列写到了邻列」
/// 才会在原始值上暴露出来；若与实现共用一份列清单，两侧同错时不红。排序固定为
/// `(dimension, score_version)` 升序，使多行的用例有确定的取行顺序。
fn raw_skill_rows(tx: &Tx<'_>, model: &str) -> Vec<Vec<Value>> {
    tx.query(
        "SELECT model_id, dimension, score_version, score, confidence, sample_count,
                time_range_start, time_range_end
         FROM model_skill_score WHERE model_id = ?1 ORDER BY dimension, score_version",
        &[Value::text(model)],
    )
    .unwrap()
}

/// `model_skill_score` 的行数。
fn count_skill_rows(tx: &Tx<'_>) -> i64 {
    int_of(&tx.query("SELECT COUNT(*) FROM model_skill_score", &[]).unwrap()[0][0])
}

/// §24 的五个字段**逐项**往返：写（八列的原始值对手写字面量）与读
/// （五个字段逐项比对）**两侧都要**。
///
/// 只做读侧：`save_skill_observation` 一列都不写、或写到邻列，本用例照样全绿。
/// 只做写侧：解码把两列读串，本用例照样全绿（手写值 → 列 → 读回，两处错相互抵消）。
///
/// 五个字段取**互不相同**的非默认值（评分 `9.2`、置信度 `0.75`、样本数 `17`、版本 `3`、
/// 时间窗两端各是一个十三位的毫秒数）：任一字段漏写或与邻字段写串都在此变红。
#[test]
fn a_skill_observation_round_trips_field_by_field() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();
    seed_profiled_model(&tx, "model-alpha");

    let written = observation(9.2, 0.75, 17, 3, (1_700_000_000_123, 1_700_000_999_456));
    save_skill_observation(&tx, &id("model-alpha"), SkillDimension::Coding, &written).unwrap();

    // ===== 写侧：八列的原始值与手写字面量逐列比 =====
    assert_eq!(
        raw_skill_rows(&tx, "model-alpha"),
        vec![vec![
            Value::text("model-alpha"),
            Value::text("coding"),
            Value::Int(3),
            Value::text("9.2"),
            Value::text("0.75"),
            Value::Int(17),
            Value::Int(1_700_000_000_123),
            Value::Int(1_700_000_999_456),
        ]],
        "八列的原始值应与手写字面量逐列相同（写错列、漏写列、两列写串都在此变红）"
    );

    // ===== 读侧：五个字段逐项比 =====
    let series = load_skill_series(&tx, &id("model-alpha"), SkillDimension::Coding).unwrap();
    assert_eq!(series.len(), 1, "只写了一条观测");
    let read = &series[0];
    assert_eq!(read.score().get(), 9.2, "§24 score");
    assert_eq!(read.confidence().get(), 0.75, "§24 confidence");
    assert_eq!(read.sample_count(), 17, "§24 sample_count");
    assert_eq!(read.version(), 3, "§24 version");
    assert_eq!(
        read.time_range(),
        (1_700_000_000_123, 1_700_000_999_456),
        "§24 time_range 的**闭区间两端**"
    );

    tx.commit().unwrap();
}

/// 同 `(model_id, dimension, score_version)` 的第二次写入被**复合主键**拒，**且原行内容不变**
/// （裸 `INSERT`，不是 `INSERT OR REPLACE`；设计 §3.1 第 2 条）。
///
/// **第二条与第一条五个字段全不同**：若原行停在第二条的值，「被拒之后原行没动」在
/// `OR REPLACE` 下**照样成立**——那个形状的断言区分不出裸 `INSERT` 与 `OR REPLACE`
/// （等价变异体的判据：这两版在哪个入参上会给出不同结果）。
///
/// 红的条件两条，各在一侧：`INSERT` 被改成 `OR REPLACE`（或先 `DELETE` 再写）→ 原始值那条红；
/// 写入口整个坏掉（第二次照样被拒、第一次也写不进去）→ 第一条的 `unwrap` 就红。
#[test]
fn the_same_dimension_and_version_cannot_be_written_twice() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();
    seed_profiled_model(&tx, "model-alpha");

    let first = observation(1.0, 0.25, 5, 3, (100, 200));
    save_skill_observation(&tx, &id("model-alpha"), SkillDimension::Coding, &first).unwrap();

    let second = observation(2.0, 0.9, 9, 3, (300, 400));
    match save_skill_observation(&tx, &id("model-alpha"), SkillDimension::Coding, &second) {
        Err(PersistError::Database(m)) => assert!(
            m.contains("UNIQUE constraint failed") && m.contains("model_skill_score"),
            "第二次写入应由复合主键拒绝并点名该表，实际 {m}"
        ),
        Err(other) => panic!("应为 PersistError::Database，实际 {other:?}"),
        Ok(()) => panic!("同 (model_id, dimension, score_version) 的第二次写入不应成功"),
    }

    assert_eq!(count_skill_rows(&tx), 1, "被拒的写入不得留下第二行");
    assert_eq!(
        raw_skill_rows(&tx, "model-alpha"),
        vec![vec![
            Value::text("model-alpha"),
            Value::text("coding"),
            Value::Int(3),
            Value::text("1"),
            Value::text("0.25"),
            Value::Int(5),
            Value::Int(100),
            Value::Int(200),
        ]],
        "被拒的写入不得改写原行的任何一列（`OR REPLACE` 会让本断言红）"
    );

    tx.commit().unwrap();
}

/// §24 的「时间序列」：同维度存三个版本，`load_skill_series` 返回**三条且按 `score_version` 升序**
/// （顺序固定，调用方才不必自己排）。
///
/// **版本乱序写入**（7 → 2 → 5）——但**排序不因此可观察**：SQLite 按复合主键
/// `(model_id, dimension, score_version)` 的索引序返回这些行，与 `ORDER BY` 的升降无关。
/// 实测把整条 `ORDER BY` 删掉，本用例**仍然全绿**：那是一条**等价变异体**
/// （判据是「这两版在哪个入参上会给出不同结果」——在本表上举不出），不是本用例漏了。
/// 故红的条件是**把排序方向写成 `DESC`**。**`ORDER BY` 仍然显式写下**：本函数承诺升序，
/// 不该把这条承诺押在「查询计划恰好走索引」上。
/// （订正来历：初稿在此写「物理顺序不是升序，排序才是被测的那件事」，实测证伪。）
///
/// 另写**另一个维度**一条，故 `WHERE dimension` 那一半同时被钉——若漏掉它，本用例会读到四条。
///
/// 红的条件：排序方向反过来（`DESC`）即红；漏一个版本即红；`WHERE dimension` 取反即红。
#[test]
fn a_series_comes_back_in_ascending_score_version() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();
    seed_profiled_model(&tx, "model-alpha");

    for (version, score) in [(7_u32, 7.0), (2, 2.0), (5, 5.0)] {
        save_skill_observation(
            &tx,
            &id("model-alpha"),
            SkillDimension::Coding,
            &observation(score, 0.5, 1, version, (0, 1)),
        )
        .unwrap();
    }
    // 另一个维度的一条：它不该出现在 `Coding` 的序列里。
    save_skill_observation(
        &tx,
        &id("model-alpha"),
        SkillDimension::ToolUse,
        &observation(3.0, 0.5, 1, 9, (0, 1)),
    )
    .unwrap();

    let series = load_skill_series(&tx, &id("model-alpha"), SkillDimension::Coding).unwrap();
    assert_eq!(
        series.iter().map(|o| o.version()).collect::<Vec<_>>(),
        vec![2, 5, 7],
        "同维度的三条应按 score_version 升序（只取该维度）"
    );
    assert_eq!(
        series.iter().map(|o| o.score().get()).collect::<Vec<_>>(),
        vec![2.0, 5.0, 7.0],
        "升序之后各行的内容也逐条对得上（顺序对了但内容串行会在此变红）"
    );

    tx.commit().unwrap();
}

/// 向量里的「当前值」是**该维 `version` 最大**的那次观测，**不是** `time_range` 最晚的那次
/// （设计 §2.2；与 `src/profile.rs` 的 `current_observation` 用例同一条判据——此处钉的是
/// **落库路径**也用它，不是另一套规则）。
///
/// 两条判据在这一格上**分歧**：`version = 2` 的那条时间窗更晚（`2000..3000`），
/// `version = 5` 的那条更早（`1000..1500`）。取「**时间窗最晚**」即红。
///
/// **「取最后一条」在本题材上不红**：库按索引序返回，最后一条恰是 `version` 最大的那条，
/// 故那是一条**等价变异体**（两版在哪个入参上给出不同结果——在本表上举不出）。
/// 据实写在这里，不把它冒充成反例。
/// （订正来历：初稿在此写「取『时间窗最晚』或『最后写入的那条』即红」，后半句实测证伪。）
#[test]
fn the_vector_takes_one_observation_per_dimension_by_highest_version() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();
    seed_profiled_model(&tx, "model-alpha");

    save_skill_observation(
        &tx,
        &id("model-alpha"),
        SkillDimension::Coding,
        &observation(1.0, 0.5, 1, 2, (2_000, 3_000)),
    )
    .unwrap();
    save_skill_observation(
        &tx,
        &id("model-alpha"),
        SkillDimension::Coding,
        &observation(4.0, 0.5, 1, 5, (1_000, 1_500)),
    )
    .unwrap();

    let vector = load_skill_vector(&tx, &id("model-alpha"))
        .unwrap()
        .expect("画像在，向量应是 `Some`");
    let current = vector
        .get(SkillDimension::Coding)
        .expect("该维度有一次观测，应是 `Some`");
    assert_eq!(current.version(), 5, "当前值是 version 最大的那条");
    assert_eq!(current.score().get(), 4.0, "带出的是那一条的评分");
    assert_eq!(
        current.time_range(),
        (1_000, 1_500),
        "取的是 version 最大的那条，故时间窗是更早的那个（不是更晚的 `2000..3000`）"
    );

    tx.commit().unwrap();
}

/// 「缺席不是 0」：无名观测的维度在向量里是 `None`，**不是**某个默认观测
/// （设计 §2.2：没有样本就没有评分）。
///
/// 三侧都钉，缺一即漏一类坏法：
/// 1. **有画像、零观测** → 九维**逐项** `None`（抽代表会漏掉能各自漂移的那几维）；
/// 2. **写入两维** → 这两维 `Some`、其余七维仍 `None`（只钉第 1 侧时，「谁都返回 `None`」不红）；
/// 3. **九维全写** → 九维全 `Some` 且各是写进去的那条（钉住分组不丢维度、也不串维度）。
#[test]
fn every_dimension_without_an_observation_is_absent_not_zero() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    // ===== 第 1 侧：有画像、一条观测都没有 =====
    seed_profiled_model(&tx, "model-empty");
    let empty = load_skill_vector(&tx, &id("model-empty"))
        .unwrap()
        .expect("画像是「有」的，故向量是 `Some(空向量)`，不是 `None`");
    for dimension in ALL_DIMENSIONS {
        assert_eq!(
            empty.get(dimension),
            None,
            "{dimension:?} 尚无观测，应是 `None`（缺席不是 0）"
        );
    }

    // ===== 第 2 侧：写入两维 =====
    seed_profiled_model(&tx, "model-two");
    save_skill_observation(
        &tx,
        &id("model-two"),
        SkillDimension::Coding,
        &observation(1.0, 0.5, 1, 1, (0, 1)),
    )
    .unwrap();
    save_skill_observation(
        &tx,
        &id("model-two"),
        SkillDimension::ToolUse,
        &observation(2.0, 0.5, 1, 1, (0, 1)),
    )
    .unwrap();
    let two = load_skill_vector(&tx, &id("model-two")).unwrap().unwrap();
    assert!(
        two.get(SkillDimension::Coding).is_some(),
        "写过的维度应是 `Some`"
    );
    assert!(
        two.get(SkillDimension::ToolUse).is_some(),
        "写过的维度应是 `Some`"
    );
    for dimension in ALL_DIMENSIONS {
        if matches!(
            dimension,
            SkillDimension::Coding | SkillDimension::ToolUse
        ) {
            continue;
        }
        assert_eq!(
            two.get(dimension),
            None,
            "{dimension:?} 没写过观测，仍应是 `None`（不是被别的维度带出来的 0）"
        );
    }

    // ===== 第 3 侧：九维全写，各维的评分**两两不同** =====
    seed_profiled_model(&tx, "model-all");
    for (i, dimension) in ALL_DIMENSIONS.iter().enumerate() {
        save_skill_observation(
            &tx,
            &id("model-all"),
            *dimension,
            &observation(i as f64, 0.5, 1, 1, (0, 1)),
        )
        .unwrap();
    }
    let all = load_skill_vector(&tx, &id("model-all")).unwrap().unwrap();
    for (i, dimension) in ALL_DIMENSIONS.iter().enumerate() {
        assert_eq!(
            all.get(*dimension).map(|o| o.score().get()),
            Some(i as f64),
            "{dimension:?} 应是写进去的那一条（评分 {i}）——分组丢维或串维都在此变红"
        );
    }

    tx.commit().unwrap();
}

/// 列里存的是**手写字面量**（小写、`_` 连接），**不是** `Debug` 表示（全局约束、设计 §2.4）。
///
/// 两侧都走：写侧经**被测写入路径**读**原始列值**，与手写字面量逐字比；反向把手写字面量
/// **直接写进库**，`load_skill_series` 应解出对应的那一枚——反向那侧不依赖 `as_str`，
/// 故「写侧与读侧同时漂移」不会让本用例静默。
///
/// 只举 `tool_use` 与 `constraint_following` 两个**多词**维度：单词维度上
/// 「小写」与「`Debug` 表示」只差首字母大小写，多词维度才同时钉住 `_` 连接
/// （九维的编码表逐项照片在 `tests/profile.rs` 的 `skill_dimension_encoding_is_lowercase_with_underscores`；
/// 此处钉的是**列里的字节**）。
#[test]
fn the_dimension_column_is_lowercase_with_underscores() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();
    seed_profiled_model(&tx, "model-alpha");

    save_skill_observation(
        &tx,
        &id("model-alpha"),
        SkillDimension::ToolUse,
        &observation(1.0, 0.5, 1, 1, (0, 1)),
    )
    .unwrap();
    save_skill_observation(
        &tx,
        &id("model-alpha"),
        SkillDimension::ConstraintFollowing,
        &observation(2.0, 0.5, 1, 1, (0, 1)),
    )
    .unwrap();

    let literals: Vec<String> = tx
        .query(
            "SELECT dimension FROM model_skill_score WHERE model_id = ?1 ORDER BY dimension",
            &[Value::text("model-alpha")],
        )
        .unwrap()
        .iter()
        .map(|row| text_of(&row[0]))
        .collect();
    assert_eq!(
        literals,
        vec!["constraint_following".to_string(), "tool_use".to_string()],
        "多词维度的列内容应是手写字面量（`ToolUse` 那样的 `Debug` 表示会让本断言红）"
    );

    // 反向：手写字面量直接写进库，读侧应解出对应的那一枚。
    seed_profiled_model(&tx, "model-literal");
    for (dimension, literal) in [
        (SkillDimension::ToolUse, "tool_use"),
        (SkillDimension::ConstraintFollowing, "constraint_following"),
    ] {
        insert_skill_row(&tx, "model-literal", literal, 1, "1", "0.5", 1, 0, 1);
        assert_eq!(
            load_skill_series(&tx, &id("model-literal"), dimension)
                .unwrap()
                .len(),
            1,
            "手写字面量 {literal} 应解出 {dimension:?}"
        );
    }

    tx.commit().unwrap();
}

/// 列里的 `dimension` 是表外取值 → `load_skill_vector` 报**具体** `Err`，**不取默认值**。
///
/// 把表外串猜成某一枚维度正是设计要拦的（`SkillDimension::parse` 的注释：那是第二份表示）。
/// 两个形状各一条：未知词 `empathy`，以及 `Debug` 表示 `ToolUse`——后者正是「落库编码写错」
/// 时的典型形状，它若真被写进库，本用例是发现它的那条链的终点。
///
/// 断言里点名那个表外串：只写 `is_err()` 的话，「报了个错」与「说清了是哪个串」分不出来。
#[test]
fn an_unknown_dimension_in_the_column_is_rejected() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();
    seed_profiled_model(&tx, "model-alpha");

    insert_skill_row(&tx, "model-alpha", "empathy", 1, "1", "0.5", 1, 0, 1);
    for raw in ["empathy", "ToolUse"] {
        tx.execute(
            "UPDATE model_skill_score SET dimension = ?1 WHERE model_id = ?2",
            &[Value::text(raw), Value::text("model-alpha")],
        )
        .unwrap();

        match load_skill_vector(&tx, &id("model-alpha")) {
            Err(PersistError::Database(m)) => assert!(
                m.contains("未知 SkillDimension") && m.contains(raw),
                "应为「未知 SkillDimension: {raw}」，实际 {m}"
            ),
            Err(other) => panic!("应为 PersistError::Database，实际 {other:?}"),
            Ok(read) => panic!(
                "表外维度 {raw:?} 应被拒，实际读回成功（is_some={}）",
                read.is_some()
            ),
        }
    }

    // 对照臂：写回合法维度后读得到——挡住的是那个取值，不是这一行。
    tx.execute(
        "UPDATE model_skill_score SET dimension = 'coding' WHERE model_id = ?1",
        &[Value::text("model-alpha")],
    )
    .unwrap();
    assert!(
        load_skill_vector(&tx, &id("model-alpha")).unwrap().is_some(),
        "对照臂：合法维度应读得到"
    );

    tx.commit().unwrap();
}

/// 列里的 `score` / `confidence` 是表外取值 → **具体** `Err`，**不取默认值**。
///
/// # 这两列共用同一个转换点，据实写明
///
/// 两个列都经 `decode_literal` 那形状的转换（类型名与 `parse` 是参数），
/// 与 `model_profile` 的 `cost_profile` / `confidence` 同一处置。故这不是两条独立分支：
/// 把那个 `.ok_or_else` 改成取默认值，本用例的三条会**一起**红。
///
/// 三个入参覆盖两类失败：非有限（`NaN`）、非数值（`not-a-number`）、越界（`confidence = 1.5`）。
/// `score` 的取值域在规范里未定义（§23 只给示例 `9.2`），故 `score` 只拒「不是有限实数」那一类；
/// 越界那一类只在 `confidence` 上构得出。
#[test]
fn an_out_of_range_score_in_the_column_is_rejected() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();
    seed_profiled_model(&tx, "model-alpha");

    insert_skill_row(&tx, "model-alpha", "coding", 1, "1", "0.5", 1, 0, 1);

    // 每轮只坏一个列，其余先写回合法值：解码按列序走，前一列坏着的话后一条断言会先撞上
    // 前一个列的错误，就点不到本条要验的那个列。
    for (column, type_name, raw, legal) in [
        ("score", "SkillScore", "NaN", "1"),
        ("score", "SkillScore", "not-a-number", "1"),
        ("confidence", "Ratio", "1.5", "0.5"),
    ] {
        tx.execute(
            &format!("UPDATE model_skill_score SET {column} = ?1 WHERE model_id = ?2"),
            &[Value::text(raw), Value::text("model-alpha")],
        )
        .unwrap();

        match load_skill_vector(&tx, &id("model-alpha")) {
            Err(PersistError::Database(m)) => assert!(
                m.contains(&format!("未知 {type_name}")) && m.contains(raw),
                "{column} = {raw:?} 应报「未知 {type_name}: {raw}」，实际 {m}"
            ),
            Err(other) => panic!("应为 PersistError::Database，实际 {other:?}"),
            Ok(read) => panic!(
                "{column} = {raw:?} 应被拒，实际读回成功（is_some={}）",
                read.is_some()
            ),
        }

        tx.execute(
            &format!("UPDATE model_skill_score SET {column} = ?1 WHERE model_id = ?2"),
            &[Value::text(legal), Value::text("model-alpha")],
        )
        .unwrap();
    }

    // 对照臂：两列都合法后读得到。
    assert!(
        load_skill_vector(&tx, &id("model-alpha")).unwrap().is_some(),
        "对照臂：两列合法时应读得到"
    );

    tx.commit().unwrap();
}

/// 列里的**时间窗反序**（`time_range_end < time_range_start`）→ **具体** `Err`，**不取默认值**。
///
/// `model_skill_score` 没有 `CHECK (time_range_end >= time_range_start)`，故这一格构得出
/// （裸 SQL 写反序的两个端点）；而 `SkillObservation` 的构造期判据是 `end < start` 即
/// `ProfileError::BadTimeRange`（设计 §2.4）。**落库路径要把那一枚转成具体 `Err`**，
/// 不能吞掉——把反序的时间窗读成一个兜底观测，等于替规范发明一个它没给的取值。
///
/// 红的条件：把 `SkillObservation::try_new` 的 `Err` 吞掉（换成一个兜底观测、
/// 或把反序的那条跳过）即红。
#[test]
fn a_reversed_time_range_in_the_column_is_rejected() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();
    seed_profiled_model(&tx, "model-alpha");

    insert_skill_row(&tx, "model-alpha", "coding", 1, "1", "0.5", 1, 3_000, 2_000);

    // `3_000 -> 2_000`（反序）与 `2_000 -> 2_000`（退化区间，自洽）只差一格：
    // 后者必须**读得到**，故本用例两侧都钉——只写反序那侧时，把守卫改成 `end <= start` 不红。
    assert!(
        matches!(load_skill_vector(&tx, &id("model-alpha")), Err(PersistError::Database(m)) if m.contains("2000") && m.contains("3000")),
        "反序的时间窗应被拒，且点名那两个端点"
    );

    tx.execute(
        "UPDATE model_skill_score SET time_range_start = 2000 WHERE model_id = ?1",
        &[Value::text("model-alpha")],
    )
    .unwrap();
    assert!(
        load_skill_vector(&tx, &id("model-alpha")).unwrap().is_some(),
        "对照臂：`end == start` 是退化区间，自洽，应读得到"
    );

    tx.commit().unwrap();
}

/// 列里的 `score_version` / `sample_count` 在各自类型的取值域之外 → **具体** `Err`。
///
/// 两个列在 Rust 侧是 `u32` / `u64`，而 `INTEGER` 列不禁止负数（也不限制上界），故这三格
/// 构得出：`score_version = -1`、`score_version = 2^32`、`sample_count = -1`。
/// **不取默认值**（不折算成 0、不取绝对值）——同 `Ratio` / `evidence_count` 的处置：
/// 替规范发明一个它没给的取值，会让「越界的行」与「合法的 0」在库里长得一样。
///
/// 三格各一条而不是只取 `-1`：`u32` 与 `u64` 的**上界**是两回事，只钉负数就漏掉了溢出那一侧
/// （`2^32` 对 `u32` 越界、对 `i64` 不越界，故它钉的是 `u32` 的转换而不是 SQLite 的存储）。
#[test]
fn an_out_of_range_version_and_sample_count_in_the_column_is_rejected() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();
    seed_profiled_model(&tx, "model-alpha");

    insert_skill_row(&tx, "model-alpha", "coding", 1, "1", "0.5", 1, 0, 1);

    for (column, raw) in [
        ("score_version", -1_i64),
        ("score_version", 4_294_967_296),
        ("sample_count", -1),
    ] {
        tx.execute(
            &format!("UPDATE model_skill_score SET {column} = ?1 WHERE model_id = ?2"),
            &[Value::Int(raw), Value::text("model-alpha")],
        )
        .unwrap();

        match load_skill_vector(&tx, &id("model-alpha")) {
            Err(PersistError::Database(m)) => assert!(
                m.contains(column) && m.contains(&raw.to_string()),
                "{column} = {raw} 应被拒且点名该列与该值，实际 {m}"
            ),
            Err(other) => panic!("应为 PersistError::Database，实际 {other:?}"),
            Ok(read) => panic!(
                "{column} = {raw} 应被拒，实际读回成功（is_some={}）",
                read.is_some()
            ),
        }

        tx.execute(
            &format!("UPDATE model_skill_score SET {column} = 1 WHERE model_id = ?1"),
            &[Value::text("model-alpha")],
        )
        .unwrap();
    }

    // 对照臂：两列都写回合法值后读得到。
    assert!(
        load_skill_vector(&tx, &id("model-alpha")).unwrap().is_some(),
        "对照臂：两列合法时应读得到"
    );

    tx.commit().unwrap();
}

/// 未画像的模型写观测被**外键**拒绝（`model_id REFERENCES model_profile(id)`，设计 §3.1 第 1 条）。
///
/// 这是「没有画像就没有观测」的**库侧落点**：`save_skill_observation` 自己不查画像在不在
/// （那会是同一个判据的第二个产生点），拒绝由外键给出。登记了但还没走过画像流水线的模型
/// 就是这一格——它在 `model_registry` 里有一行，在 `model_profile` 里没有。
///
/// 断言到内层变体：外层须是 `PersistError::Database` 且信息里点了外键约束。只写 `is_err()`
/// 的话，「报了个错」与「说清了是哪一类拒绝」分不出来。
///
/// 末尾一条对照臂：播下画像之后**同一条观测**存得进去——挡住的是「没有画像」，
/// 不是这条观测本身有什么毛病。
#[test]
fn an_observation_for_an_unprofiled_model_is_rejected() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    // 登记了（`model_registry` 有一行），但画像流水线还没走完（`model_profile` 没有行）。
    register_model(&tx, &id("model-a")).unwrap();
    let obs = observation(1.0, 0.5, 1, 1, (0, 1));

    match save_skill_observation(&tx, &id("model-a"), SkillDimension::Coding, &obs) {
        Err(PersistError::Database(m)) => assert!(
            m.contains("FOREIGN KEY constraint failed"),
            "应是 model_skill_score.model_id 上的外键拒绝这一行，实际 {m}"
        ),
        Err(other) => panic!("应为 PersistError::Database，实际 {other:?}"),
        Ok(()) => panic!("未画像的模型不该存得下观测"),
    }
    assert_eq!(count_skill_rows(&tx), 0, "被拒的写入不得留下行");

    // 对照臂：画像播下之后，同一条观测存得进去。
    advance_to_verified(&tx, "model-a");
    seed_profile_row(&tx, "model-a");
    save_skill_observation(&tx, &id("model-a"), SkillDimension::Coding, &obs).unwrap();
    assert_eq!(count_skill_rows(&tx), 1, "画像就位后同一条观测应存得下去");

    tx.commit().unwrap();
}

/// 库里不存在的 id → `load_skill_vector` 给 `Ok(None)`（**不是** `Err`，也不是空向量）。
///
/// 画像不存在与「画像在、尚无观测」是两件事，两者都取 `Option` 的两侧：前者是 `None`
/// （没有这个画像），后者是 `Some(空向量)`（画像在、九维都没观测）。本用例钉前者，
/// 后者的逐项照片在 `every_dimension_without_an_observation_is_absent_not_zero`。
///
/// 两侧对钉：先断言不存在的 id 是 `None`，再断言**播种之后**同一 id 读得到——只写 `None`
/// 那侧时，「`load_skill_vector` 恒返回 `None`」这种坏法不会红。
#[test]
fn load_skill_vector_of_an_unknown_id_is_none() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    assert!(
        load_skill_vector(&tx, &id("model-absent")).unwrap().is_none(),
        "库里没有这个画像，读到的是「没有」，不是空向量"
    );

    // 对照臂：登记 + 播画像之后，同一个 id 读得到（内容为空向量）。
    seed_profiled_model(&tx, "model-absent");
    let vector = load_skill_vector(&tx, &id("model-absent"))
        .unwrap()
        .expect("对照臂：画像在，应是 `Some`");
    for dimension in ALL_DIMENSIONS {
        assert_eq!(
            vector.get(dimension),
            None,
            "对照臂：还没有观测，{dimension:?} 应是 `None`"
        );
    }

    tx.commit().unwrap();
}

/// `load_profile` **继承**了技能表的失败面（协调者裁决，2026-10-06；见计划 `## 遗留` 的
/// 「load_profile 的 skill_vector 与 rank 的输入面」条）。
///
/// `load_profile` 组合 `load_skill_vector` 来填第十二个字段，故技能列里的表外取值会让
/// **`load_profile` 本身**失败——这条通路与画像列的表外取值同一条（`PersistError`）。
/// 不写出来的话，「画像读不出来」会被读成「`model_profile` 那一行坏了」，而实际坏的是
/// 另一张表。
///
/// 本用例的画像行**十一列全部合法**（由 `seed_profile_row` 播下），只有技能列是表外的：
/// 故失败只可能来自技能表。
///
/// 红的条件：`load_profile` 不调 `load_skill_vector`（或调用时把它的 `Err` 吞掉、
/// 换成空向量）即红——它会照样读出 `Ok(Some(...))`。
///
/// 对照臂：删掉那一条坏观测之后 `load_profile` 读得到——挡住的是那个取值，不是这份画像。
#[test]
fn load_profile_fails_on_an_out_of_table_value_in_the_skill_table() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();
    seed_profiled_model(&tx, "model-alpha");

    insert_skill_row(&tx, "model-alpha", "empathy", 1, "1", "0.5", 1, 0, 1);

    match load_profile(&tx, &id("model-alpha")) {
        Err(PersistError::Database(m)) => assert!(
            m.contains("未知 SkillDimension") && m.contains("empathy"),
            "技能列的表外取值应让 load_profile 报具体 Err，实际 {m}"
        ),
        Err(other) => panic!("应为 PersistError::Database，实际 {other:?}"),
        Ok(read) => panic!(
            "技能列有表外取值时 load_profile 应失败，实际读回成功（is_some={}）",
            read.is_some()
        ),
    }

    // 对照臂：删掉那一条坏观测，同一份画像（十一列没动过）读得到。
    tx.execute(
        "DELETE FROM model_skill_score WHERE model_id = ?1",
        &[Value::text("model-alpha")],
    )
    .unwrap();
    let read = load_profile(&tx, &id("model-alpha"))
        .unwrap()
        .expect("对照臂：画像那一行一直是好的，应读得到");
    assert_profile_fields(
        &read,
        "version-beta",
        "provider-gamma",
        "revision-delta",
        &["text", "image"],
        &["tool-epsilon", "tool-zeta"],
        &["loses constraints in very long tasks"],
        Some(Cost),
        None,
        42,
        0.94,
    );

    tx.commit().unwrap();
}
