//! P1 注册到 P0 恢复流程的钩子。

use continuum_graph::mark_running_nodes_lost;
use continuum_persist::{PersistError, RecoveryHook, RecoveryPhase, Tx};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// 把处于 RUNNING 或 VERIFYING 的节点标记为 LOST（§319 的
/// `reconcile running nodes` 与 `mark lost executions`）。
pub struct MarkRunningNodesLost {
    marked: Arc<AtomicUsize>,
}

impl MarkRunningNodesLost {
    pub fn new() -> Self {
        Self {
            marked: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// 供启动流程读取本次标记的节点数。
    pub fn counter(&self) -> Arc<AtomicUsize> {
        Arc::clone(&self.marked)
    }
}

impl RecoveryHook for MarkRunningNodesLost {
    fn phase(&self) -> RecoveryPhase {
        RecoveryPhase::ReconcileRunningNodes
    }

    fn run(&self, tx: &Tx<'_>) -> Result<(), PersistError> {
        // 该表的读写整体交给 continuum-graph：本 crate 不再出现列名、列取值
        // 或 Value 的类型匹配。编码与查询条件同源，两处不会分叉。
        let marked = mark_running_nodes_lost(tx)?;
        self.marked.store(marked, Ordering::Relaxed);
        Ok(())
    }
}
