//! 三张表的迁移（设计 §3.1、§3.3）。
//!
//! 表定义与行级读写同址（同 P3A 的 `continuum-capability/src/persist.rs`）。
//! Task 5 落迁移：`p3d_model_migrations()` 建出 §3.1 的三张表。Task 6 落
//! **`model_registry` 一表的行级读写**：`register_model` / `load_lifecycle` /
//! `transition_in_tx`（设计 §3.2）。Task 7 落 **`model_profile` 一表的行级读写**
//! （[`save_profile`] / [`load_profile`]）与它上面那道「画像早于 verified 被拒」的闸门
//! （设计 §4.3）。**`model_skill_score` 一表的行级读写仍由后续 task 落进本文件**
//! （`save_skill_observation` / `load_skill_vector` / `load_skill_series`）。
//!
//! # §22 的「初步画像」本层不落，这是决定不是遗漏
//!
//! §22 的流水线是「读官方文档 → 搜 model card → 收集 benchmark → 收集公开 failure mode →
//! **生成初步画像** → 执行 Active Probe → Verifier → 生成正式 Profile」
//! （`docs/spec/01-concepts.md:1022-1066`）。初步画像**先于** probe，而本层的 `model_profile`
//! 行只在 `probed → verified` 时产生（[`save_profile`] 的闸门），故**初步画像没有落库的落点**。
//!
//! **不落它是决定**：初步画像的全部内容来自公开资料、**样本数为零**；把它作为一个可路由的画像
//! 存下来，就是给一个从未实测的模型一个与实测画像同形的身份，而那正是 §21「新增模型不能直接
//! 进入自动 Router」要拦的。**它的效果由 §247 的两个字段承载**——`evidence_count` 与
//! `confidence`：初步阶段二者分别是「证据条数」与「低置信」，正式画像生成时一并写入
//! （设计 §4.3）。
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

use continuum_capability::{Cost, Latency};
use continuum_core::model::ModelId;
use continuum_core::tool::ToolId;
use continuum_persist::{Migration, PersistError, Tx, Value, value::kind_name};

use crate::error::LifecycleError;
use crate::lifecycle::{LifecycleState, transition};
use crate::profile::{ModelProfile, Ratio, SkillVector};

/// `model_profile` 的列清单（设计 §3.1 的建表 SQL 是权威取值；顺序与建表语句一致）。
///
/// 只写一份：按 id 读是唯一的读路径，[`row_to_profile`] 的下标按它数。
/// **`save_profile` 的 `INSERT` 列名不引用本常量**（那是 SQL 文本的一部分），
/// 故两处列序只能靠用例钉：`tests/persist.rs` 的
/// `the_model_profile_columns_are_exactly_the_eleven_columns` 钉**声明序**，
/// `a_profile_round_trips_field_by_field` 钉**列内容**——读回来的字段值必须等于手写字面量
/// （它同时钉住了读路径的下标与写路径的列序，两处写串都在那里变红）。
const PROFILE_COLUMNS: &str = "id, version, provider, model_revision, modalities, tools, \
                               failure_modes, cost_profile, latency_profile, evidence_count, \
                               confidence";

/// 画像**已产出**、允许存正式画像的六态（设计 §4.3）。
///
/// 四个异常态（`stale` / `degraded` / `quarantined` / `disabled`）在允许集里：它们改的是画像的
/// **可用性**，不是撤销它——§249 只禁它们进入自动路由，没说不许有画像。**逐项列出**而不是
/// 写成「`verified` 之后都行」：拒绝集 `{discovered, unprofiled, researched, probed}` 与它
/// 互不相交且合并即为十态（`tests/persist.rs` 的
/// `saving_a_profile_before_verified_is_rejected_with_the_state` 对十态逐项断言）。
const PROFILE_ALLOWED_STATES: [LifecycleState; 6] = [
    LifecycleState::Verified,
    LifecycleState::Active,
    LifecycleState::Stale,
    LifecycleState::Degraded,
    LifecycleState::Quarantined,
    LifecycleState::Disabled,
];

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

