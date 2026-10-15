//! 探索预算（§334 的落点、§12.5）：`check_exploration` 只判「**能不能自动执行**」。
//!
//! # §334 与 §7 的对应（§12.5 的表，逐行）
//!
//! §334：「寻找替代方案也属于预算。默认可自动执行：低成本 / 可逆 / 短时间 /
//! 无外部副作用。高成本探索必须进入 Decision Gate。」§7 把前两个展开成示例数值
//! （`wall_time ≤ 60s`、`money_cost = 0`），第三个写成 `external_side_effect = none`
//! （**不是数值**——是一个种类），另有两行（`network_transfer <= threshold`、
//! `compute_cost <= threshold`）**原文只写 `threshold`、没有数值**（§16 第 21 条）。
//!
//! 四个**阈值**判据（§15.2 第 10 条的「四个阈值」）落在这四个量纲上：
//! `wall_time` / `money` / `network_transfer` / `gpu_time`（`compute_cost` 的
//! §333 名字）。**两个判据不在 `Dimensions` 里**（`has_external_effect`——它是
//! 一个种类、不是一个量，判定属边界层；`reversible`——§334 的「可逆」未给量纲）：
//! 它们是**必填入参**。**`token` 维在两个判据集里都没有对应**（§12.5 的表格
//! 逐行对照里没有 `token` 行）——本层不发明它的判据。
//!
//! # 「策略真的收紧了」在本层不可观察（§15.2 第 10 条）
//!
//! 四个阈值可由策略收紧，而**策略的来源（谁配、配在哪）规范未给**——
//! 故本层能拍的只有「**给定阈值时的判定**」：把阈值当**入参**收下
//! （[`ExplorationThresholds`]），不发明「策略怎么收紧」的机制。
//!
//! # 两处申报（brief/设计未给、两处不能同真时选的一侧）
//!
//! 1. **判定顺序**：`has_external_effect` → `reversible` → 逐维数值。
//!    设计未给多判据同时触发时的选择规则；本实现把**两枚 `None` 格**（「不是
//!    量、是种类或性质」的问题）排在数值格之前——外部副作用与不可逆是
//!    「这件事本身不该自动做」，比「花得有点多」更靠前。数值格之间取
//!    **字段序第一处**（同 [`crate::SettleOutcome::Over`] 的处置）。
//! 2. **负值不单独处置**：本函数没有 `Err` 位置（签名是设计的写死形状
//!    `-> ExplorationVerdict`），且设计未给本函数上的负值处置——本实现**按字面
//!    比较**（负值 ≤ 任何非负阈值 ⇒ 不门控）。**申报**：§12.2.1 的四入口表
//!    （`allocate` / `reserve` / `settle` / `check`）没有本函数，「负值不是
//!    『不合理』而是『不可能』」这句话在它身上没有承载位置。**收件人：复审**
//!    （若判「探索预算处也要同侧」——那要改签名或发明一条 `None` 判据，
//!    属另一轮的决定）。

use crate::dimensions::{ALL, DimensionKind, Dimensions};

/// §334 的四个阈值（§7 的示例数值；ENG-005 §三.1 采前两个为**规范级默认**）。
///
/// # 四个字段是 `pub` 的（判据同 [`crate::Dimensions`]）
///
/// 本类型**没有需要靠私有性守的不变量**（四个 `i64`、无取值域、无跨字段关系），
/// 故 `pub` 不削弱任何保证。
///
/// # 为什么**没有** `Default`（`tests/type_level.rs` 的 trybuild 样例钉住）
///
/// §7 末两行的 `network_transfer <= threshold` 与 `compute_cost <= threshold`
/// **原文只写 `threshold`、没有数值**（§16 第 21 条）——故它们是**必填入参**、
/// 本层**不发明这两个数**。一条无参的 `Default`（或任何全默认构造）等于替调用方
/// 发明了这两个数；「前两个用规范级默认」的正路是 [`ExplorationThresholds::spec_defaults`]
/// （后两个仍必须由调用方以值给出）。
///
/// **不派生任何东西**（同 D 的 `BudgetView` 的处置）：今天没有当场消费方。
pub struct ExplorationThresholds {
    /// `wall_time` 的阈值。**规范级默认 60**（§7 的示例数值；ENG-005 采为
    /// 规范级默认——「它是规范里**已有**的数字，不是新发明的」）。
    pub wall_time: i64,
    /// `money` 的阈值。**规范级默认 0**（§7 的 `money_cost = 0`——「= 0」
    /// 就是 `≤ 0`）。
    pub money: i64,
    /// `network_transfer` 的阈值。**§7 无数值**——必填，本层不发明。
    pub network_transfer: i64,
    /// `gpu_time`（§7 的 `compute_cost`）的阈值。**§7 无数值**——必填，本层不发明。
    pub gpu_time: i64,
}

