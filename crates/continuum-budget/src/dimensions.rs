//! §333 的五个量纲：名字的闭集（[`DimensionKind`]）与纯数据载体（[`Dimensions`]）。
//!
//! # 编码 / 解码与类型同址
//!
//! [`DimensionKind::as_str`] / [`DimensionKind::parse`] 在本文件（`budget_ledger.
//! dimension` 列是封闭枚举列，小写加下划线；表外取值读回具体 `Err`——落点在
//! [`crate::ledger::dangling_reservations`] 的解码处）。**本 crate 别处不自建
//! 第二份编码。**

/// §333 的五个量纲的名字（**闭集五枚**）。
///
/// 编码 / 解码与类型同址（见 [`DimensionKind::as_str`] / [`DimensionKind::parse`]）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DimensionKind {
    Money,
    WallTime,
    Token,
    GpuTime,
    NetworkTransfer,
}

impl DimensionKind {
    /// 落库编码：**小写加下划线**（本仓枚举列纪律）。**无通配臂**——加变体即编译不过
    /// （逼着这一行跟着变）。
    pub fn as_str(&self) -> &'static str {
        match self {
            DimensionKind::Money => "money",
            DimensionKind::WallTime => "wall_time",
            DimensionKind::Token => "token",
            DimensionKind::GpuTime => "gpu_time",
            DimensionKind::NetworkTransfer => "network_transfer",
        }
    }

    /// 落库解码。**认不出的串返回 `None`**（调用点转具体 `Err`，**不取默认值**——
    /// 把认不出的串猜成某一维，正是枚举列纪律要拦的）。**带通配臂**（表外取值是
    /// 常态，`None` 是它的正常返回，不是「不可能发生」）。
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "money" => Some(DimensionKind::Money),
            "wall_time" => Some(DimensionKind::WallTime),
            "token" => Some(DimensionKind::Token),
            "gpu_time" => Some(DimensionKind::GpuTime),
            "network_transfer" => Some(DimensionKind::NetworkTransfer),
            _ => None,
        }
    }
}

/// 五维的**固定顺序**（与枚举声明序一致）——crate 内算法逐维遍历、dangling 的
/// 排序与 settle 的「字段序第一处违反」共用它。
pub(crate) const ALL: [DimensionKind; 5] = [
    DimensionKind::Money,
    DimensionKind::WallTime,
    DimensionKind::Token,
    DimensionKind::GpuTime,
    DimensionKind::NetworkTransfer,
];

/// 某一维在 [`ALL`] 里的位置（内部与测试的排序键）。
pub(crate) fn rank(kind: DimensionKind) -> usize {
    ALL.iter()
        .position(|k| *k == kind)
        .expect("ALL 是五维的穷尽列表，position 不会失败")
}

/// §333 的五个量纲。**单位与取值域规范未给**（§16 第 19 条）——本层**不解释、
/// 不换算、不做跨量纲比较**。
///
/// # 五个字段是 `pub` 的（判据，同 D 的 `BudgetView`）
///
/// 本类型**没有需要靠私有性守的不变量**（五个 `Option<i64>`、无取值域、无跨维
/// 关系），故 `pub` 不削弱任何保证；且 Task 19 的「编译器强制穷尽的解构」要在
/// **另一个 crate 的测试**里读这五个字段——字段私有则那条断言写不出来，而它正是
/// 裁定 §12.6.1 的全部防线。**三个角色的保证不在本类型上，在三个 newtype 上**
/// （各自私有、无公开构造函数；`pub` 的是 `Dimensions` 的字段，不是它们）。
///
/// # `None` 与 `Some(0)` 不是同一件事
///
/// `None` = 该量纲**不构成约束**；`Some(0)` = **额度为零**——混同会让「不约束」
/// 变成「一分钱都不能花」（同 `crates/continuum-model-registry/src/budget.rs:33-36`
/// 的 `BudgetView` 文档）。本类型在这两者之间不做任何折合。
///
/// # 为什么只派生 `Debug`
///
/// 纯数据载体，**没有语义能力可加**（同 `BudgetView` 的处置）：不派生
/// `Clone` / `Copy` / `PartialEq`——今天没有消费方。**`Debug` 有当场消费方**：
/// [`crate::SettleOutcome`] 的 `Debug` 输出与各断言的诊断消息（`Debug` 与 `Clone`
/// 不是语义能力，前者是断言失败时必须打得出来——同 `IntentId` 的判据）。
#[derive(Debug)]
pub struct Dimensions {
    pub money: Option<i64>,
    pub wall_time: Option<i64>,
    pub token: Option<i64>,
    pub gpu_time: Option<i64>,
    pub network_transfer: Option<i64>,
}

impl Dimensions {
    /// 按 [`ALL`] 的固定顺序读成数组（crate 内算法用）。
    pub(crate) fn to_values(&self) -> [Option<i64>; 5] {
        [
            self.money,
            self.wall_time,
            self.token,
            self.gpu_time,
            self.network_transfer,
        ]
    }

    /// [`Dimensions::to_values`] 的逆（内部构造计算结果用）。
    pub(crate) fn from_values(values: [Option<i64>; 5]) -> Dimensions {
        Dimensions {
            money: values[0],
            wall_time: values[1],
            token: values[2],
            gpu_time: values[3],
            network_transfer: values[4],
        }
    }

    /// 第一处（按 [`ALL`] 顺序）负值所在的维与值；没有负值给 `None`。
    ///
    /// §12.2.1 的口径「一个本该非负的量纲取了负值」的判据——四个吃 `Dimensions`
    /// 的入口（`allocate` / `reserve` / `settle` / `check`）外加本 task 落地的
    /// **根入账函数**（[`crate::record_root`]：它同刻落地、同样吃一份 `Dimensions`，
    /// **同一口径、不做豁免**）共用它。
    pub(crate) fn first_negative(&self) -> Option<(DimensionKind, i64)> {
        let values = self.to_values();
        for (i, kind) in ALL.iter().enumerate() {
            if let Some(v) = values[i] {
                if v < 0 {
                    return Some((*kind, v));
                }
            }
        }
        None
    }
}