/// 画像的写入点：**先读登记项的当前状态**，不在 [`PROFILE_ALLOWED_STATES`] 内即
/// [`LifecycleError::ProfileBeforeVerified`]（设计 §4.3）。
///
/// # 返回类型是 `LifecycleError` 而不是 `PersistError`
///
/// 设计 §3.2 把本函数记成 `-> Result<(), PersistError>`，而 §4.1 / §4.3 要求它返回
/// `Err(LifecycleError::ProfileBeforeVerified { state })`——**两处相抵，`PersistError` 装不下
/// 那枚变体**（它是 `LifecycleError` 的成员，`PersistError` 没有对应臂）。取 §4.1 / §4.3 那一侧：
/// 闸门是本函数存在的理由，签名装不下闸门的错误值即签名错。落库本身的失败经
/// `#[from]` 升格成 [`LifecycleError::Persist`]，故这一枚没丢信息。**这是计划侧要改的一处**
/// （已在本 task 报告里报回）。
///
/// # 闸门为什么读库而不是收一个状态参数
///
/// 收状态参数就等于把「当前状态是什么」交给调用方声称——那是设计 §4.2 判为纸保证的那类做法。
/// 读与写在同一事务里（`Tx` 由调用方开），故「读到的状态」与「写下的那一行」是同一时刻的库。
///
/// # 未登记的模型（`load_lifecycle` 给 `None`）
///
/// **不在本函数里报错**，直接落到 `INSERT`：`model_profile.id REFERENCES model_registry(id)`
/// 会拒绝这一行（`Db::open_with` 开了 `PRAGMA foreign_keys=ON`），失败升格成
/// [`LifecycleError::Persist`]。照片是 `tests/persist.rs` 的
/// `a_profile_for_an_unregistered_model_is_rejected`。**不静默创建登记项**——登记是
/// [`register_model`] 的活（同 [`transition_in_tx`] 对 `UnknownModel` 的处置）。
///
/// # 裸 `INSERT`，不是 `OR REPLACE`
///
/// 同 [`register_model`]：同 id 的第二次写入由主键拒绝，判据在库层而非调用方自查
/// （设计 §3.1 第 2 条）。画像**何时**该被改写（重新 profiling）是 §82 的事，本层不替它开口子。
///
/// # 三个列表列与两个存在性列
///
/// 三个列表列取 JSON 字符串数组（**容器**；`tools` 的元素取 [`ToolId::as_str`]），
/// 空数组落 `"[]"` 而**不是 `NULL`**——三个列都是 `NOT NULL`，且「空数组」与「没有这一列」
/// 是两件事。`cost_profile` / `latency_profile` 走 P3A 的存在性编码（设计 §2.5）：
/// `None` → `NULL`（尚未登记画像），`Some(个体)` → 该个体的 [`Cost::as_str`] / [`Latency::as_str`]。
///
/// # 第十二个字段 `skill_vector` **不写在这里**
///
/// 画像有十二个字段，而 `model_profile` 只有**十一列**——`skill_vector` 落
/// `model_skill_score` 那张键控时间序列表（同一维度多个版本，一个列装不下）。
/// 故本函数**不按十二列写 SQL**，`skill_vector` 由 `save_skill_observation`（后续 task）写。
pub fn save_profile(tx: &Tx<'_>, profile: &ModelProfile) -> Result<(), LifecycleError> {
    if let Some(state) = load_lifecycle(tx, profile.id())? {
        if !PROFILE_ALLOWED_STATES.contains(&state) {
            return Err(LifecycleError::ProfileBeforeVerified { state });
        }
    }
    // `None`（未登记）走到这里：闸门判不了（没有状态可读），由外键拒绝这次写入。

    tx.execute(
        "INSERT INTO model_profile
           (id, version, provider, model_revision, modalities, tools, failure_modes,
            cost_profile, latency_profile, evidence_count, confidence)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        &[
            Value::text(profile.id().as_str()),
            Value::text(profile.version()),
            Value::text(profile.provider()),
            Value::text(profile.model_revision()),
            Value::text(encode_string_list(profile.modalities(), "modalities")?),
            Value::text(encode_tool_ids(profile.tools())?),
            Value::text(encode_string_list(profile.failure_modes(), "failure_modes")?),
            presence(profile.cost_profile().map(|cost| cost.as_str())),
            presence(profile.latency_profile().map(|latency| latency.as_str())),
            Value::Int(profile.evidence_count() as i64),
            Value::text(profile.confidence().as_str()),
        ],
    )?;
    Ok(())
}

