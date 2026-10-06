//! 三张表的迁移（设计 §3.1、§3.3）。
//!
//! 表定义与行级读写同址（同 P3A 的 `continuum-capability/src/persist.rs`）。
//! Task 5 落迁移：`p3d_model_migrations()` 建出 §3.1 的三张表。Task 6 落
//! **`model_registry` 一表的行级读写**：`register_model` / `load_lifecycle` /
//! `transition_in_tx`（设计 §3.2）。**`model_profile` / `model_skill_score` 两表的行级读写
//! 仍由后续 task 落进本文件**（`save_profile` / `load_profile` / `save_skill_observation` /
//! `load_skill_vector` / `load_skill_series`）。
//!
//! # 三处判据
//!
//! 三条都是**表结构对读写函数的约束**，照片在**有产生方**的那一处；本文件如实标出各自
//! 已有与尚缺的照片，别把「注释写了」读成「已覆盖」。
//!
//! 1. **`model_skill_score` 挂 `model_profile` 而不挂 `model_registry`**（设计 §3.1 第 1 条）：
//!    观测是画像的一部分，没有画像就没有观测。落在 `load_skill_vector`（后续 task）上，
//!    即它不可能是「对未画像的模型返回空向量」，只能是「返回该画像的向量」。
//! 2. **主键「同一维度的同一版本只有一次观测」的落点**（设计 §3.1 第 2 条）：`model_skill_score`
//!    与 `model_registry` 的写入一律**裸 `INSERT`**、不 `OR REPLACE`——否则「补记一次观测」
//!    会静默覆盖历史，而 §24 要的正是历史。**`model_registry` 那一半已有照片**：本表的写入
//!    入口是 [`register_model`]（裸 `INSERT`），照片是
//!    `tests/persist.rs` 的 `registering_the_same_id_twice_is_rejected_and_the_row_is_unchanged`
//!    （原行先被迁离初值，故 `OR REPLACE` 会被区分出来）。**`model_skill_score` 那一半仍无照片**，
//!    随它的写入函数在后续 task 落地。
//! 3. **三个列表列取 JSON 数组容器**（`modalities` / `tools` / `failure_modes`），与 P3A 的
//!    `tool.required_capabilities` 同形；**不建子表**是因为它们**没有逐元素属性**——与
//!    `model_skill_score` 的分界是判据（版本、样本数、时间窗），不是「谁更长」。
//!
//! 枚举列的落库编码（`lifecycle_state` 取 [`crate::LifecycleState::as_str`]、`dimension` 取
//! [`crate::SkillDimension::as_str`]）挂在各自的类型上，**不在本文件另建一份表**（全局约束）。

use continuum_core::model::ModelId;
use continuum_persist::{Migration, PersistError, Tx, Value, value::kind_name};

use crate::error::LifecycleError;
use crate::lifecycle::{LifecycleState, transition};

/// 本 crate 注册的迁移：§3.1 的三张表全在**一条**迁移里。
///
/// 编号 80：1、2 是 `continuum-persist` 的内建（events、audit_log），10 是 P1 的 artifact，
/// 20 是 P1 的 graph，30 是 P2 的 workspace，40 是 P2 的 effect，41 是 P2 的 policy，
/// 50 是 P3A 的 capability——八条都在**本驱动装配的那个集合**里（`main.rs` 的
/// `runtime_migrations()`），80 在该集合里未被占用。**这个结论是现场读出来的**，不是照抄
/// 任务书：两处转录（`main.rs` 的装配处与 `tests/migrations.rs` 的 `expected_migrations()`）
/// 逐条对过，唯一性另由 `migration_versions_are_unique` 逐条断言。
///
/// **「未被占用」是按库说的，不是全仓**（设计 §3.3）：`crates/continuum-persist/src/bin/crash-writer.rs`
/// 的 `60` 与 `crates/continuum-persist/tests/recovery.rs` 的探针表 `50` 都在**别的库**里，
/// 与驱动的集合**永不同库**——故不得据此认为「60 / 50 已占」，也不得据全仓 grep 认为某号已占。
///
/// **只取 80、不预留 81**（设计 §3.3）：三张表在某一条迁移里，本子项目没有第二个建表点，
/// 预留一个没有表要建的编号就是留一条死迁移。
///
/// 一条迁移含多条语句，`Db::migrate` 用 `execute_batch` 执行（`crates/continuum-persist/src/db.rs`）。
pub fn p3d_model_migrations() -> Vec<Migration> {
    vec![Migration::new(
        80,
        "p3d_model_registry",
        "CREATE TABLE model_registry (          -- §21 §249：登记与生命周期
            id               TEXT PRIMARY KEY, -- §81／§315 的模型 id
            lifecycle_state  TEXT NOT NULL     -- §249 十态，小写 _ 连接
        );

        CREATE TABLE model_profile (           -- §247：画像（probed → verified 时才产生，见 §4.3）
            id               TEXT PRIMARY KEY REFERENCES model_registry(id),
            version          TEXT NOT NULL,
            provider         TEXT NOT NULL,
            model_revision   TEXT NOT NULL,
            modalities       TEXT NOT NULL,    -- JSON 字符串数组
            tools            TEXT NOT NULL,    -- JSON 字符串数组，元素是 ToolId
            failure_modes    TEXT NOT NULL,    -- JSON 字符串数组
            cost_profile     TEXT,             -- NULL = 尚未登记；非 NULL = 已登记（P3A 的存在性编码）
            latency_profile  TEXT,             -- 同上
            evidence_count   INTEGER NOT NULL,
            confidence       TEXT NOT NULL     -- Ratio 的十进制串
        );

        CREATE TABLE model_skill_score (       -- §248 §24：时间序列，不是常数
            model_id       TEXT NOT NULL REFERENCES model_profile(id),
            dimension      TEXT NOT NULL,      -- §248 九维，小写 _ 连接
            score_version  INTEGER NOT NULL,   -- §24 的 version
            score          TEXT NOT NULL,
            confidence     TEXT NOT NULL,
            sample_count   INTEGER NOT NULL,
            time_range_start INTEGER NOT NULL, -- Unix 毫秒
            time_range_end   INTEGER NOT NULL,
            PRIMARY KEY (model_id, dimension, score_version)
        );",
    )]
}

