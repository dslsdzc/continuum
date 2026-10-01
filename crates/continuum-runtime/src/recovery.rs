//! P1 注册到 P0 恢复流程的钩子。

use continuum_graph::mark_node_lost;
use continuum_persist::{PersistError, RecoveryHook, RecoveryPhase, Tx, Value};
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
        let rows = tx.query(
            "SELECT graph_id, node_id FROM adfir_node WHERE state IN ('running', 'verifying')",
            &[],
        )?;
        for row in &rows {
            let graph_id = match &row[0] {
                Value::Text(s) => s.clone(),
                other => {
                    return Err(PersistError::Database(format!(
                        "graph_id 应为文本，实际 {other:?}"
                    )))
                }
            };
            let node_id = match &row[1] {
                Value::Text(s) => s.clone(),
                other => {
                    return Err(PersistError::Database(format!(
                        "node_id 应为文本，实际 {other:?}"
                    )))
                }
            };
            // 写入交给 continuum-graph：两列的编码辅助函数住在那边，
            // 本 crate 不拼任何列取值字面量，避免两处各写一套再分叉。
            // 上面的 SELECT 保留在本 crate——「哪些节点要处理」是恢复策略。
            mark_node_lost(tx, &graph_id, &node_id)?;
        }
        self.marked.store(rows.len(), Ordering::Relaxed);
        Ok(())
    }
}
