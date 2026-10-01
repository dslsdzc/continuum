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

    // PENDING/READY → BLOCKED 由设计第 12 节的阻塞规则要求：CONTROL 与 DEPENDENCY
    // 前驱失败时，依赖它的节点通常尚未运行，正处在 PENDING 或 READY。
    let legal = matches!(
        (from, to),
        (Pending, Ready)
            | (Pending, Blocked)
            | (Ready, Blocked)
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

/// 判定状态是否为**终态**（设计第 9 节：COMPLETED、FAILED、CANCELLED、
/// INVALIDATED、LOST 五种）。
///
/// 判据是「终态」而非「无出边」。INVALIDATED 与 CANCELLED 可由任意状态到达，
/// 包括彼此与 COMPLETED（`§306`：输入变化时受影响节点被 INVALIDATED，已完成
/// 节点同样适用），故 `Cancelled → Invalidated` 这类出边是存在的。消费方不得
/// 据本谓词推断节点没有后继；需要「无出边」时请另查边集。
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
