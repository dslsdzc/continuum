//! §12.3 的结算策略：**无读数即按预扣**——一条规则、两处应用里的**第一处**
//! （流式调用，G §14 第 8 条）。
//!
//! # 为什么单独一个模块（而不是并进 `tree.rs` 的 `settle`）
//!
//! `settle` 处理的是**混合**情形（每一维各自「有读数 / 无读数」——实际用量的
//! 逐维口径）；本模块把「**整次没有读数**」这一个情形做成**具名入口**：
//! 调用方不必自己拼「五维全 `None` 的 `Dimensions`」（那是 `settle` 的编码细节，
//! 且拼错了会静默地改变语义），也不容易误传一份「部分有读数」的估计。
//! **它不新增任何语义**——`settle` 对「某维无读数」的处置（按预扣结算该维）
//! 就是本函数的逐维展开；第二处应用（悬空的预留）在
//! [`crate::BudgetReservationRecovery`]。
//!
//! # 保守偏差（写死，代价照实记）
//!
//! 连续多次无读数的执行会把额度用光，而实际可能没花那么多——这是一个
//! **已知的保守偏差**，不是缺陷。方向取 fail-closed 的理由见函数文档。

use continuum_persist::Tx;

use crate::account::Reservation;
use crate::dimensions::Dimensions;
use crate::error::BudgetError;
use crate::tree::{SettleOutcome, settle};

/// §12.3 的一条规则、两处应用：**一次执行如果没有实际读数，就按预扣额结算；不退余量。**
///
/// 两处：**流式调用**（G §14 第 8 条——`StreamChunk` 只有 `delta` / `done`，
/// 没有 usage；本函数）、**悬空的预留**（进程在预留与结算之间死掉——
/// [`crate::BudgetReservationRecovery`]）。
///
/// # 理由一致
///
/// 无读数时「实际 ≤ 预扣」没有依据；按预扣结算**永不低记**（fail-closed），
/// 而释放余量是 fail-open（可能永久漏记一次真实消耗）。
///
/// # 保守偏差的代价（写明）
///
/// 连续多次无读数的执行会把额度用光，而实际可能没花那么多——这是一个
/// **已知的保守偏差**，不是缺陷。它的边界：读数一旦就位（调用方给出该维的实际
/// 值），结算就走 [`settle`] 的读数口径；本函数**只负责「整次没有读数」**这一
/// 情形，混合情形（部分维有读数）由 `settle` 的逐维口径处理。
///
/// # 口径（与 `settle` 完全一致，因为它就是 `settle` 的全 `None` 展开）
///
/// - 结算后 `settled == reserved`（逐维，账本 `settle` 行的 amount 记**预扣额**）、
///   `reserved == 0`（预扣被消化掉——§12.3 的照片 (a)）；
/// - `refunded` 逐维为 `Some(0)`（无读数的维退 0——**余量不退**）；
/// - 失败路径与 `settle` 相同（owner 不在树上 ⇒ [`BudgetError::UnknownOwner`]）；
/// - **不 `commit`**：事务由调用方持有并提交（同本 crate 的全部写路径）。
///
/// # 它是不是第二条构造路径（判过，不是）
///
/// 它消费一枚 [`Reservation`]（由 [`crate::reserve`] 交出），产出
/// [`SettleOutcome`]——**不构造任何角色**。同一枚 `Reservation` 只能经它或
/// `settle` **结算一次**（`Reservation` 不派生 `Clone` / `Copy`——那正是「不双重
/// 结算」的类型层保证，两条路径共用它）。
pub fn settle_without_a_reading(
    tx: &Tx<'_>,
    reservation: Reservation,
    at: i64,
) -> Result<SettleOutcome, BudgetError> {
    // 五维全 `None` = 「整次没有读数」的编码（`settle` 的逐维口径：
    // 无读数的维按预扣结算、不退余量）。
    settle(tx, reservation, Dimensions::from_values([None; 5]), at)
}
