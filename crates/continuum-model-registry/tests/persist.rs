//! 三张表的建出、列清单、主键结构，以及 §21 §4.1 的登记项与生命周期读写
//! （设计 §3.1、§3.2、§3.3、§4.1）。
//!
//! **覆盖到哪就说到哪**：三张表的**列名、声明类型、主键（含 `model_skill_score` 的复合主键
//! 及其列序）**逐列钉住——清单用 `assert_eq!` 全量比对，不是「包含」。**`NOT NULL` 没有照片**，
//! 理由见下节。三张表**都有**列断言（早期版本只钉了 `model_profile` 一张，文件头却写着「列结构」，
//! 措辞大于实际覆盖面）。
//!
//! Task 6 起本文件另钉 `model_registry` 的**行级读写**：`register_model` / `load_lifecycle` /
//! `transition_in_tx`（设计 §3.2）。**`model_profile` / `model_skill_score` 两表的行级读写
//! 不在这里**——它们属后续 task，本文件对那两张表只钉结构。
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

use continuum_core::model::ModelId;
use continuum_model_registry::{
    LifecycleError, LifecycleState, load_lifecycle, p3d_model_migrations, register_model,
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
