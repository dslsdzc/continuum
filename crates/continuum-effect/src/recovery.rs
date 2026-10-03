//! 重启恢复：把停在 `EXECUTING` 的效应标记为 `UNKNOWN`（设计上篇第 7.4 节）。
//!
//! 进程在外部操作执行到一半时死掉，重启后那条记录停在 `EXECUTING`——外部究竟
//! 做没做成，本进程无从得知，故标成 `UNKNOWN` 交人对账，**不猜**成 `COMMITTED`
//! 或 `FAILED`（设计 §7.6）。
//!
//! 只处理 `EXECUTING`：`PLANNED` / `AUTHORIZED` 尚未触碰外部，`COMMITTED` /
//! `FAILED` / `ROLLED_BACK` 是终态（[`crate::effect::transition`] 的表上无出边），
//! `UNKNOWN` 已经是「不知道」，都无需本钩子插手。
//!
//! 本模块不含 SQL 字面量：表的读写全在 [`crate::persist`]，本模块只把
//! 「查 → 逐条推进」串起来。

use crate::effect::EffectState;
use crate::journal::advance;
use crate::persist;
use continuum_persist::{PersistError, RecoveryHook, RecoveryPhase, Tx};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// 把处于 `EXECUTING` 的效应标记为 `UNKNOWN`。
///
/// 装配到 [`continuum_persist::RecoveryRegistry`] 不在本 task（见 P2 下篇 Task 12）。
pub struct MarkExecutingAsUnknown {
    marked: Arc<AtomicUsize>,
}

impl MarkExecutingAsUnknown {
    pub fn new() -> Self {
        Self {
            marked: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// 供启动流程读取**最近一次 [`RecoveryHook::run`]** 标记的记录数。
    /// 形态与 `continuum_runtime::MarkRunningNodesLost::counter` 对齐：
    /// `run` 只拿得到 `&self`，计数经内部可变性带出。每次 `run` 覆写为本次的数目，
    /// 而不是累加——它报的是本次恢复做了多少，不是历史累计。
    pub fn counter(&self) -> Arc<AtomicUsize> {
        Arc::clone(&self.marked)
    }
}

impl RecoveryHook for MarkExecutingAsUnknown {
    /// 取 [`RecoveryPhase::ReconcileIncompleteEffects`]：§319 里与该阶段对应的
    /// 正是「未完成的外部操作」。
    fn phase(&self) -> RecoveryPhase {
        RecoveryPhase::ReconcileIncompleteEffects
    }

    fn run(&self, tx: &Tx<'_>) -> Result<(), PersistError> {
        let mut marked = 0usize;
        for (id, updated_at) in persist::rows_in_state(tx, EffectState::Executing)? {
            // 经 `advance` 而非直接 UPDATE：表外迁移仍被状态机拦住，且每次成功的
            // 推进会在同一个 `Tx` 内追加一条审计——直接改状态列会绕过该审计。
            //
            // `now` 取该记录既有的 `updated_at`：恢复钩子不收时钟（不引入时钟依赖），
            // 也不让「恢复」这件事改写时间戳语义——记录最后一次实际活动的时刻不变。
            advance(tx, &id, EffectState::Unknown, updated_at)?;
            marked += 1;
        }
        self.marked.store(marked, Ordering::Relaxed);
        Ok(())
    }
}
