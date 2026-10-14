//! 三个角色三个类型（设计 §12.1，照 D §6.3 的同一判据：**上限与余量不能共用
//! 一个类型**）。
//!
//! | 类型 | 角色 | 谁写 |
//! |---|---|---|
//! | [`Allocation`] | 树上节点的**上限**（声明式约束） | 分配方（[`crate::record_root`] / [`crate::allocate`]） |
//! | [`Reservation`] | 一次在飞的**预扣** | [`crate::reserve`] |
//! | [`Remaining`] | **计算值**：`allocation − settled − reserved`（逐维） | 无（[`crate::remaining`] 现算） |
//!
//! # 三者的内部字段一律私有、无公开构造函数
//!
//! 只能经**分配方**、[`crate::reserve`]、[`crate::remaining`] 得到——故「把余量
//! 填进上限的位置」在**类型上**不可写（`let _: Allocation = remaining;` 编译不过），
//! 且 crate 外**造不出**任何一枚。**六份 trybuild 样例钉的就是这两件事**
//! （三反例 ＋ 三正控制，见 `tests/type_level.rs`）。
//!
//! # 「构造」与「读」要分清
//!
//! 三个角色各自公开一个读法 [`Allocation::dims`] / [`Reservation::dims`] /
//! [`Remaining::dims`]（返回 `&Dimensions`，**不是角色**——`Dimensions` 不是角色，
//! 读它不产生「余量当上限用」的可写位置），`Remaining` 另有五个逐维读者
//! （`money()` 一族，§12.6 的投影读法）。它们**不构成**第二条构造路径。

use crate::dimensions::Dimensions;
use crate::tree::BudgetOwner;

/// 树上节点的**上限**（声明式约束）——分配方写。
///
/// 里子**私有、无公开构造函数**：字段标 `pub(crate)`，**crate 外不可见**
/// （trybuild 的样例钉住这一点）；`pub` 的是 [`Dimensions`] 的五个字段——
/// 它不是角色，没有可伪造的语义。
pub struct Allocation(pub(crate) Dimensions);

impl Allocation {
    /// 五维的公开读法：返回 `&Dimensions`，**不是**把三个角色之间的类型边打通。
    pub fn dims(&self) -> &Dimensions {
        &self.0
    }
}

/// 一次在飞的**预扣**——[`crate::reserve`] 写。
///
/// # 里子是 `owner` ＋ `dims` 两项（一处申报）
///
/// 设计 §12.1 的速写是 `pub struct Reservation(Dimensions);`，而设计 §12.2 写死
/// `settle(reservation, actual)` **只有两个入参**——没有 owner 就写不了结算账
/// （账本行的 `owner` 列是结算的键），故预扣必须随身携带它的主人。私有性、
/// 无公开构造函数、「三类型不共用」这些保证一字不变。
///
/// # 不派生 `Clone` / `Copy`（刻意的）
///
/// 「一次在飞的预扣」被 [`crate::settle`] **move** 消费——同一枚 `Reservation`
/// 因此在类型上只能结算一次。这是类型层能拍的「不双重结算」保证，代价为零。
pub struct Reservation {
    pub(crate) owner: BudgetOwner,
    pub(crate) dims: Dimensions,
}

impl Reservation {
    /// 五维的公开读法（同 [`Allocation::dims`] 的判据）。
    pub fn dims(&self) -> &Dimensions {
        &self.dims
    }
}

/// **计算值**：`allocation − settled − reserved`（逐维）——[`crate::remaining`]
/// 现算，不是落库的列。
///
/// 里子私有、无公开构造函数（同 [`Allocation`]）。
pub struct Remaining(pub(crate) Dimensions);

impl Remaining {
    /// 五维的公开读法（Task 19 的穷尽解构与 §12.6 的恒等投影都要它）：
    /// 返回 `&Dimensions`。
    pub fn dims(&self) -> &Dimensions {
        &self.0
    }

    /// §12.6 的投影逐维读法（`BudgetView { money: r.money(), … }`）。
    pub fn money(&self) -> Option<i64> {
        self.0.money
    }

    /// §12.6 的投影逐维读法（同 [`Remaining::money`]）。
    pub fn wall_time(&self) -> Option<i64> {
        self.0.wall_time
    }

    /// §12.6 的投影逐维读法（同 [`Remaining::money`]）。
    pub fn token(&self) -> Option<i64> {
        self.0.token
    }

    /// §12.6 的投影逐维读法（同 [`Remaining::money`]）。
    pub fn gpu_time(&self) -> Option<i64> {
        self.0.gpu_time
    }

    /// §12.6 的投影逐维读法（同 [`Remaining::money`]）。
    pub fn network_transfer(&self) -> Option<i64> {
        self.0.network_transfer
    }
}
