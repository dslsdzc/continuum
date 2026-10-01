//! `§237` 的状态迁移表。规范只给出十三个状态名，未定义迁移关系；
//! 本表由 P1 设计第 9 节确定。

use crate::node::NodeState;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StateError {
    #[error("迁移 {from:?} → {to:?} 不在允许的迁移表内")]
    Illegal { from: NodeState, to: NodeState },
}

/// 判定一次状态迁移是否合法。
pub fn transition(from: NodeState, to: NodeState) -> Result<NodeState, StateError> {
    use NodeState::*;

    // INVALIDATED 与 CANCELLED 可由任意状态到达（§306 与取消语义）
    if matches!(to, Invalidated | Cancelled) {
        return Ok(to);
    }

    let legal = matches!(
        (from, to),
        (Pending, Ready)
            | (Ready, Queued)
            | (Queued, Running)
            | (Running, Verifying)
            | (Verifying, Completed)
            | (Running, Waiting)
            | (Waiting, Ready)
            | (Running, Blocked)
            | (Blocked, Ready)
            | (Running, Suspended)
            | (Waiting, Suspended)
            | (Blocked, Suspended)
            | (Suspended, Ready)
            | (Running, Failed)
            | (Verifying, Failed)
            | (Running, Lost)
            | (Verifying, Lost)
    );

    if legal {
        Ok(to)
    } else {
        Err(StateError::Illegal { from, to })
    }
}

pub fn is_terminal(state: NodeState) -> bool {
    matches!(
        state,
        NodeState::Completed
            | NodeState::Failed
            | NodeState::Cancelled
            | NodeState::Invalidated
            | NodeState::Lost
    )
}
