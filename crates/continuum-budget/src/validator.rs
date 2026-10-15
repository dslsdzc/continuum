//! Budget Validator（§110、§12.4）：`check` **只判「超没超」**。
//!
//! # 它覆盖 §110 流程里的哪一格（写死）
//!
//! §110：`Current Plan → Budget Validator →（预计超预算）→ 先寻找合法低成本方案
//! → 找不到 → Decision Required`。**本组件只覆盖中间那一格**：
//! `check` 返回 [`BudgetVerdict`]（`Within` / `Exceeds`），**不找替代方案**。
//!
//! # 「先寻找合法低成本方案」不在本层（§12.4、§16 第 20 条）
//!
//! 那是一次**重新规划**（换模型 / 换节点 / 分块 / 延长时限），是执行层 Planner
//! 的动作（本层没有 Planner）。分工是：`check` 返回 `Exceeds` → **调用方去找
//! 替代方案** → 找不到时调本层的 `Decision` 路径。「找不到」由谁判定、试几次算
//! 「找不到」**规范未给**。**本 crate 的公开面里没有任何函数产出替代方案**
//! （否定式照片：`tests/validator.rs` 的
//! `the_validator_does_not_seek_low_cost_alternatives`）。
//!
//! # 三条口径（§12.4 写死，逐条）
//!
//! 1. **超支不是 `Err`**，是 [`BudgetVerdict::Exceeds`]——§110 的流程要把
//!    「预计超支」当**一个可处置的结论**往下走（找替代方案 → 找不到则 Decision），
//!    做成 `Err` 会让调用方把它当失败而中止。
//! 2. **负值拒收**（[`BudgetError::NegativeAmount`]，§12.2.1）：负估计让下面的
//!    比较**恒为 `Within`**，计划不因它被拦下——与 `settle` 的负 `actual` 同形。
//!    **量纲的单位 / 取值域不是 `Err`**：§333 未给单位（§16 第 19 条），
//!    本层不解释，「值看起来不合理」在本层没有判据。
//! 3. **`UnknownOwner` 是 fail-closed 的**：查不到分配记录时**不给 `Within`**
//!    （给 `Within` 等于对一棵不存在的子树放行——与 §12.2 的 I1 同侧）。

use continuum_persist::Tx;

use crate::dimensions::{ALL, DimensionKind, Dimensions};
use crate::error::BudgetError;
use crate::tree::{BudgetOwner, reject_negative, remaining};

/// §110 的裁决（**超支是一个可处置的结论，不是一个 `Err`**——见模块文档第 1 条）。
///
/// # 只派生 `Debug`（同 [`crate::SettleOutcome`] 的判据）
///
/// 今天没有 `PartialEq` 的消费方（测试逐变体 `match` 断言）；`Debug` 是断言
/// 失败时必须打得出来的那个。
#[derive(Debug)]
pub enum BudgetVerdict {
    /// 逐维都在余量内（**对本次 `estimate` 里 `Some` 的每一维**而言——
    /// 无读数的维不构成约束，见 [`check`] 的口径）。
    Within,
    /// **哪一维**超、**超了多少**（`by = estimate − remaining`）。
    ///
    /// 一次估计里多维多超时，本实现报**字段序第一处**违反者（同
    /// [`crate::SettleOutcome::Over`] 的处置；设计未给这种情形下的选择规则）。
    Exceeds { dimension: DimensionKind, by: i64 },
}

/// §110 的判定：**逐维比较 `estimate` 与 `remaining`**（§12.1 的 I1 的现算值）。
///
/// # 判据是 `remaining`，**不是 `allocation`**（§12.4 末段）
///
/// `remaining` 已经把该 owner 的占用（自己的 `settled` / `reserved`）算进去了；
/// 按 `allocation` 比会把 ENG-005 要的「一次全局判断」拆回「各层各自裁决」。
///
/// # 逐维口径（与 `reserve` 同型）
///
/// - 只对**本次 `estimate` 里 `Some` 的维**判定（该维 `None` = 本次估计不构成约束）；
/// - `remaining` 该维为 `None`（分配额不构成约束）时**跳过**该维；
/// - `estimate[维] > remaining[维]` ⇒ [`BudgetVerdict::Exceeds`]，报**字段序第一处**；
/// - 负 `estimate` 在任何比较之前被拒（[`BudgetError::NegativeAmount`]）。
///
/// # 失败路径
///
/// 本函数是**只读路径**（不写树、不写账本——它连 `at` 都不收）；`Err` 只有两枚：
/// [`BudgetError::NegativeAmount`] 与 [`BudgetError::UnknownOwner`]（§12.2.1 的
/// 四行表里 `check` 那一行的两枚）。**不 `commit`**（它不开事务，`tx` 由调用方持）。
pub fn check(
    tx: &Tx<'_>,
    estimate: &Dimensions,
    owner: BudgetOwner,
) -> Result<BudgetVerdict, BudgetError> {
    reject_negative(estimate)?;
    let remaining = remaining(tx, &owner)?;

    let want = estimate.to_values();
    let have = remaining.dims().to_values();
    for (i, dimension) in ALL.iter().enumerate() {
        let Some(estimate_amount) = want[i] else {
            continue;
        };
        let Some(remaining_amount) = have[i] else {
            continue;
        };
        if estimate_amount > remaining_amount {
            return Ok(BudgetVerdict::Exceeds {
                dimension: *dimension,
                by: estimate_amount - remaining_amount,
            });
        }
    }
    Ok(BudgetVerdict::Within)
}
