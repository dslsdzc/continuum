//! 三张表的迁移（设计 §3.1、§3.3）。
//!
//! 表定义与行级读写同址（同 P3A 的 `continuum-capability/src/persist.rs`）。**本 task 只落
//! 迁移**：`p3d_model_migrations()` 建出 §3.1 的三张表；`save_profile` / `load_profile` /
//! `save_skill_observation` / `load_skill_vector` / `load_skill_series` / `register_model` /
//! `load_lifecycle` / `transition`（设计 §3.2）由后续 task 落进本文件。
//!
//! # 三处判据
//!
//! 1. **`model_skill_score` 挂 `model_profile` 而不挂 `model_registry`**（设计 §3.1 第 1 条）：
//!    观测是画像的一部分，没有画像就没有观测。故 `load_skill_vector` 不可能是「对未画像的
//!    模型返回空向量」，只能是「返回该画像的向量」。
//! 2. **主键 `(model_id, dimension, score_version)` 是「同一维度的同一版本只有一次观测」的
//!    落点**（设计 §3.1 第 2 条）。`model_skill_score` 与 `model_registry` 的写入都是**裸
//!    `INSERT`**，不 `OR REPLACE`——否则「补记一次观测」会静默覆盖历史，而 §24 要的正是历史。
//! 3. **三个列表列取 JSON 数组容器**（`modalities` / `tools` / `failure_modes`），与 P3A 的
//!    `tool.required_capabilities` 同形；**不建子表**是因为它们**没有逐元素属性**——与
//!    `model_skill_score` 的分界是判据（版本、样本数、时间窗），不是「谁更长」。
//!
//! 枚举列的落库编码（`lifecycle_state` 取 [`crate::LifecycleState::as_str`]、`dimension` 取
//! [`crate::SkillDimension::as_str`]）挂在各自的类型上，**不在本文件另建一份表**（全局约束）。

use continuum_persist::Migration;

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