/// 按 id 读一份画像。**没有这一行返回 `Ok(None)`**，不是 `Err`（同 [`load_lifecycle`]；
/// 设计 §2.1 把这条反例定为「画像必须来自库」的照片）。
///
/// # 读回来的画像里 `skill_vector` **今天**是空的
///
/// 不是默认值，也不是本函数漏读一列：`skill_vector` 根本不在 `model_profile` 表里
/// （见 [`save_profile`] 末节）。它的装载者是 `load_skill_vector`（`model_skill_score` 表的
/// **唯一**装载者，Task 8 交付），本 task 里那个函数还不存在，故 `try_new` 收到的是
/// [`SkillVector::from_current`] 的空向量（九维全 `None`，即「尚无观测」——`None` 不是 0）。
///
/// **协调者已裁（2026-10-06，见计划 `## 遗留` 的「load_profile 的 skill_vector」条）**：
/// Task 8 交付 `load_skill_vector` 时**一并让本函数调它来填这个字段**，故本函数读回的画像
/// 到那时十二个字段齐全。**「组合」不等于「第二个装载者」**：查 `model_skill_score` 的只有
/// `load_skill_vector` 一个，本函数只是把它的结果放进画像。
/// **本 task 不预先铺这条通路**（`load_skill_vector` 还不存在），也**不要**在这里自己写一遍
/// 那张表的查询——那是把唯一装载者拆成两个。
///
/// # 表外取值一律具体 `Err`（四类，逐类有照片）
///
/// 四类表外取值**逐条转成具体 `Err`，不取默认值**——把表外串猜成某一枚会成为第二份表示
/// （同 `LifecycleState::parse` 的理由）：(a) 三个列表列不是 JSON 字符串数组、
/// (b) `cost_profile` / `latency_profile` 不是 `Cost` / `Latency` 的那一个字面量、
/// (c) `confidence` 不是 `[0,1]` 内的十进制串（[`Ratio::parse`] 给 `None`）、
/// (d) `evidence_count` 是负数。非文本的列值另报 [`PersistError::ColumnType`]。
///
/// 照片：`tests/persist.rs` 的 `a_list_column_that_is_not_a_json_string_array_is_rejected`（a）、
/// `a_cost_or_latency_column_that_is_not_the_presence_literal_is_rejected`（b）、
/// `an_out_of_range_confidence_in_the_column_is_rejected`（c）、
/// `a_negative_evidence_count_in_the_column_is_rejected`（d）。
///
/// **但「四类」是按出错形状分的，不是按代码分支分的**——据实写在下面，
/// 免得把「四条用例」读成「四条互不相同的守卫」：
/// (b) 与 (c) **共用同一个转换点** [`decode_literal`]（`type_name` 与 `parse` 是参数），
/// 把那里的 `.ok_or_else` 改成取默认值，两条用例**一起**红；(a) 与 (d) 各自独占一个转换点
/// （[`decode_string_list`] / [`decode_tool_ids`]，以及 `u64::try_from`）。故**代码分支只有三处**。
pub fn load_profile(tx: &Tx<'_>, id: &ModelId) -> Result<Option<ModelProfile>, PersistError> {
    let rows = tx.query(
        &format!("SELECT {PROFILE_COLUMNS} FROM model_profile WHERE id = ?1"),
        &[Value::text(id.as_str())],
    )?;
    let Some(row) = rows.into_iter().next() else {
        return Ok(None);
    };
    row_to_profile(&row).map(Some)
}

/// 一行 → 画像。列序与 [`PROFILE_COLUMNS`] 一一对应。
///
/// 十二条字段里读十一条列：`skill_vector` 由空向量填（理由见 [`load_profile`]）。
fn row_to_profile(row: &[Value]) -> Result<ModelProfile, PersistError> {
    let id = ModelId::new(text_at(row, 0)?);
    let modalities = decode_string_list(&text_at(row, 4)?, "modalities")?;
    let tools = decode_tool_ids(&text_at(row, 5)?)?;
    let failure_modes = decode_string_list(&text_at(row, 6)?, "failure_modes")?;

    let cost_profile = match optional_text_at(row, 7)? {
        None => None,
        Some(raw) => Some(decode_literal(&raw, "Cost", Cost::parse)?),
    };
    let latency_profile = match optional_text_at(row, 8)? {
        None => None,
        Some(raw) => Some(decode_literal(&raw, "Latency", Latency::parse)?),
    };

    let raw_evidence = int_at(row, 9)?;
    let evidence_count = u64::try_from(raw_evidence).map_err(|_| {
        PersistError::Database(format!("evidence_count 是负数：{raw_evidence}"))
    })?;
    let confidence = decode_literal(&text_at(row, 10)?, "Ratio", Ratio::parse)?;

    Ok(ModelProfile::try_new(
        id,
        text_at(row, 1)?,
        text_at(row, 2)?,
        text_at(row, 3)?,
        modalities,
        tools,
        // 第十二个字段不在这张表里（见 `load_profile`）：空向量 = 九维都尚无观测。
        SkillVector::from_current(Vec::new()),
        failure_modes,
        latency_profile,
        cost_profile,
        evidence_count,
        confidence,
    ))
}

/// 存在性编码（同 P3A 的 `tool.cost`）：`None` → `NULL`（尚未登记画像），
/// `Some(字面量)` → 该字面量。 [`Cost::as_str`] / [`Latency::as_str`] 都是**空串**
/// （取值域为空，设计 §3.1），故这里的空串**不是**「空 / 未知 / 未登记」——「未登记」是 `NULL`。
fn presence(literal: Option<&str>) -> Value {
    match literal {
        Some(s) => Value::text(s),
        None => Value::Null,
    }
}

