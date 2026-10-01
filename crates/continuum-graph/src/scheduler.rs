//! Graph Scheduler（§303）。只判定 READY 与可并发，不做放置。

use crate::graph::AdfirGraph;
use crate::ids::NodeId;
use crate::node::NodeState;
use crate::state::transition;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchedulerConfig {
    /// 并行上限。默认 1——P3 接入资源模型之前资源约束不可知。
    pub max_parallel: usize,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self { max_parallel: 1 }
    }
}

/// 返回本轮可提升为 QUEUED 的节点。
///
/// READY 条件：所有 DATA、CONTROL、DEPENDENCY 入边来源处于 COMPLETED。
/// 返回数量不超过 `max_parallel`。
///
/// 返回集合内任两点之间不存在 CONTROL 或 DATA 路径——该性质由 READY 条件
/// 直接蕴含：排序前驱已 COMPLETED 的节点不会被选为候选（候选只取 PENDING
/// 与 READY），因此不另设连通性检查。设计第 12 节已按此改写。
pub fn select_runnable(graph: &AdfirGraph, config: &SchedulerConfig) -> Vec<NodeId> {
    let mut candidates: Vec<NodeId> = graph
        .nodes()
        .iter()
        .filter(|n| n.state == NodeState::Pending || n.state == NodeState::Ready)
        .filter(|n| predecessors_completed(graph, &n.id))
        .map(|n| n.id.clone())
        .collect();
    candidates.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    candidates.truncate(config.max_parallel);
    candidates
}

fn predecessors_completed(graph: &AdfirGraph, node: &NodeId) -> bool {
    graph.edges_to(node).iter().all(|e| {
        if !e.kind.orders_execution() {
            return true;
        }
        graph
            .node(&e.from_node)
            .map(|n| n.state == NodeState::Completed)
            .unwrap_or(false)
    })
}

/// 把调度排序前驱（DATA、CONTROL、DEPENDENCY）已失败或已阻塞的下游节点
/// 迁移为 BLOCKED。
///
/// 不可推进的前驱态包含 BLOCKED、FAILED、CANCELLED、INVALIDATED、LOST。
/// 返回被标记的节点。
///
/// 传递：某节点被标记 BLOCKED 后，依赖它的节点同样不可推进，一并标记。
/// 迭代到不动点。只处理 PENDING 与 READY 的节点——已 QUEUED 或 RUNNING 的
/// 节点不因前驱失败被拽回，只在下一轮调度时因前置条件不满足而不再 READY。
pub fn apply_blocking(graph: &mut AdfirGraph) -> Vec<NodeId> {
    let mut blocked: Vec<NodeId> = Vec::new();

    loop {
        let mut changed = false;
        let ids: Vec<NodeId> = graph.nodes().iter().map(|n| n.id.clone()).collect();

        for id in ids {
            let state = graph.node(&id).expect("节点应存在").state;
            if !matches!(state, NodeState::Pending | NodeState::Ready) {
                continue;
            }

            // 边集与 `select_runnable` 的 READY 判据一致：DATA 前驱失败时其
            // Artifact 永不出现，下游同样不可推进。只算 Control 与 Dependency
            // 会让这类节点既不可 READY 也不 BLOCKED，永久搁死在 PENDING。
            //
            // 前驱处于 BLOCKED 也计为不可推进，且按状态判定而非「本次调用新标记」——
            // 后者在多轮调度下会漏传。
            let stalled = graph.edges_to(&id).iter().any(|e| {
                if !e.kind.orders_execution() {
                    return false;
                }
                graph
                    .node(&e.from_node)
                    .map(|n| {
                        n.state == NodeState::Blocked
                            || matches!(
                                n.state,
                                NodeState::Failed
                                    | NodeState::Cancelled
                                    | NodeState::Invalidated
                                    | NodeState::Lost
                            )
                    })
                    .unwrap_or(false)
            });

            if stalled {
                let next = transition(state, NodeState::Blocked)
                    .expect("PENDING 与 READY 到 BLOCKED 均为合法迁移");
                if let Some(node) = graph.node_mut(&id) {
                    node.state = next;
                }
                blocked.push(id);
                changed = true;
            }
        }

        if !changed {
            break;
        }
    }

    blocked
}

/// 把调度排序前驱（DATA、CONTROL、DEPENDENCY）已全部 COMPLETED 的 BLOCKED
/// 节点迁移为 READY。
///
/// 与 `apply_blocking` 对称（设计第 12 节的后半句）。返回被解除阻塞的节点。
/// 无排序前驱的 BLOCKED 节点视为满足条件——没有阻塞来源。
pub fn apply_unblocking(graph: &mut AdfirGraph) -> Vec<NodeId> {
    let mut unblocked: Vec<NodeId> = Vec::new();
    let ids: Vec<NodeId> = graph.nodes().iter().map(|n| n.id.clone()).collect();

    for id in ids {
        if graph.node(&id).expect("节点应存在").state != NodeState::Blocked {
            continue;
        }
        let all_completed = graph
            .edges_to(&id)
            .iter()
            .filter(|e| e.kind.orders_execution())
            .all(|e| {
                graph
                    .node(&e.from_node)
                    .map(|n| n.state == NodeState::Completed)
                    .unwrap_or(false)
            });
        if all_completed {
            let next = transition(NodeState::Blocked, NodeState::Ready)
                .expect("BLOCKED 到 READY 为合法迁移");
            if let Some(node) = graph.node_mut(&id) {
                node.state = next;
            }
            unblocked.push(id);
        }
    }

    unblocked
}
