//! 本 crate 的错误类型。计划「文件结构」一节把 `IntentError、SpecError、ContractError、
//! PlanError、DecisionError、ConstraintError` 都列在 `src/error.rs`；**本文件落六枚**：
//! `IntentError`（Task 8）、`IntentStatusError`（Task 9）、`SpecError`（Task 10）、
//! `ContractError`（Task 11）、`PlanError`（Task 13）、`DecisionError`（Task 15）。
//! **`ConstraintError` 不在这里**——Task 16 落地时收在 `constraint.rs`（与
//! `VerificationRequirement` 的判定面同址；计划那一格的张力见 Task 16 的台账条目）。

use continuum_persist::PersistError;

use crate::intent::{IntentKind, IntentStatus};
use crate::plan::{PlanType, ReviewState};

/// §223 的 `COMPLETED → EVOLVING` 只在 `kind = PRODUCT` 时合法（原文「Product Intent MAY……」）。
///
/// # 两枚各有真实产生方，一条不漏
///
/// - [`IntentStatusError::Illegal`]：迁移对不在 §5.3 的迁移表内——由
///   [`crate::IntentStatus::transition`] 的查表处产出，`from` / `to` 原样带出。
/// - [`IntentStatusError::NotProduct`]：`COMPLETED → EVOLVING` 而 `kind != PRODUCT`，
///   在同一处产出，`kind` 是**库里读到的那个**（不是被映射过的任何东西）。
///
/// # 照片
///
/// 判定（§5.3）在 `intent.rs` 是 `pub(crate)`（本类型之外的公开面只有
/// [`crate::apply_intent_transition`]），故判定的照片全部经落库版落在
/// `tests/intent_state.rs`：`every_listed_pair_is_accepted` 逐条钉表（31 条）、
/// `unlisted_pairs_are_illegal_…` 与 `self_loops_are_decided_on_both_sides`
/// 各钉 [`IntentStatusError::Illegal`] 的两侧、`completed_to_evolving_depends_on_the_kind`
/// 两侧钉 [`IntentStatusError::NotProduct`]。
///
/// # 为什么不并进 `IntentError` 的两枚「表外取值」
///
/// [`crate::IntentError::UnknownStatus`] 说的是「库里 `status` 列的值不在十三枚里」
/// （数据坏了），本类型说的是「这次迁移不合法」（数据是对的、动作是错的）——两类错误
/// 调用方的处置不同（修动作／修数据），同 P3D 把 `Illegal` 与 `Persist` 分在两处的理由。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IntentStatusError {
    /// 迁移对不在 §5.3 的迁移表里。`from` / `to` 原样带出——只说「这个迁移不合法」
    /// 而不给是哪一对，调用方无法判断是来源错还是目标错（形状取自 `continuum-graph`
    /// 的 `StateError::Illegal` 与 P3D 的 `LifecycleError::Illegal`）。
    #[error("迁移 {from:?} → {to:?} 不在 §223／§5.3 的迁移表内")]
    Illegal { from: IntentStatus, to: IntentStatus },

    /// `COMPLETED → EVOLVING` 只对 Product Intent 合法（§223）。`kind` 原样带出。
    #[error("kind = {kind:?}：COMPLETED → EVOLVING 只对 Product Intent 合法（§223）")]
    NotProduct { kind: IntentKind },
}

