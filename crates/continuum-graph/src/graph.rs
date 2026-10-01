//! ADFIR 图（§235）与连接校验（§239）。

use crate::edge::{Edge, EdgeKind};
use crate::ids::{ContractIdRef, GraphId, NodeId};
use crate::node::Node;
use continuum_port::{compatible, Direction, Port, PortError, PortId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum GraphError {
    #[error("节点 {id} 已存在")]
    DuplicateNode { id: NodeId },
    #[error("节点 {id} 不存在")]
    UnknownNode { id: NodeId },
    #[error("端口 {id} 已存在")]
    DuplicatePort { id: PortId },
    #[error("端口 {id} 不存在")]
    UnknownPort { id: PortId },
    #[error("端口类型不匹配：{0}")]
    PortMismatch(#[from] PortError),
    #[error("连接 {from} → {to} 会成环")]
    Cycle { from: PortId, to: PortId },
    #[error("连接的起点 {from} 必须是输出端口，终点 {to} 必须是输入端口")]
    WrongOrientation { from: PortId, to: PortId },
    #[error("入口节点 {id} 的输入端口存在入边")]
    InvalidEntryNode { id: NodeId },
    #[error("出口节点 {id} 的输出端口存在出边")]
    InvalidTerminalNode { id: NodeId },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdfirGraph {
    pub id: GraphId,
    pub version: u32,
    nodes: Vec<Node>,
    ports: HashMap<PortId, (NodeId, Port)>,
    edges: Vec<Edge>,
    entry_nodes: Vec<NodeId>,
    terminal_nodes: Vec<NodeId>,
    pub contract_id: ContractIdRef,
}

impl AdfirGraph {
    pub fn new(id: GraphId, contract_id: ContractIdRef) -> Self {
        Self {
            id,
            version: 1,
            nodes: Vec::new(),
            ports: HashMap::new(),
            edges: Vec::new(),
            entry_nodes: Vec::new(),
            terminal_nodes: Vec::new(),
            contract_id,
        }
    }

    pub fn entry_nodes(&self) -> &[NodeId] {
        &self.entry_nodes
    }

    pub fn terminal_nodes(&self) -> &[NodeId] {
        &self.terminal_nodes
    }

    pub fn set_entry_nodes(&mut self, nodes: Vec<NodeId>) {
        self.entry_nodes = nodes;
    }

    pub fn set_terminal_nodes(&mut self, nodes: Vec<NodeId>) {
        self.terminal_nodes = nodes;
    }

    /// 校验入口与出口节点的两条约束（设计第 8.3 节）。
    ///
    /// 入口节点的输入端口不得有入边；出口节点的输出端口不得有出边。
    /// 图在投入使用前必须通过本校验——增量构造过程中不检查，
    /// 因为构造完成前这些约束无意义。
    pub fn validate(&self) -> Result<(), GraphError> {
        for id in &self.entry_nodes {
            let node = self
                .node(id)
                .ok_or_else(|| GraphError::UnknownNode { id: id.clone() })?;
            let has_incoming = node
                .inputs
                .iter()
                .any(|p| self.edges.iter().any(|e| e.to_port == *p));
            if has_incoming {
                return Err(GraphError::InvalidEntryNode { id: id.clone() });
            }
        }
        for id in &self.terminal_nodes {
            let node = self
                .node(id)
                .ok_or_else(|| GraphError::UnknownNode { id: id.clone() })?;
            let has_outgoing = node
                .outputs
                .iter()
                .any(|p| self.edges.iter().any(|e| e.from_port == *p));
            if has_outgoing {
                return Err(GraphError::InvalidTerminalNode { id: id.clone() });
            }
        }
        Ok(())
    }

    pub fn add_node(&mut self, node: Node) -> Result<(), GraphError> {
        if self.nodes.iter().any(|n| n.id == node.id) {
            return Err(GraphError::DuplicateNode { id: node.id });
        }
        self.nodes.push(node);
        Ok(())
    }

    /// `node` 接受 `&str` 与 `NodeId` 两种写法。
    pub fn add_port(
        &mut self,
        node: impl Into<NodeId>,
        port: Port,
    ) -> Result<(), GraphError> {
        let node: NodeId = node.into();
        let target = self
            .nodes
            .iter_mut()
            .find(|n| n.id == node)
            .ok_or_else(|| GraphError::UnknownNode { id: node.clone() })?;
        if self.ports.contains_key(port.id()) {
            return Err(GraphError::DuplicatePort {
                id: port.id().clone(),
            });
        }
        match port.direction() {
            Direction::Input => target.inputs.push(port.id().clone()),
            Direction::Output => target.outputs.push(port.id().clone()),
        }
        self.ports.insert(port.id().clone(), (node, port));
        Ok(())
    }

    pub fn port(&self, id: &PortId) -> Option<(NodeId, &Port)> {
        self.ports.get(id).map(|(n, p)| (n.clone(), p))
    }

    pub fn node(&self, id: &NodeId) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == *id)
    }

    pub fn node_mut(&mut self, id: &NodeId) -> Option<&mut Node> {
        self.nodes.iter_mut().find(|n| n.id == *id)
    }

    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }

    pub fn edges_from(&self, node: &NodeId) -> Vec<&Edge> {
        self.edges.iter().filter(|e| e.from_node == *node).collect()
    }

    pub fn edges_to(&self, node: &NodeId) -> Vec<&Edge> {
        self.edges.iter().filter(|e| e.to_node == *node).collect()
    }

    /// 建立一条连接。
    ///
    /// 校验顺序：两端端口存在 → 朝向（起点输出、终点输入）→ `artifact_type` 相同
    /// （§239）→ 参与环检测的边类型不成环。任一校验失败时不留下边。
    ///
    /// 朝向必须在本层校验：`compatible` 只要求两端方向不同，是方向对称的；
    /// 而本层的边有方向（`from_node → to_node`），失效传播与调度排序都按它解读。
    ///
    /// 同节点的输出连回自身输入不单设检查：那是一条环，由环检测拦下。
    pub fn connect(
        &mut self,
        from: &PortId,
        to: &PortId,
        kind: EdgeKind,
    ) -> Result<(), GraphError> {
        let (from_node, from_port) = self
            .port(from)
            .ok_or_else(|| GraphError::UnknownPort { id: from.clone() })?;
        let (to_node, to_port) = self
            .port(to)
            .ok_or_else(|| GraphError::UnknownPort { id: to.clone() })?;

        if from_port.direction() != Direction::Output || to_port.direction() != Direction::Input {
            return Err(GraphError::WrongOrientation {
                from: from.clone(),
                to: to.clone(),
            });
        }

        compatible(from_port, to_port)?;

        let edge = Edge {
            from_node: from_node.clone(),
            from_port: from.clone(),
            to_node: to_node.clone(),
            to_port: to.clone(),
            kind,
        };

        if kind.participates_in_cycle_check() && self.would_create_cycle(&edge) {
            return Err(GraphError::Cycle {
                from: from.clone(),
                to: to.clone(),
            });
        }

        self.edges.push(edge);
        Ok(())
    }

    fn would_create_cycle(&self, candidate: &Edge) -> bool {
        // 从 candidate.to_node 出发沿参与环检测的边前进，
        // 若能到达 candidate.from_node 则成环。
        let mut stack = vec![candidate.to_node.clone()];
        let mut seen: Vec<NodeId> = Vec::new();
        while let Some(current) = stack.pop() {
            if current == candidate.from_node {
                return true;
            }
            if seen.contains(&current) {
                continue;
            }
            seen.push(current.clone());
            for e in self.edges.iter().filter(|e| {
                e.kind.participates_in_cycle_check() && e.from_node == current
            }) {
                stack.push(e.to_node.clone());
            }
        }
        false
    }
}
