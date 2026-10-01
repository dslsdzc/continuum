//! P1 注册到 P0 恢复流程的钩子。

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
            "SELECT graph_id, node_id FROM adfir_node WHERE state IN ('RUNNING', 'VERIFYING')",
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
            // node_id 只在图内唯一，两张图各有 "n1" 会撞主键，故带 graph_id
            tx.execute(
                "UPDATE adfir_node SET state = 'LOST' WHERE graph_id = ?1 AND node_id = ?2",
                &[Value::text(graph_id.clone()), Value::text(node_id.clone())],
            )?;
            tx.execute(
                "INSERT OR REPLACE INTO node_attempt
                   (graph_id, node_id, attempt, state, failure_class)
                 VALUES (?1, ?2, 1, 'LOST', 'UNKNOWN')",
                &[Value::text(graph_id), Value::text(node_id)],
            )?;
        }
        self.marked.store(rows.len(), Ordering::Relaxed);
        Ok(())
    }
}
