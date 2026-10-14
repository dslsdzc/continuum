//! 本 crate 的错误类型。

use continuum_persist::PersistError;

use crate::dimensions::DimensionKind;
use crate::tree::BudgetOwner;

/// 预算层的失败（设计 §12.2.1 的**唯一一份**变体清单）。
///
/// # 三枚语义失败：**恰好三枚**（§12.2.1 写死）
///
/// [`BudgetError::UnknownOwner`] / [`BudgetError::Insufficient`] /
/// [`BudgetError::NegativeAmount`]——别处不得再加，也不得删减。四行表的
/// 「使用者 × 可返回枚」（`allocate` / `reserve` / `settle` / `check`）逐行照它。
///
/// # 第四枚 [`BudgetError::Persist`]：一处申报（读法写死）
///
/// §12.2.1 的三枚是**预算层的语义失败**，而本层的每个入口都要经 `Tx` 读库——
/// 持久化失败若被降级成上面任何一枚，调用方会以为「数据就是这样」，而真相是
/// 这次压根没读到（本仓对 `Persist` 枚一贯的判据，见 `IntentError::Persist` 一族五处）。
/// **没有它，任何读库函数都写不出**（`?` 的落点），故本实现加这一枚：
/// 它**不是**上面那份「预算层失败」清单的第四项（三枚语义枚一枚不改、语义不漂），
/// 是持久层的失败按本仓一律的处置转出。**照片**：`tests/persist.rs` 的
/// `out_of_table_values_read_back_as_specific_errors` 三条（表外取值不被静默映射、
/// 不被猜成某一枚）。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BudgetError {
    /// 树上没有该 owner 的分配记录（`budget_node` 里查不到它）。**fail-closed**：
    /// 不给 `Within`（§12.4 第 3 条——给 `Within` 等于对一棵不存在的子树放行）。
    /// `owner` 是**入参那一个**。
    #[error("预算树上没有该 owner 的分配记录: {}", .owner.as_key())]
    UnknownOwner { owner: BudgetOwner },

    /// 预扣／分配时某一维余量不足。**承载「是哪一维」**（I1 与 I2 的判据点）。
    #[error("某一维余量不足: {dimension:?}")]
    Insufficient { dimension: DimensionKind },

    /// 一个本该非负的量纲取了负值。**吃 `Dimensions` 的入口共用本枚**
    /// （`allocate` / `reserve` / `settle` / `check`，外加同刻落地的根入账函数
    /// [`crate::record_root`]）——语义同为「本该非负却取了负」，故一律拒收、
    /// 不做豁免。`value` 是**那个负数**。
    #[error("该非负的量纲取了负值: {dimension:?} = {value}")]
    NegativeAmount { dimension: DimensionKind, value: i64 },

    /// `Tx` 的往返失败（含撞主键、表缺失、表外取值）。`#[from]` 转出——
    /// **不吞成上面三枚**（判据见本类型文档的申报段）。
    #[error(transparent)]
    Persist(#[from] PersistError),
}
