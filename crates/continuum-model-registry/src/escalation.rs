//! §251 的**升级阶梯的数据形状**（设计 `docs/superpowers/specs/2026-10-05-p3d-model-registry-router-design.md` §7.1）。
//!
//! 本模块只建**阶梯本身**：五档的 [`EscalationStep`] 与「下一档」这个纯函数 [`next_step`]。
//! **触发不在本层**——下面逐条写明本层**不建**什么、以及理由，免得被读成漏项（设计 §7.1）。
//!
//! # 本层明确不建
//!
//! - **失败检测与升级触发**：它要读一次执行的失败（§309 的 `FailureClass`），那是**子项目 G**
//!   （模型调用路径，含模型侧失败分类）的活；也**不记在子项目 F 名下**——F 是工具调用路径，
//!   与模型调用失败不是同一条路径（设计 §1.2 末段、§7.1）。本层若自己判失败，就会出现
//!   「同一个失败两个裁决者」（驱动按 §309 判、本层又判一次），正是 ENG-005 被否掉的
//!   「多层各自裁决」的同一种病灶。
//! - **§251 的前提检查「仍满足 Contract 和 Budget」**：按 §110，判预算是**语义层的
//!   Budget Validator**，不是资源层。本层**不读预算**，故这条前提在本层没有输入（设计 §7.1）。
//! - **§86 的降级调度**：§86 的降级例（「复杂规划 → Tier 2 High，随后 400 个机械文件检查 →
//!   Tier 1 Low」，`docs/spec/02-positioning.md:847-867`）是**任务级**的模型再选择——
//!   用一个新的 `TaskSkillRequirement` **再调一次 `rank`**。本模块**不为它建任何新机构**：
//!   降级走「同一入口的第二次调用」，**不经过 [`next_step`]**（设计 §7.1 末段）。
//!
//! # 为什么本模块没有 `EscalationLadder`
//!
//! 设计 §7.1 已裁定：**阶梯本身（`EscalationLadder`）本阶段不建**。三条理由合起来是「**不可构造**」：
//! **零产生方**（本层没有任何东西会造出一个阶梯、也没有任何东西往里面塞步骤）、
//! **零消费方**（[`next_step`] 收发的都是裸 [`EscalationStep`]）、
//! **字段私有且无构造函数、无访问器**——连测试都造不出一个实例。
//! 这与本仓先例同一判据：`RoutingError::Persist` 正因**零产生方**被删。
//! **建它的触发条件**（设计 §7.1 写死，免得后来者以为漏了）：**出现第一个消费者时**才建，
//! 届时**连同一个读口（访问器）一起加**。
//!
//! 「阶梯是有序的」这件事**不因此丢失**：顺序由 [`next_step`] 的一串去向**显式**给出，
//! 并由 `tests/escalation.rs` 的 `the_ladder_is_ordered_as_251_states` 钉住。

/// §251 的阶梯五档，**逐档照录**（`docs/spec/05-normative.md:922-944`）。
///
/// `Tier2High` 是 §251 的原词：按 §17／§18「能力档位与推理强度分离」，它对应的是
/// 「Tier2 ＋ 高推理强度」这个**组合**，**不是第六档能力**。
///
/// # 不另建 `Tier` / `ReasoningEffort` 两个枚举
///
/// 本层没有消费者需要按轴拆分；拆了就是给两个没有产生方的类型建形状（设计 §7.1 的
/// 「不预先发明」）。`Tier2High` 这一档的存在**不**要求把两个轴显式化。
///
/// # 没有落库编码，故没有 `as_str`
///
/// 设计 `:586-587` 具名本类型「不进任何列，**没有落库编码，也就没有字面量可给**」，
/// 与 `RoutableState` / `FamilyRelation` / `RoutingReason` / 四枚错误类型同类。
/// Task 11 对 `FamilyRelation` 正是照此办理（未写 `as_str`）。**不发明一个没有列要装的编码函数。**
///
/// # §86 的 `Tier 1 Low` **不是**第六个成员
///
/// §86 的降级例写「Tier 1 Low」（`docs/spec/02-positioning.md:847-867`），而 §251 阶梯五档里没有它：
/// 「Low」是 §18 的**推理强度**取值，不是 §17 的 Tier。本设计**不为它加第六个成员**——
/// 加一个不在阶梯里的成员会让「照录 §251」这句话变假（设计 §7.1、§11 第 21 条）。
/// 这处**规范自身的不一致**记在 §11 第 21 条，**收件人是规范维护者**。
/// 本层的照片是 `tests/compile_fail/tier_one_low_is_not_a_step.rs`：`Tier1Low` **写不出来**。
///
/// # 派生集（逐项按「有没有当天用得上的消费方」判，本 crate 的常设规则）
///
/// | 派生 | 当天消费方 |
/// |---|---|
/// | `Debug` | `tests/escalation.rs` 的 `assert_eq!`——比对 `Option<EscalationStep>` / `Vec<EscalationStep>` 时要打印两边的值 |
/// | `PartialEq` | 同上，它就是 `assert_eq!` 的比较本身 |
/// | `Copy` | `the_ladder_is_ordered_as_251_states` 的走链循环里，同一个值既进 `Vec` 又喂给 [`next_step`]（不 `Copy` 则那一行是 E0382） |
/// | `Clone` | **没有直接消费方**；留着的真因是它是 `Copy` 的编译期前提（`Copy: Clone`），不是「将来可能有人用」 |
/// | `Eq` | **没有**——`assert_eq!` 只要 `Debug` ＋ `PartialEq`，故**不派生**（判据与 `FamilyRelation` / `BudgetView` 同：没有消费方就不加） |
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EscalationStep {
    /// 第一档：能力最低、最省的模型（§251 的 `Tier1`）。
    Tier1,
    /// 第二档（§251 的 `Tier2`）。
    Tier2,
    /// 「Tier2 ＋ 高推理强度」这个**组合**（§251 的原词 `Tier2 High`），不是第六档能力。
    Tier2High,
    /// 换到另一个 model family（§251 的 `Cross-family`）。
    CrossFamily,
    /// 专用模型（§251 的 `Specialized Model`）。
    Specialized,
}

/// 「下一档」。**纯函数**——本层建阶梯的**有序步骤**，触发不在本层（设计 §7.1）。
///
/// # 顺序**显式**写出，不靠变体的声明序
///
/// `match` 穷尽且**无 `..`**：加第六档时本函数**编译不过**（E0004 non-exhaustive patterns），
/// 新增的档位不会悄悄漏掉去向。顺序由这一串 `Some(..)` 给出，**不来自** [`EscalationStep`]
/// 的声明序——声明序是一种没说出口的规格（同 [`crate::router`] 里 `family_rank` 的理由）。
///
/// # 末档返回 `None`
///
/// [`EscalationStep::Specialized`] 之后没有档，故是 `None`——不是自环
/// `Some(Specialized)`，也不是回卷到 `Tier1`。「回卷」是**降级**，走「用一个新的
/// `TaskSkillRequirement` 再调一次 `rank`」，**不经过本函数**（本模块头）。
pub fn next_step(current: EscalationStep) -> Option<EscalationStep> {
    match current {
        EscalationStep::Tier1 => Some(EscalationStep::Tier2),
        EscalationStep::Tier2 => Some(EscalationStep::Tier2High),
        EscalationStep::Tier2High => Some(EscalationStep::CrossFamily),
        EscalationStep::CrossFamily => Some(EscalationStep::Specialized),
        EscalationStep::Specialized => None,
    }
}
