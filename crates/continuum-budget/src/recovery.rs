//! §12.3 第二处：**悬空预留**的恢复结算——挂在
//! [`RecoveryPhase::MarkLostExecutions`]（§13.1 的四步判据）。
//!
//! # 挂这一态的理由（§13.1 的四步判据，备选逐条排除）
//!
//! 1. **不新增 phase**：`RecoveryPhase` 的五态是 §319 的转录（
//!    `crates/continuum-persist/src/recovery.rs:18` 的注释：「顺序即 §319 的执行
//!    顺序，不得改动」）——本层没有判据去加第六态。
//! 2. **不是 `ReconcileRunningNodes`**：该阶段的钩子 `MarkRunningNodesLost`
//!    正是**产出「哪些运行已 LOST」这个事实的那一步**；本层的结算要**消费**
//!    那个事实。两者挂同一阶段会让正确性依赖**阶段内的注册顺序**，而那不是
//!    一份被文档化的契约。挂到下一阶段（`MarkLostExecutions`）就把这个依赖
//!    消掉：那时 LOST 已经落了库。
//! 3. **不是 `ReconcileIncompleteEffects`**：该阶段的钩子 `MarkExecutingAsUnknown`
//!    处理的是**效应**（把停在 `EXECUTING` 的外部效应标 `UNKNOWN`，
//!    `crates/continuum-effect/src/recovery.rs:47-49`）；而悬空的**预算预留不是
//!    一次效应**——§12.3 的触发是「**这次执行没有读数**」，与「外部做没做成
//!    不知道」是两件事。挂进效应阶段会让一处「效应语义」的改动波及预算记账
//!    （反之亦然）。
//! 4. **不是 `ResumeEligibleTasks`**：那时额度已被后续任务读走，结算太晚。
//!
//! # 不重复 `MarkRunningNodesLost` 已经做掉的那一半
//!
//! 那一步做的是**把节点状态标成 `LOST`**（`continuum_graph::mark_running_nodes_lost`
//! ——**点名说「不是本层的事」，不是本 crate 的依赖**：本 crate 的 workspace 内
//! 依赖只有 `continuum-persist` 与 `continuum-semantics` 两条）；本层做的是
//! **读那个已经成立的事实、把悬空的 `reserve` 按预扣结算**（§12.3）。
//! 两者**一个写状态、一个记账**，不重不漏——**挂到下一阶段正是为了让后者读得到
//! 前者**（上段第 2 条）。否定式照片：`tests/recovery.rs` 的
//! `the_recovery_hook_does_not_repeat_what_mark_running_nodes_lost_did`。
//!
//! # 注册不在本模块（Task 20 的装配项）
//!
//! 本模块只交出钩子类型；`RecoveryRegistry::new()` ＋ `register` 的那两行
//! 落在 `crates/continuum-runtime/src/recover_cmd.rs`（设计 §13.1 的「这条由
//! 本层的实现 task 做」指的是本层交付钩子的那个 task，装配归驱动）。
//! 本 task 的测试直接以 `run(tx)` 调用它（它本来的用法：由注册表在阶段事务内调）。
//!
//! # 不收时钟
//!
//! 钩子写的 `settle` 行用**被消化那条 `reserve` 行自己的 `at`**
//! （`undigested_reserve_rows` 随行读出，见 `src/ledger.rs`）——`RecoveryHook::run`
//! 没有 `at` 入参，本层也不取时钟（同 effect 的 `MarkExecutingAsUnknown` 取
//! 「记录既有的 `updated_at`」的处置：恢复不引入时钟依赖、不改写时间戳语义）。

use continuum_persist::{PersistError, RecoveryHook, RecoveryPhase, Tx};

use crate::dimensions::{Dimensions, rank};
use crate::ledger::{LedgerKind, undigested_reserve_rows};
use crate::persist::append_ledger;

/// §13.1 的恢复钩子：把悬空的 `reserve` **按预扣额结算**（§12.3 第二处），不释放。
///
/// # 逐条结算（不是「一条和」）
///
/// 对**每一条**未消化的 `reserve` 行写一条同额 `settle` 行：账本的消化按
/// **条数**配对（见 `src/ledger.rs` 的 `balances`），只写「未消化额之和」一条
/// 消化不完 N 条悬空（会重复记账、要跑 N 次才收敛）。理由写死在 `src/ledger.rs`
/// 的 `undigested_reserve_rows`（crate 内）的文档里。
///
/// # 只动悬空的那部分
///
/// 输入是「未消化的 `reserve` 行」——已正常结算的预扣不在其中，**不被重结算**；
/// 已释放（若有产生方）的也不在其中。照片：`tests/recovery.rs` 的对照 owner。
///
/// # 事务
///
/// **不 `commit`**：`run` 在阶段事务内被调用（`run_recovery` 的循环持有并提交
/// 事务；任一步失败，调用方必须回滚）。
pub struct BudgetReservationRecovery;

impl RecoveryHook for BudgetReservationRecovery {
    /// 取 [`RecoveryPhase::MarkLostExecutions`]：§319 里「运行已丢失」的账目
    /// 收尾——本层读那个事实、把悬空的预扣按预扣额结算。
    fn phase(&self) -> RecoveryPhase {
        RecoveryPhase::MarkLostExecutions
    }

    fn run(&self, tx: &Tx<'_>) -> Result<(), PersistError> {
        for row in undigested_reserve_rows(tx)? {
            // 单维的 `Dimensions`：`append_ledger` 只对该维写一条 `settle` 行
            // （额 = 该 `reserve` 行的原额；`at` = 该行自己的 `at`）。
            let mut values = [None; 5];
            values[rank(row.dimension)] = Some(row.amount);
            append_ledger(
                tx,
                &row.owner,
                LedgerKind::Settle,
                &Dimensions::from_values(values),
                row.at,
            )?;
        }
        Ok(())
    }
}
