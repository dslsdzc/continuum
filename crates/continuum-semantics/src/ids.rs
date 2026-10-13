//! 本层的标识类型：`IntentId` / `PlanId` / `ContractId` / `RequirementId`
//! （计划「文件结构」一节把前三个与 `RequirementId` 归在 `src/ids.rs`，
//! `docs/superpowers/plans/2026-10-06-p4-semantic-layer.md:300`）。
//!
//! # 四个都是不透明串，且**不共享一个类型**
//!
//! §222 只给了 `Intent.id` 这个名字，没给取值域；`PlanId` / `ContractId` / `RequirementId`
//! 同理（§228 / §224 都只有字段名）。故四者都是**不透明串**：本层保存事实，不发明取值域。
//!
//! **为什么不合并成一个 `Id(String)`**：那样 `intent_id` 与 `contract_id` 会在类型上
//! 可以互换——把一枚 `PlanId` 传进要 `IntentId` 的位置会编得过。四枚类型各自独立
//! （`RequirementId` 的处置与理由逐字相同），这一处交换就成了编译期错误。代价是每个
//! 类型各写一遍那两个方法，而那是**一分不值的重复**（没有不变量、没有编码差异），
//! 换来的是**一处本层唯一能挡的错**。
//!
//! # 与 `continuum_canonical::EntityId` 的关系
//!
//! 本文件**不复用** `EntityId`：它指的是 §272 的**候选实体**，本文件的四个指的是
//! **本层自己的记录键**。两者今天都是不透明串，但它们是两件事（一个从候选集来、
//! 一个从库里来）——合并会让「拿一个实体去当 intent 主键」在类型上可写。
//!
//! # 没有 `PartialEq` / `Eq`（同 `EntityId`）
//!
//! 设计没有给「两个 id 是同一个」以外的任何判据，而本层的断言都经 [`IntentId::as_str`]
//! 比较——调用点看得到自己在比什么。加 `PartialEq` 是一枚**今天没有用例**的表达力。
//!
//! **`Debug` 与 `Clone` 不是语义能力**：前者是断言失败时必须打得出来，后者是它们要能
//! 装进 `Intent` 并随值传递（设计 §1.2 的「以值进入」）。
//!
//! # 「不改写、不裁剪」「无校验」的照片
//!
//! `IntentId` / `PlanId` / `ContractId` 三个都写了这两句绝对措辞，**各自直接**的照片在
//! `tests/intent.rs` 的 `opaque_strings_are_not_rewritten_trimmed_or_rejected`：
//! 带空白／特殊字符／非 ASCII 的串逐字节往返、空串也被接受（此前只有 `PlanId` 有一条
//! 直接照片，另两个靠普通串间接经过）。[`RequirementId`] 的同款照片在 `tests/contract.rs`
//! 的 `requirement_ids_are_not_rewritten_trimmed_or_rejected`（Task 11 追加）。

/// §222 的 `Intent.id`。落库是 `intent` 表的主键。
#[derive(Debug, Clone)]
pub struct IntentId(String);

impl IntentId {
    /// 取里子串。**不改写、不裁剪**。
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 从任意串构造。**无校验**——本类型没有「合法取值」这个概念
    /// （§222 只给了字段名）。
    ///
    /// 名字用 `from_str` 而没有实现 `std::str::FromStr`：后者要求一个 `Err` 关联类型，
    /// 而本类型**不失败**（同 `EntityId::from_str`）。
    pub fn from_str(s: &str) -> Self {
        IntentId(s.to_owned())
    }
}

/// §228 的 `Plan.id`（本 task 只登记它，因为 [`crate::Intent::active_plan_id`] 要命名它；
/// `Plan` 本体归 Task 13）。
#[derive(Debug, Clone)]
pub struct PlanId(String);

impl PlanId {
    /// 取里子串。**不改写、不裁剪**。
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 从任意串构造。**无校验**（同 [`IntentId::from_str`]）。
    pub fn from_str(s: &str) -> Self {
        PlanId(s.to_owned())
    }
}

/// §224 的 `TaskContract.id`。**本层定义的真类型**（设计 §13.2：执行层的 `ContractIdRef`
/// 与它是同概念两类型，权威在本层；本层**不引用** `ContractIdRef`，故对 `continuum-graph`
/// 的依赖边为零）。
///
/// Task 8 只登记它（因为 [`crate::ContractRef`] 要命名它）；`TaskContract` 本体由 Task 11
/// 落在 `src/contract.rs`。
#[derive(Debug, Clone)]
pub struct ContractId(String);

impl ContractId {
    /// 取里子串。**不改写、不裁剪**。
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 从任意串构造。**无校验**（同 [`IntentId::from_str`]）。
    pub fn from_str(s: &str) -> Self {
        ContractId(s.to_owned())
    }
}

/// §224 的 `Requirement.id`（Task 11）。
///
/// 处置与前三枚**逐字相同**（不透明串、无校验、不改写不裁剪、无 `PartialEq`）：
/// 它出现在 `contract_requirement` 表的主键第三段与 Task 12 的 diff
/// （`DiffEntry.requirement_id`）里，是**本层自己的记录键**。
///
/// **「不改写、不裁剪」的照片**：`tests/contract.rs` 的
/// `requirement_ids_are_not_rewritten_trimmed_or_rejected`。
#[derive(Debug, Clone)]
pub struct RequirementId(String);

impl RequirementId {
    /// 取里子串。**不改写、不裁剪**。
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 从任意串构造。**无校验**（同 [`IntentId::from_str`]）。
    pub fn from_str(s: &str) -> Self {
        RequirementId(s.to_owned())
    }
}