/// Intent 落库与读回的错误。
///
/// # 五枚各有真实产生方，一条不漏
///
/// - [`IntentError::UnknownKind`]：`intent` 表的 `kind` 列是**封闭六值**
///   （[`crate::IntentKind`]），库里读到表外取值（历史行、被外部改过的行）时由
///   [`crate::load_intent`] / [`crate::list_children`] 的解码处产出，
///   **带库里那一个原始串**。
/// - [`IntentError::UnknownStatus`]：同上，`status` 列是闭集十三枚
///   （[`crate::IntentStatus`]）。
/// - [`IntentError::Transition`]：§5.3 的迁移判定失败（Task 9 新增），
///   [`crate::apply_intent_transition`] 产出——与 `UnknownStatus` 的分界见
///   [`IntentStatusError`] 的文档。
/// - [`IntentError::UnknownIntent`]：没有这个 Intent 可迁移（Task 9 新增），
///   [`crate::apply_intent_transition`] 产出。
/// - [`IntentError::Persist`]：`Tx` 的往返失败，`#[from]` 转出——**不吞成上面几枚**：
///   数据库层的失败若被降级成 `UnknownKind` / `UnknownStatus` / `UnknownIntent`，
///   调用方会以为「数据就是这样」，而真相是这次压根没读到（同 `continuum_canonical::AliasError`
///   的 `Persist` 那条理由）。
///
/// **两枚「表外取值」是分开的，不合并成一枚 `UnknownValue { column, value }`**：
/// 合并之后「kind 表外」与「status 表外」在类型上不再可分辨，而两者的处置方向可能不同
/// （一个要扩枚举、一个要修这一行）。分开是**一字之省**的代价换来可分辨的裁决。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IntentError {
    /// `kind` 列是表外取值。`value` 是**库里那一个**原始取值，不是被映射过的任何东西。
    #[error("表外的 intent kind 取值: {value}")]
    UnknownKind { value: String },

    /// `status` 列是表外取值。`value` 同上。
    #[error("表外的 intent status 取值: {value}")]
    UnknownStatus { value: String },

    /// §5.3 的迁移判定失败（[`IntentStatusError`] 的两枚之一）。
    #[error(transparent)]
    Transition(#[from] IntentStatusError),

    /// 库里没有这个 Intent，没有状态可迁移。`id` 是**入参那一个**的里子串。
    ///
    /// **产生方只有 [`crate::apply_intent_transition`]**；[`crate::load_intent`] 对不存在的
    /// id 返回 `Ok(None)`——读一个不存在的 Intent 是「没有」，不是「出错」（同
    /// `continuum_canonical::AliasError::NotFound` 与 P3D 的 `UnknownModel` 的处置）。
    /// **不静默创建**：登记是 [`crate::save_intent`] 的活；把一次笔误的 id 变成一行
    /// 新 Intent，会让「没见过的 Intent」与「打错字的 Intent」在库里长得一样。
    ///
    /// **里子是 `String` 而不是 [`crate::IntentId`]**：本层没有给 `IntentId` 实现
    /// `PartialEq`（见 `ids.rs` 的模块文档），而本类型派生 `PartialEq`；同
    /// `continuum_canonical::AliasError::NotFound { alias: String }`。
    /// **照片**：`tests/intent_state.rs` 的 `an_unknown_intent_is_rejected_and_names_the_id`。
    #[error("intent 不存在: {id}")]
    UnknownIntent { id: String },

    /// `Tx` 的往返失败。
    #[error(transparent)]
    Persist(#[from] PersistError),
}

/// Specification 落库与读回的错误（Task 10）。
///
/// # 四枚各有真实产生方，一条不漏
///
/// - [`SpecError::UnknownsKind`]：`specification.unknowns_kind` 列是**闭集两枚**
///   （`none_found` / `enumerated`，编码 / 解码挂在 [`crate::Unknowns::kind_str`] /
///   [`crate::Unknowns::parse_kind`] 上），
///   库里读到表外取值（历史行、被外部改过的行）时由 [`crate::load_specification`]
///   的解码处产出，**带库里那一个原始串**。
/// - [`SpecError::UnknownItemKind`]：`specification.unknowns` 列里元素的 `kind`
///   是**闭集两枚**（`conflict` / `undefined_behavior`，编码 / 解码挂在
///   [`crate::Unknown::kind_str`] / [`crate::Unknown::parse_kind`] 上），
///   表外取值在同一解码处产出，带原始串。
/// - [`SpecError::Intent`]：组合写路径 [`crate::save_intent_with_specification`]
///   里 Intent 侧的失败（含 `save_intent` 自己的 `Persist` 一枚），`#[from]` 转出。
/// - [`SpecError::Persist`]：`Tx` 的往返失败，`#[from]` 转出。**容器坏**（列表列不是
///   JSON 数组、`unknowns` 元素不是 `{kind, value}` 对象等）也走这一枚的
///   `Database` 形状——与 `continuum-model-registry` 的列表列同一条分工
///   （「容器坏」与「表外取值」是两类失败，别处有各自先例）。
///
/// **两枚表外取值是分开的，不合并成一枚**：合并之后「`unknowns_kind` 列坏了」与
/// 「列表里某一项坏了」在类型上不再可分辨，而两者的处置方向可能不同（一个要修那一行、
/// 一个要修列表里那一项）——与 [`IntentError`] 的两枚同一条理由。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SpecError {
    /// `unknowns_kind` 列是表外取值。`value` 是**库里那一个**原始取值。
    #[error("表外的 specification unknowns_kind 取值: {value}")]
    UnknownsKind { value: String },

    /// `unknowns` 列表里元素的 `kind` 是表外取值。`value` 同上。
    #[error("表外的 unknown 元素 kind 取值: {value}")]
    UnknownItemKind { value: String },

    /// 组合写路径里 Intent 侧的失败。
    #[error(transparent)]
    Intent(#[from] IntentError),

    /// `Tx` 的往返失败（含容器坏）。
    #[error(transparent)]
    Persist(#[from] PersistError),
}

/// Contract 落库与读回的错误（Task 11）。
///
/// # 两枚各有真实产生方，一条不漏
///
/// - [`ContractError::UnknownClass`]：`contract_requirement.class` 列是**闭集四枚**
///   （编码 / 解码挂在 [`crate::RequirementClass::as_str`] / `parse` 上，
///   与类型同址），库里读到表外取值（历史行、被外部改过的行）时由
///   [`crate::load_contract`] 的解码处产出，**带库里那一个原始串**。
/// - [`ContractError::Persist`]：`Tx` 的往返失败，`#[from]` 转出。**`(id, version)` 的
///   二次写**（§226 的 MUST NOT 原地修改）也走这一枚的 `Database` 形状——
///   `PersistError` 没有独立的冲突变体，且 `Tx::execute` 把 rusqlite 的错误码抹成了
///   字符串，故调用方只能在 `Database` 内按消息识别冲突（同 `continuum-effect` 的
///   既有处置与 `continuum-workspace` 的 gate 审计失败）。**不吞成 `UnknownClass`**：
///   数据库层的失败若被降级，调用方会以为「数据就是这样」，而真相是这次压根没读到
///   （同 [`IntentError::Persist`] 与 [`SpecError::Persist`] 的理由）。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ContractError {
    /// `class` 列是表外取值。`value` 是**库里那一个**原始取值，不是被映射过的任何东西。
    #[error("表外的 requirement class 取值: {value}")]
    UnknownClass { value: String },

    /// `Tx` 的往返失败（含 `(id, version)` 撞主键）。
    #[error(transparent)]
    Persist(#[from] PersistError),
}

/// Plan 的审查状态机与执行门的错误（Task 13）、`plan` 的行级读写（Task 15）。
///
/// # 五枚各有真实产生方，一条不漏
///
/// - [`PlanError::IllegalReview`]：迁移对不在 §9.2 的迁移表内——由
///   [`crate::ReviewState::transition`] 的查表处产出，`from` / `to` 原样带出。
/// - [`PlanError::UserApprovalRequired`]：§229 的执行门——`STRATEGIC` 的 Plan 未经
///   `USER_APPROVED` 被要求执行，由 [`crate::ReviewState::reaches_ready`] 产出，
///   `plan_type` 与 `state` 是**当时那一对**（不是被映射过的任何东西）。
/// - [`PlanError::UnknownType`] / [`PlanError::UnknownReview`]（Task 15 新增）：
///   `plan.type` / `plan.review_state` 两列是**封闭枚举**（编码 / 解码挂在
///   [`crate::PlanType::as_str`] / `parse` 与 [`crate::ReviewState::as_str`] / `parse`
///   上，与类型同址），库里读到表外取值（历史行、被外部改过的行）时由
///   [`crate::load_plan`] 的解码处产出，**带库里那一个原始串、不取默认值**。
///   照片：`tests/plan.rs` 的
///   `enum_codes_use_lowercase_underscore_and_reject_out_of_table_values`（经库读回）。
/// - [`PlanError::Persist`]（Task 15 新增）：`Tx` 的往返失败，`#[from]` 转出——
///   **`(id, major, minor)` 的二次写**（`save_plan` 的裸 `INSERT`）也走这一枚的
///   `Database` 形状（同 [`ContractError::Persist`] 的处置）。**不吞成上面两枚**：
///   数据库层的失败若被降级，调用方会以为「数据就是这样」，而真相是这次压根没读到。
///
/// # Task 13 时此处没有 `Persist` 一枚（留档）
///
/// Task 13 的 `plan` 表没有行级读写，故当时的抉择是「不为零产生方的错误变体留位」
/// ——那一枚随行级 I/O（Task 15）落地，本 task 加上的就是它。上面两枚「表外取值」
/// 同此（Task 13 的 `plan.rs` 预告「表外取值读回具体 `Err` 的处置随行级 I/O 落地」）。
///
/// # 照片
///
/// 前两枚的直接照片在 `tests/plan.rs`：`unlisted_review_pairs_are_rejected`
/// （逐条断言 `from` / `to` 原样带出）与
/// `a_strategic_plan_cannot_be_executed_without_user_approval`
/// （断言 `plan_type` / `state` 是那一对）；后三枚见上（经库）。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PlanError {
    /// 迁移对不在 §9.2 的审查迁移表内。`from` / `to` 原样带出——只说「这个迁移不合法」
    /// 而不给是哪一对，调用方无法判断是来源错还是目标错（形状取自
    /// [`IntentStatusError::Illegal`]）。
    #[error("迁移 {from:?} → {to:?} 不在 §9.2 的审查迁移表内")]
    IllegalReview { from: ReviewState, to: ReviewState },

    /// `STRATEGIC` 的 Plan 未经 `USER_APPROVED` 不得执行（§229）。
    /// `plan_type` / `state` 是**当时那一对**。
    #[error("Strategic Plan 未经 USER_APPROVED 不得执行（§229）：plan_type = {plan_type:?}、review_state = {state:?}")]
    UserApprovalRequired {
        plan_type: PlanType,
        state: ReviewState,
    },

    /// `plan.type` 列是表外取值。`value` 是**库里那一个**原始取值（Task 15）。
    #[error("表外的 plan type 取值: {value}")]
    UnknownType { value: String },

    /// `plan.review_state` 列是表外取值。`value` 同上（Task 15）。
    #[error("表外的 plan review_state 取值: {value}")]
    UnknownReview { value: String },

    /// `Tx` 的往返失败（含 `(id, major, minor)` 撞主键）。Task 15 随行级 I/O 新增。
    #[error(transparent)]
    Persist(#[from] PersistError),
}

/// Decision 的落库 / 读回与 §332 恢复链的错误（Task 15）。
///
/// # 十枚各有真实产生方，一条不漏
///
/// - [`DecisionError::InvalidBlocking`]：`decision.blocking` 列的表外取值（不是 0/1）
///   ——由 [`crate::load_decision`] 的解码处产出，**带原始数值、不取默认值**
///   （枚举 / 布尔列纪律的读回侧）。
/// - [`DecisionError::UnknownDecision`]：`apply_result` 指向的 `decision_id` 不存在
///   ——**不静默创建**（同 [`IntentError::UnknownIntent`] 的处置）；`id` 是入参那一个。
/// - [`DecisionError::UnknownOption`]：`option_index` 越界（「指向某个
///   `DecisionOption`」指向不存在的项）——不钳到某一项；`decision_id` 与 `index`
///   原样带出。
/// - [`DecisionError::UnknownContract`]：`ContractUpdate` 的基准版本不存在——
///   §226 的「新版本」要以一个真实基准起算，没有基准就不静默新建。
/// - [`DecisionError::UnknownRequirement`]：`RequirementUpdate` 指名的 id 不在基准
///   Contract 里——同 Constraint Validator 悬空键的 fail-closed 处置（判不了走 `Err`，
///   不静默添加一个 requirement）。
/// - [`DecisionError::UnknownPlan`]：`PlanUpdate` 的基准版本不存在（同上，不静默新建）。
/// - [`DecisionError::Contract`] / [`DecisionError::Plan`] / [`DecisionError::Intent`]：
///   §332 恢复链三步各自的子错误，`#[from]` 转出（同 `SpecError::Intent` 对
///   `IntentError` 的组合写路径形状）——调用方按子系统分辨失败来源，不吞成一枚。
/// - [`DecisionError::Persist`]：`decision` / `decision_result` 两张表自己的往返失败
///   （含 `decision_result` 主键撞键——一个决策恰一个回答），`#[from]` 转出。
///
/// **六枚「表外 / 指向不存在的」是分开的，不合并成一枚**：合并之后调用方不再能
/// 分辨该修哪一处（同 [`IntentError`] 的两枚表外取值与 P3D 的 `UnknownModel` 的分辨
/// 原则）。**红条件**各见 `tests/decision.rs` 的对应用例。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DecisionError {
    /// `decision.blocking` 列不是 0/1。`value` 是**库里那一个**原始数值。
    #[error("表外的 decision blocking 取值: {value}")]
    InvalidBlocking { value: i64 },

    /// `apply_result` 指向的 Decision 不存在。`id` 是**入参那一个**的里子串。
    #[error("decision 不存在: {id}")]
    UnknownDecision { id: String },

    /// `option_index` 越界（指向的 DecisionOption 不存在）。两个数原样带出。
    #[error("decision {decision_id} 没有第 {index} 项选项")]
    UnknownOption { decision_id: String, index: usize },

    /// `ContractUpdate` 的基准版本不存在。`id` / `version` 是**入参那一对**。
    #[error("contract 不存在: {id} v{version}")]
    UnknownContract { id: String, version: u32 },

    /// `RequirementUpdate` 指名的 requirement 不在基准 Contract 里。
    #[error("contract {contract_id} v{version} 里没有 requirement: {id}")]
    UnknownRequirement {
        contract_id: String,
        version: u32,
        id: String,
    },

    /// `PlanUpdate` 的基准版本不存在。
    #[error("plan 不存在: {id} v{major}.{minor}")]
    UnknownPlan { id: String, major: u32, minor: u32 },

    /// 恢复链里 Contract 侧的失败（`load_contract` / `save_contract`）。
    #[error(transparent)]
    Contract(#[from] ContractError),

    /// 恢复链里 Plan 侧的失败（`load_plan` / `save_plan`）。
    #[error(transparent)]
    Plan(#[from] PlanError),

    /// 恢复链里 Intent 回迁的失败（[`crate::apply_intent_transition`]）。
    #[error(transparent)]
    Intent(#[from] IntentError),

    /// `Tx` 的往返失败（含 `decision_result` 撞主键）。
    #[error(transparent)]
    Persist(#[from] PersistError),
}