/// 自由文本列表列的容器编码：JSON 字符串数组。
///
/// 选 JSON 数组的理由与三个列的存在理由同在（设计 §3.1 第 3 条）：元素是自由文本，
/// 定长分隔符在有取值域的那天会撞上取值本身。**只做容器**——元素的编码不在这里。
fn encode_string_list(items: &[String], column: &str) -> Result<String, PersistError> {
    serde_json::to_string(items)
        .map_err(|e| PersistError::Database(format!("{column} 不可序列化为 JSON: {e}")))
}

/// [`encode_string_list`] 的逆。**容器坏了才报错**：元素是自由文本，没有表外取值可言。
fn decode_string_list(raw: &str, column: &str) -> Result<Vec<String>, PersistError> {
    serde_json::from_str(raw).map_err(|e| {
        PersistError::Database(format!("{column} 不是字符串数组（{raw}）: {e}"))
    })
}

/// `tools` 列的容器编码：JSON 字符串数组，元素取 [`ToolId::as_str`]。
///
/// 与 [`encode_string_list`] 分开写，是因为元素类型不同（[`ToolId`] 而不是 `String`）——
/// 合起来要一个「转成串」的闭包，而那样调用点就看不出元素取的是哪个编码函数。
fn encode_tool_ids(tools: &[ToolId]) -> Result<String, PersistError> {
    let names: Vec<&str> = tools.iter().map(ToolId::as_str).collect();
    serde_json::to_string(&names)
        .map_err(|e| PersistError::Database(format!("tools 不可序列化为 JSON: {e}")))
}

/// [`encode_tool_ids`] 的逆。[`ToolId::new`] 收自由文本，故没有表外取值这一类失败。
fn decode_tool_ids(raw: &str) -> Result<Vec<ToolId>, PersistError> {
    let names: Vec<String> = serde_json::from_str(raw)
        .map_err(|e| PersistError::Database(format!("tools 不是字符串数组（{raw}）: {e}")))?;
    Ok(names.into_iter().map(ToolId::new).collect())
}

/// 按各类型自己那一对 `as_str` / `parse` 解码一个标量列，`parse` 给 `None` 即具体 `Err`。
///
/// 三个标量列（`cost_profile` / `latency_profile` / `confidence`）的处置形状相同，
/// 差异只有类型名与那个 `parse` 函数，故收在一处：**「不取默认值」这条判据只写一遍**。
fn decode_literal<T>(
    raw: &str,
    type_name: &str,
    parse: impl FnOnce(&str) -> Option<T>,
) -> Result<T, PersistError> {
    parse(raw).ok_or_else(|| PersistError::Database(format!("未知 {type_name}: {raw}")))
}

/// 按下标取列，用 `convert` 把该列的 [`Value`] 收窄到 `T`（同 P3A 的同名函数）。
///
/// **`PersistError::ColumnType` 只在本函数里构造一处**：三个具体取列函数只提供各自的
/// `convert`。收窄不了的两种情形靠 `actual` 区分：列在但类型不符记该值的 [`kind_name`]，
/// 列缺失（`convert` 根本没被调到）记 `"missing"`。
fn column_at<T>(
    row: &[Value],
    index: usize,
    convert: impl Fn(&Value) -> Option<T>,
) -> Result<T, PersistError> {
    row.get(index)
        .and_then(convert)
        .ok_or_else(|| PersistError::ColumnType {
            index,
            actual: row.get(index).map_or("missing", kind_name),
        })
}

fn as_text(v: &Value) -> Option<String> {
    match v {
        Value::Text(s) => Some(s.clone()),
        _ => None,
    }
}

fn as_int(v: &Value) -> Option<i64> {
    match v {
        Value::Int(i) => Some(*i),
        _ => None,
    }
}

/// `NULL` 收窄成 `Some(None)`（「该列是空的」也是一个可观察的结果，不是收窄失败），
/// 文本收窄成 `Some(Some(text))`，其余 `None`（收窄失败）。
fn as_optional_text(v: &Value) -> Option<Option<String>> {
    match v {
        Value::Null => Some(None),
        Value::Text(s) => Some(Some(s.clone())),
        _ => None,
    }
}

fn text_at(row: &[Value], index: usize) -> Result<String, PersistError> {
    column_at(row, index, as_text)
}

/// `NULL` → `None`，文本 → `Some`，其余报错。
fn optional_text_at(row: &[Value], index: usize) -> Result<Option<String>, PersistError> {
    column_at(row, index, as_optional_text)
}

fn int_at(row: &[Value], index: usize) -> Result<i64, PersistError> {
    column_at(row, index, as_int)
}