impl ExplorationThresholds {
    /// 规范级默认的两项（`wall_time = 60`、`money = 0`）＋**必填的两项**
    /// （`network_transfer` / `gpu_time`，§7 只写 `threshold`、没有数值）。
    ///
    /// # 两个默认数的来历
    ///
    /// §7 的示例数值（`docs/spec/01-concepts.md:362-402`）；ENG-005 §三.1 末行
    /// 采纳它们为**规范级默认**并注明「它是规范里**已有**的数字，不是新发明的」。
    /// 策略可在两个默认级之上**收紧**（改动经字段与别的构造路径；
    /// 「策略真的收紧了」在本层不可观察——§15.2 第 10 条）。
    pub fn spec_defaults(network_transfer: i64, gpu_time: i64) -> Self {
        ExplorationThresholds {
            wall_time: 60,
            money: 0,
            network_transfer,
            gpu_time,
        }
    }
}

/// §334 的裁决。
///
/// `dimension: None` = 因「**不可逆 / 有外部副作用**」而门控——两者**共用一个
/// `None`** 是设计写死的（它们都不是「量」：外部副作用是一个**种类**，可逆
/// 是一个**性质**）。
///
/// # 只派生 `Debug`（同 [`crate::SettleOutcome`] 的判据）
///
/// 今天没有 `PartialEq` 的消费方（测试逐变体 `match` 断言）；`Debug` 是断言
/// 失败时必须打得出来的那个。
#[derive(Debug)]
pub enum ExplorationVerdict {
    /// 默认可自动执行（§334 的四个判据**全部**在阈值内——含两个必填 `bool`）。
    AutoAllowed,
    /// 进入 Decision Gate：**哪一维**超了（`Some`），或因「不可逆 /
    /// 有外部副作用」而门控（`None`）。
    DecisionRequired { dimension: Option<DimensionKind> },
}

/// §334 的判定：给**一次**动作的属性，回答「能不能自动执行」。
///
/// # 判定（顺序见模块文档的申报 1）
///
/// 1. `has_external_effect = true` ⇒ [`ExplorationVerdict::DecisionRequired`]
///    `{ dimension: None }`；
/// 2. `reversible = false` ⇒ 同上；
/// 3. 逐维（五维的固定顺序——`ALL`，即枚举声明序）：该维 `Some(v)` 且 `v > 阈值` ⇒
///    `DecisionRequired { dimension: Some(该维) }`（第一处即报）；
///    该维 `None`（不构成约束）或 `token`（无判据）跳过；
/// 4. 否则 `AutoAllowed`。
///
/// # 比较是 `≤`（包含）
///
/// §7 的 `wall_time <= 60s` 是包含的：恰等于阈值**放行**（照片取恰等的点——
/// `tests/exploration.rs` 的八条）。
pub fn check_exploration(
    dims: &Dimensions,
    thresholds: &ExplorationThresholds,
    has_external_effect: bool,
    reversible: bool,
) -> ExplorationVerdict {
    if has_external_effect {
        return ExplorationVerdict::DecisionRequired { dimension: None };
    }
    if !reversible {
        return ExplorationVerdict::DecisionRequired { dimension: None };
    }

    let values = dims.to_values();
    for (i, dimension) in ALL.iter().enumerate() {
        let Some(value) = values[i] else {
            continue;
        };
        let Some(limit) = threshold_of(thresholds, *dimension) else {
            continue;
        };
        if value > limit {
            return ExplorationVerdict::DecisionRequired {
                dimension: Some(*dimension),
            };
        }
    }
    ExplorationVerdict::AutoAllowed
}

/// 某一维的阈值；**该维没有判据时给 `None`**。**无通配臂**——加变体即编译不过
/// （逼着这一行跟着变）。
fn threshold_of(thresholds: &ExplorationThresholds, kind: DimensionKind) -> Option<i64> {
    match kind {
        DimensionKind::Money => Some(thresholds.money),
        DimensionKind::WallTime => Some(thresholds.wall_time),
        DimensionKind::Token => None,
        DimensionKind::GpuTime => Some(thresholds.gpu_time),
        DimensionKind::NetworkTransfer => Some(thresholds.network_transfer),
    }
}
