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
//! `save_profile` / `load_profile`（设计 §4.3）。`model_skill_score` 一表仍只钉结构。
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
    LifecycleError, LifecycleState, ModelProfile, Ratio, SkillDimension, load_lifecycle,
    load_profile, p3d_model_migrations, register_model, save_profile, transition_in_tx,
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
    // 第十二个字段不在这张表里（十一列 ≠ 十二字段）。读回来的向量是**空向量**：
    // 九维全 `None`（「尚无观测」），不是九维全 0——那需要 `load_skill_vector`（后续 task）。
    for dimension in [
        SkillDimension::Reasoning,
        SkillDimension::Coding,
        SkillDimension::Media,
    ] {
        assert_eq!(
            loaded.skill_vector().get(dimension),
            None,
            "skill_vector 落 model_skill_score 表，不经 load_profile：{dimension:?} 应为 None"
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