/// §21 的「发现即登记」：新模型进 Registry，初始状态 `discovered`。
///
/// **裸 `INSERT`，不是 `INSERT OR REPLACE`**：同 id 的第二次登记由主键拒绝，判据在库层
/// 而非调用方自查（与 P3A 的 `save_tool` 同一判据，设计 §3.1 第 2 条）。这一点有照片——
/// `tests/persist.rs` 的 `registering_the_same_id_twice_is_rejected_and_the_row_is_unchanged`
/// 先把原行迁离初值再重登记，故 `OR REPLACE` 会被区分出来（若原行停在初值，
/// 「被拒后仍是初值」在 `OR REPLACE` 下照样成立）。
///
/// 初值经 [`LifecycleState::as_str`] 编码，不写 SQL 字面量（全局约束：本 crate 的枚举列编码
/// 挂在类型上，且不依赖 serde、不用 `Debug`）。
pub fn register_model(tx: &Tx<'_>, id: &ModelId) -> Result<(), PersistError> {
    tx.execute(
        "INSERT INTO model_registry (id, lifecycle_state) VALUES (?1, ?2)",
        &[
            Value::text(id.as_str()),
            Value::text(LifecycleState::Discovered.as_str()),
        ],
    )?;
    Ok(())
}

/// 读一个登记项的当前状态。**没有这一行返回 `Ok(None)`**，不是 `Err`
/// （`LifecycleError::UnknownModel` 用在**转移**上——没有登记项可改）。
///
/// 表外取值（列里不是十态之一）返回**具体** `Err`，**不取默认值**：把串猜成另一枚状态
/// 会成为第二份表示（同 `LifecycleState::parse` 与 P3A 的 `decode_capabilities` 的理由），
/// 且会掩盖「有人往库里写了别的东西」。非文本的列值另报 [`PersistError::ColumnType`]。
pub fn load_lifecycle(tx: &Tx<'_>, id: &ModelId) -> Result<Option<LifecycleState>, PersistError> {
    let rows = tx.query(
        "SELECT lifecycle_state FROM model_registry WHERE id = ?1",
        &[Value::text(id.as_str())],
    )?;
    let Some(row) = rows.into_iter().next() else {
        return Ok(None);
    };
    let raw = match row.into_iter().next() {
        Some(Value::Text(s)) => s,
        other => {
            return Err(PersistError::ColumnType {
                index: 0,
                actual: other.as_ref().map(kind_name).unwrap_or("missing"),
            });
        }
    };
    LifecycleState::parse(&raw)
        .map(Some)
        .ok_or_else(|| PersistError::Database(format!("未知 LifecycleState: {raw}")))
}

/// §4.1 迁移表的落库版：读当前状态 → 内存 [`transition`] → 写回。
///
/// **返回的是迁移前的旧态（`from`）**，不是 `to`（设计 §3.2 的定稿；理由同内存版）。
/// 落库的是 `to`，经 [`LifecycleState::as_str`] 编码。
///
/// # 失败不留下半写的行
///
/// 两次失败都在**写之前**返回：没有登记项时 [`LifecycleError::UnknownModel`]（**不静默创建**
/// ——登记是 [`register_model`] 的活），非法对时 [`LifecycleError::Illegal`]。
/// 故「失败路径只断言是哪一种 `Err`」之外，还要断言库里的行没动——
/// `tests/persist.rs` 的 `an_illegal_transition_leaves_the_row_unchanged` 读回库里的值，
/// 专治「先写 `to`、再去查表」这种先斩后奏（它的返回值同样是 `Illegal`，只断言 `Err` 看不出）。
///
/// 读与写在同一事务里（`Tx` 由调用方开），故「读到的旧态」与「写回的那一行」是同一行。
pub fn transition_in_tx(
    tx: &Tx<'_>,
    id: &ModelId,
    to: LifecycleState,
) -> Result<LifecycleState, LifecycleError> {
    let from = load_lifecycle(tx, id)?
        .ok_or_else(|| LifecycleError::UnknownModel { id: id.clone() })?;
    // 非法对在此返回——**在下面那次 execute 之前**，故失败不留半写的行。
    let from = transition(from, to)?;
    tx.execute(
        "UPDATE model_registry SET lifecycle_state = ?1 WHERE id = ?2",
        &[Value::text(to.as_str()), Value::text(id.as_str())],
    )?;
    Ok(from)
}
