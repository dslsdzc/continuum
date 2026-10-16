//! 图失效传播（§306）。
//!
//! 传播是保守标记：受影响节点与其下游全部标记为 INVALIDATED。
//! 是否真正重算由复用判定（Task 8）决定。

use crate::graph::{AdfirGraph, GraphError};
use crate::ids::NodeId;
use crate::node::NodeState;
use crate::state::transition;
use std::collections::{HashMap, HashSet};

/// 从 `changed` 出发沿 DATA、DEPENDENCY、INVALIDATION 边传播，
/// 把 `changed` 与其全部下游标记为 INVALIDATED。
///
/// 返回被标记的节点（含 `changed` 自身）。上游与不在此范围内的分支不改变状态。
///
/// # 本函数的优化来历（行为不变）
///
/// 早先的实现是链深 100→1000→3000 = 0.118 ms→12.0 ms→81.6 ms 的**超线性**
/// 缩放（QA 摸底 §一 发现 2），三处线性扫描各自贡献一部分：
/// 每节点的 `marked.contains`（O(V²)）、每节点 `edges_from` 的全边表过滤
/// （O(V×E)）、以及 `node` / `node_mut` 的线性节点查找（O(V²)）。现在：
/// 判重走 `HashSet`，邻接表一次建出（只收参与传播的边），节点定位走
/// `node_positions` 的 O(1) 下标索引。**公开面不变**（签名与语义一字不动）；
/// 新增的 `node_positions` / `node_at` / `node_mut_at` 是 `pub(crate)`，
/// 不是导出面。
///
/// 逐节点的副作用顺序与改前逐条相同（栈式 DFS、边按 `edges` 表内顺序入栈、
/// 弹出即标记并立即改写状态），故返回的 `marked` 顺序与中途 `Err` 时的
/// 部分标记形态都与旧实现一致。
pub fn propagate_invalidation(
    graph: &mut AdfirGraph,
    changed: &NodeId,
) -> Result<Vec<NodeId>, GraphError> {
    // 一次性索引：id → nodes 下标（O(V)，改前是每节点一次线性查找）。
    let positions = graph.node_positions();

    // 邻接表：一次 O(E) 扫边，只收参与失效传播的边。边集在传播期间不变
    // （本函数不触碰边），故「预建索引」与改前「逐节点现查全表」等价。
    let mut adjacency: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    for edge in graph.edges() {
        if edge.kind.propagates_invalidation() {
            adjacency
                .entry(edge.from_node.clone())
                .or_default()
                .push(edge.to_node.clone());
        }
    }

    let mut marked: Vec<NodeId> = Vec::new();
    let mut seen: HashSet<NodeId> = HashSet::new();
    let mut stack: Vec<NodeId> = vec![changed.clone()];

    while let Some(current) = stack.pop() {
        if !seen.insert(current.clone()) {
            continue;
        }

        let index = *positions
            .get(&current)
            .ok_or_else(|| GraphError::UnknownNode { id: current.clone() })?;
        let state = graph.node_at(index).state;
        let next = transition(state, NodeState::Invalidated)
            .map_err(|_| GraphError::UnknownNode { id: current.clone() })?;
        graph.node_mut_at(index).state = next;
        marked.push(current.clone());

        if let Some(nexts) = adjacency.get(&current) {
            for to in nexts {
                stack.push(to.clone());
            }
        }
    }

    Ok(marked)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::ContractIdRef;

    /// 与原实现逐点对照的等价性夹具：同一张图、同一入口，优化后的标记集合、
    /// 顺序与状态结果都应与「朴素版」一致。
    ///
    /// 朴素版在这里作为**对照实现**保留（只进测试，不进生产路径）：它逐字
    /// 复刻优化前的算法（`marked.contains` + 每节点 `edges_from` 全表过滤 +
    /// `node`/`node_mut` 线性查找），两版跑在同一张图上对比。
    fn propagated_naively(graph: &mut AdfirGraph, changed: &NodeId) -> Result<Vec<NodeId>, GraphError> {
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

    fn sample_graph() -> (AdfirGraph, Vec<NodeId>) {
        use continuum_artifact::ArtifactType;
        use continuum_port::{Direction, Port, PortId};
        let mut g = AdfirGraph::new(crate::ids::GraphId::new("g-eq"), ContractIdRef::new("c-eq"));
        let ids = ["a", "b", "c", "d", "e", "f"];
        for id in ids {
            let mut node = crate::node::Node::new(
                NodeId::new(id),
                continuum_operator::OperatorId::new("op"),
                continuum_operator::OperatorVersion::new(1),
            );
            node.state = NodeState::Completed;
            g.add_node(node).unwrap();
            for (suffix, dir) in [("out", Direction::Output), ("in", Direction::Input)] {
                g.add_port(
                    id,
                    Port::new(
                        PortId::new(format!("{id}-{suffix}")),
                        dir,
                        "p",
                        ArtifactType::Text,
                    ),
                )
                .unwrap();
            }
        }
        // 一条链 a→b→c，一菱形 a→d→f、d 分支另有一条 b→e（多入边），
        // 以及一条不传播的 CONTROL 边 e→f。
        for (from, to, kind) in [
            ("a", "b", crate::edge::EdgeKind::Data),
            ("b", "c", crate::edge::EdgeKind::Data),
            ("a", "d", crate::edge::EdgeKind::Data),
            ("d", "f", crate::edge::EdgeKind::Dependency),
            ("b", "e", crate::edge::EdgeKind::Invalidation),
            ("e", "f", crate::edge::EdgeKind::Control),
        ] {
            g.connect(
                &PortId::new(format!("{from}-out")),
                &PortId::new(format!("{to}-in")),
                kind,
            )
            .unwrap();
        }
        (g, ids.iter().map(|s| NodeId::new(*s)).collect())
    }

    #[test]
    fn optimized_and_naive_agree_on_multi_edge_fixture() {
        // 对每个起点各跑一次：两版的返回集合、顺序与全图状态必须逐点相同。
        for entry in ["a", "b", "c", "d", "e", "f"] {
            let (mut optimized, _) = sample_graph();
            let (mut naive, _) = sample_graph();
            let id = NodeId::new(entry);
            let optimized_hit = propagate_invalidation(&mut optimized, &id).unwrap();
            let naive_hit = propagated_naively(&mut naive, &id).unwrap();
            assert_eq!(optimized_hit, naive_hit, "入口 {entry}：标记序列应逐点一致");
            let states = |g: &AdfirGraph| -> Vec<NodeState> {
                g.nodes().iter().map(|n| n.state).collect()
            };
            assert_eq!(states(&optimized), states(&naive), "入口 {entry}：图状态应一致");
        }
    }

    #[test]
    fn unknown_entry_reports_the_same_error_and_touches_nothing() {
        let (mut optimized, _) = sample_graph();
        let (mut naive, _) = sample_graph();
        let missing = NodeId::new("nope");
        let a = propagate_invalidation(&mut optimized, &missing).unwrap_err();
        let b = propagated_naively(&mut naive, &missing).unwrap_err();
        assert_eq!(a, b, "未知入口的错误应一致（且带的是那个 id）");
        let states = |g: &AdfirGraph| -> Vec<NodeState> {
            g.nodes().iter().map(|n| n.state).collect()
        };
        assert_eq!(states(&optimized), states(&naive), "报错时都不得改状态");
    }
}
