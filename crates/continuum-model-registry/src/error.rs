//! 本 crate 的错误类型（设计 §2.4、§4.1、§5.4）。
//!
//! 三个错误各管一件事，**不并成一个**：
//! [`ProfileError`] 标「这个值根本不是合法取值」（构造期）；
//! [`LifecycleError`] 标「这次生命周期操作不合法」；
//! [`RoutingError`] 标「这次路由请求不合法」。判据是**产生方不同**——
//! 把「值非法」与「操作非法」并成一枚，调用方就分不出该退回去修数据还是修动作。

use continuum_core::model::ModelId;
use continuum_persist::PersistError;

use crate::lifecycle::LifecycleState;

/// 画像侧取值类型的构造错误（设计 §2.4）。
///
/// **与 `LifecycleError` / `RoutingError` 分开**：它标的是「这个值根本不是合法取值」，
/// 不是「这次操作不合法」。
///
/// **它不进 `RoutingError`**：`rank` 收到的是构造好的值，构造失败在构造期就被拒
/// （设计 §2.4、§5.4）。
///
/// 设计 §2.4 给了三枚变体（`NotFinite` / `OutOfRange` / `BadTimeRange`），三枚都在这里。
/// 第三枚 `BadTimeRange { start, end }` 的产生方是 `SkillObservation` 的构造，
/// **Task 1 只定义了前两枚**——「没有产生方的变体不先铺开」，Task 2 随产生方一起落地。
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ProfileError {
    /// 不是有限实数（NaN 或 ±∞）。
    ///
    /// 两处产生方，判据同一条：`SkillScore` 的取值域在规范里**未定义**
    /// （§23 的示例是 9.2，§24 既未给范围也未给方向），故那边除了「必须是个有限实数」
    /// 之外无界可判；`Ratio` 的 `NaN` / `±∞` 也归这一枚——**不是有限实数**与
    /// **落在区间之外**是两回事，前者不是后者的一种。
    #[error("取值不是有限实数（NaN 或 ±∞）")]
    NotFinite,

    /// 有限、但落在 `[0,1]` 之外（`Ratio`）。
    ///
    /// `value` 原样带回入参，且**总是个有意义的数**：非有限的入参走
    /// [`ProfileError::NotFinite`]，不会到这里来。
    #[error("取值 {value} 落在 [0,1] 之外")]
    OutOfRange { value: f64 },

    /// 该维度的观测在时间窗上不自洽（`end < start`）。
    ///
    /// 两个端点值都原样带回：只说「反序」而不给出是哪两个端点，调用方无法判断
    /// 是哪一次观测坏在哪一格。
    ///
    /// **`end == start` 不归这一枚**：`time_range` 是**闭区间**，退化区间是一个点，
    /// 自洽。故这条的守卫是 `end < start`，不是 `end <= start`。
    #[error("时间窗反序：start={start} 晚于 end={end}")]
    BadTimeRange { start: i64, end: i64 },
}

/// 生命周期侧的错误（设计 §4.1）。
///
/// **不叫 `RegistryError`**——C 的设计在 `continuum-provider` 里已有一个同名不同物的
/// `RegistryError`（`NotFound` / `Duplicate`，是适配器注册表的错误）。两件事一个名字会让
/// 调用方与后来者混淆，与「同一件事两个词汇表」是同一种病灶的两面。
///
/// # 四枚变体的到位情况
///
/// 设计 §4.1 列了四枚（`Illegal` / `ProfileBeforeVerified` / `UnknownModel` / `Persist`）。
/// Task 4 落 `Illegal`（内存版 `transition` 的失败值）；Task 6 随落库读写补上
/// `UnknownModel { id }` 与 `Persist(#[from] PersistError)`（`persist::transition_in_tx` 的两个
/// 失败来源：没有登记项可改、以及读写出错）。
/// **`ProfileBeforeVerified` 仍未落地**：它的产生方是 `save_profile`（Task 7）——
/// **没有产生方的变体不先铺开**，一枚永不出现的变体会让 `match` 的穷尽臂说谎。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LifecycleError {
    /// 迁移对不在 §4.1 的表里。`from` / `to` 原样带出（形状取自 `continuum-graph` 的同名变体
    /// `crates/continuum-graph/src/state.rs:8-10`）。
    ///
    /// **两端都带出**：只说「这个迁移不合法」而不给是哪一对，调用方无法判断是来源错还是目标错。
    #[error("迁移 {from:?} → {to:?} 不在 §4.1 的迁移表内")]
    Illegal {
        from: LifecycleState,
        to: LifecycleState,
    },

    /// 登记项不存在，没有状态可迁移。`id` 原样带出。
    ///
    /// **产生方只有落库版**（`persist::transition_in_tx`）：内存版 `transition` 收的是
    /// 两个状态，没有「哪个模型」这一维。**不静默创建**——登记是 `register_model` 的活
    /// （设计 §3.2）；把一次笔误的 id 变成一行新登记项，会让「未见过的模型」与「打错字的
    /// 模型」在库里长得一样。
    ///
    /// **读路径不用这一枚**：`load_lifecycle` 对未登记的 id 返回 `Ok(None)`——
    /// 读一个不存在的模型是「没有」，不是「出错」。
    #[error("登记项 {id:?} 不存在，没有生命周期状态可迁移")]
    UnknownModel { id: ModelId },

    /// 落库读写出错（`#[from]`，供 `?` 直接升格）。
    ///
    /// 与 [`LifecycleError`] 另三枚的分界：那三枚说的是「这次生命周期操作不合法」，
    /// 这一枚说的是「库这一次没读成或没写成」——调用方对两者的处置不同
    /// （修动作／报基础设施故障），故不并成一枚（同 `ProfileError` 与 `RoutingError` 分开的理由）。
    #[error("生命周期落库读写出错: {0}")]
    Persist(#[from] PersistError),
}

/// 路由侧的错误（设计 §5.3）。
///
/// # 只有一枚变体：其余由 Task 10 增补
///
/// 本 task 只落 [`RoutingError::NotRoutable`]——它是可路由闸门
/// （[`crate::lifecycle::RoutableModel::try_new`]）的失败值，产生方在本 task 之内。
/// 设计 §5.3 的其余变体（需求侧与候选集侧）随各自的产生方在 Task 10 / Task 11 落地。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RoutingError {
    /// 该状态**不可进入自动路由路径**，`state` 原样带出**传入的那个十态值**。
    ///
    /// 四个产生方（设计 §4.2）：§249 禁的三态 `unprofiled` / `quarantined` / `disabled`，
    /// 以及裁决额外挡下的第四态 `stale`（`docs/superpowers/specs/2026-10-05-p3bcdf-set-decisions.md`
    /// 第一节第 4 条：§249 的三态是**下限不是上限**，漂移中的模型正在退出服役，放行即 fail-open）。
    ///
    /// **带的是 [`LifecycleState`]（十态），不是收窄后的 `RoutableState`**：被拒的那个状态
    /// 恰恰不在 `RoutableState` 里，若这里收窄，错误值就表达不出「是哪一个被拒了」。
    #[error("状态 {state:?} 不可进入自动路由路径（§249 的三态 ＋ 已裁的 stale）")]
    NotRoutable { state: LifecycleState },
}
