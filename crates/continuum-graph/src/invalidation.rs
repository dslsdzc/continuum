//! 图失效传播（§306）。
//!
//! 传播是保守标记：受影响节点与其下游全部标记为 INVALIDATED。
//! 是否真正重算由复用判定（Task 8）决定。

use crate::graph::{AdfirGraph, GraphError};
use crate::ids::NodeId;
use crate::node::NodeState;
use crate::state::transition;

/// 从 `changed` 出发沿 DATA、DEPENDENCY、INVALIDATION 边传播，
/// 把 `changed` 与其全部下游标记为 INVALIDATED。
///
/// 返回被标记的节点（含 `changed` 自身）。上游与不在此范围内的分支不改变状态。
pub fn propagate_invalidation(
    graph: &mut AdfirGraph,
    changed: &NodeId,
) -> Result<Vec<NodeId>, GraphError> {
    let mut marked: Vec<NodeId> = Vec::new();
    let mut stack: Vec<NodeId> = vec![changed.clone()];

    while let Some(current) = stack.pop() {
        if marked.contains(&current) {
            continue;
        }

        let state = graph
            .node(&current)
            .ok_or_else(|| GraphError::UnknownNode { id: current.clone() })?
            .state;
        let next = transition(state, NodeState::Invalidated)
            .map_err(|_| GraphError::UnknownNode { id: current.clone() })?;
        if let Some(node) = graph.node_mut(&current) {
            node.state = next;
        }
        marked.push(current.clone());

        for edge in graph.edges_from(&current) {
            if edge.kind.propagates_invalidation() {
                stack.push(edge.to_node.clone());
            }
        }
    }

    Ok(marked)
}
