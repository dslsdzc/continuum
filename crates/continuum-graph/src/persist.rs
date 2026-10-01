//! 图、节点、端口、边的落库（§317）。

use crate::edge::EdgeKind;
use crate::graph::AdfirGraph;
use crate::ids::{ContractIdRef, GraphId, NodeId};
use crate::node::{Node, NodeState};
use continuum_artifact::ArtifactType;
use continuum_operator::{OperatorId, OperatorVersion};
use continuum_persist::{Migration, PersistError, Tx, Value};
use continuum_port::{Direction, Port, PortId};

pub fn p1_graph_migrations() -> Vec<Migration> {
    vec![Migration::new(
        20,
        "p1_graph",
        "CREATE TABLE adfir_graph (
            id TEXT PRIMARY KEY,
            version INTEGER NOT NULL,
            contract_id TEXT NOT NULL,
            entry_nodes TEXT NOT NULL,
            terminal_nodes TEXT NOT NULL
        );
        CREATE TABLE adfir_node (
            graph_id TEXT NOT NULL,
            node_id TEXT NOT NULL,
            operator_id TEXT NOT NULL,
            operator_version INTEGER NOT NULL,
            state TEXT NOT NULL,
            execution_policy TEXT NOT NULL,
            verification_policy TEXT NOT NULL,
            constraints TEXT NOT NULL,
            capabilities TEXT NOT NULL,
            PRIMARY KEY (graph_id, node_id)
        );
        CREATE TABLE adfir_port (
            graph_id TEXT NOT NULL,
            node_id TEXT NOT NULL,
            port_id TEXT NOT NULL,
            direction TEXT NOT NULL,
            name TEXT NOT NULL,
            artifact_type TEXT NOT NULL,
            PRIMARY KEY (graph_id, port_id)
        );
        CREATE TABLE adfir_edge (
            graph_id TEXT NOT NULL,
            from_node TEXT NOT NULL,
            from_port TEXT NOT NULL,
            to_node TEXT NOT NULL,
            to_port TEXT NOT NULL,
            kind TEXT NOT NULL
        );
        CREATE TABLE execution_profile (
            graph_id TEXT NOT NULL,
            node_id TEXT NOT NULL,
            attempt INTEGER NOT NULL,
            backend TEXT,
            timeout_ms INTEGER,
            retry_policy TEXT NOT NULL,
            cost_budget TEXT,
            PRIMARY KEY (graph_id, node_id, attempt)
        );
        CREATE TABLE node_attempt (
            graph_id TEXT NOT NULL,
            node_id TEXT NOT NULL,
            attempt INTEGER NOT NULL,
            state TEXT NOT NULL,
            failure_class TEXT,
            PRIMARY KEY (graph_id, node_id, attempt)
        );",
    )]
}

pub fn save_graph(tx: &Tx<'_>, graph: &AdfirGraph) -> Result<(), PersistError> {
    let entry: Vec<&str> = graph.entry_nodes().iter().map(|n| n.as_str()).collect();
    let terminal: Vec<&str> = graph.terminal_nodes().iter().map(|n| n.as_str()).collect();
    tx.execute(
        "INSERT INTO adfir_graph (id, version, contract_id, entry_nodes, terminal_nodes)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        &[
            Value::text(graph.id.as_str()),
            Value::Int(i64::from(graph.version)),
            Value::text(graph.contract_id.as_str()),
            Value::text(serde_json::to_string(&entry).expect("可序列化")),
            Value::text(serde_json::to_string(&terminal).expect("可序列化")),
        ],
    )?;

    for node in graph.nodes() {
        tx.execute(
            "INSERT INTO adfir_node
               (graph_id, node_id, operator_id, operator_version, state,
                execution_policy, verification_policy, constraints, capabilities)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            &[
                Value::text(graph.id.as_str()),
                Value::text(node.id.as_str()),
                Value::text(node.operator.id.as_str()),
                Value::Int(i64::from(node.operator.version.as_u32())),
                Value::text(state_str(node.state)),
                Value::text(serde_json::to_string(&node.execution_policy).expect("可序列化")),
                Value::text(serde_json::to_string(&node.verification_policy).expect("可序列化")),
                Value::text(serde_json::to_string(&node.constraints).expect("可序列化")),
                Value::text(serde_json::to_string(&node.capabilities).expect("可序列化")),
            ],
        )?;
        for port_id in node.inputs.iter().chain(node.outputs.iter()) {
            let (_, port) = graph
                .port(port_id)
                .ok_or_else(|| PersistError::Database(format!("端口 {port_id:?} 缺失")))?;
            tx.execute(
                "INSERT INTO adfir_port
                   (graph_id, node_id, port_id, direction, name, artifact_type)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                &[
                    Value::text(graph.id.as_str()),
                    Value::text(node.id.as_str()),
                    Value::text(port.id().as_str()),
                    Value::text(match port.direction() {
                        Direction::Input => "input",
                        Direction::Output => "output",
                    }),
                    Value::text(port.name()),
                    Value::text(artifact_type_str(port.artifact_type())),
                ],
            )?;
        }
    }

    for edge in graph.edges() {
        tx.execute(
            "INSERT INTO adfir_edge
               (graph_id, from_node, from_port, to_node, to_port, kind)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            &[
                Value::text(graph.id.as_str()),
                Value::text(edge.from_node.as_str()),
                Value::text(edge.from_port.as_str()),
                Value::text(edge.to_node.as_str()),
                Value::text(edge.to_port.as_str()),
                Value::text(edge_kind_str(edge.kind)),
            ],
        )?;
    }
    Ok(())
}

pub fn load_graph(tx: &Tx<'_>, id: &GraphId) -> Result<Option<AdfirGraph>, PersistError> {
    let head = tx.query(
        "SELECT id, version, contract_id, entry_nodes, terminal_nodes
         FROM adfir_graph WHERE id = ?1",
        &[Value::text(id.as_str())],
    )?;
    let Some(head) = head.into_iter().next() else {
        return Ok(None);
    };

    let mut graph = AdfirGraph::new(
        GraphId::new(text(&head[0])?),
        ContractIdRef::new(text(&head[2])?),
    );
    graph.version = int(&head[1])? as u32;

    let nodes = tx.query(
        "SELECT node_id, operator_id, operator_version, state,
                execution_policy, verification_policy, constraints, capabilities
         FROM adfir_node WHERE graph_id = ?1 ORDER BY node_id",
        &[Value::text(id.as_str())],
    )?;
    for row in &nodes {
        let mut node = Node::new(
            NodeId::new(text(&row[0])?),
            OperatorId::new(text(&row[1])?),
            OperatorVersion::new(int(&row[2])? as u32),
        );
        node.state = parse_state(&text(&row[3])?)?;
        node.execution_policy = parse_json(&text(&row[4])?)?;
        node.verification_policy = parse_json(&text(&row[5])?)?;
        // 这两项若不回填会静默归零：Node::new 把它们初始化为空 Vec
        node.constraints = parse_strings(&text(&row[6])?)?;
        node.capabilities = parse_strings(&text(&row[7])?)?;
        graph.add_node(node).map_err(graph_err)?;
    }

    let ports = tx.query(
        "SELECT node_id, port_id, direction, name, artifact_type
         FROM adfir_port WHERE graph_id = ?1 ORDER BY port_id",
        &[Value::text(id.as_str())],
    )?;
    for row in &ports {
        let direction = match text(&row[2])?.as_str() {
            "input" => Direction::Input,
            "output" => Direction::Output,
            other => {
                return Err(PersistError::Database(format!("未知 direction: {other}")))
            }
        };
        graph
            .add_port(
                NodeId::new(text(&row[0])?),
                Port::new(
                    PortId::new(text(&row[1])?),
                    direction,
                    text(&row[3])?,
                    parse_artifact_type(&text(&row[4])?)?,
                ),
            )
            .map_err(graph_err)?;
    }

    // 每条边都经 connect 重建，因此 §239 的校验在读回路径上同样生效
    let edges = tx.query(
        "SELECT from_port, to_port, kind FROM adfir_edge
         WHERE graph_id = ?1 ORDER BY from_port, to_port",
        &[Value::text(id.as_str())],
    )?;
    for row in &edges {
        graph
            .connect(
                &PortId::new(text(&row[0])?),
                &PortId::new(text(&row[1])?),
                parse_edge_kind(&text(&row[2])?)?,
            )
            .map_err(graph_err)?;
    }

    let entry: Vec<String> =
        serde_json::from_str(&text(&head[3])?).map_err(|e| PersistError::Database(e.to_string()))?;
    let terminal: Vec<String> =
        serde_json::from_str(&text(&head[4])?).map_err(|e| PersistError::Database(e.to_string()))?;
    graph.set_entry_nodes(entry.into_iter().map(NodeId::new).collect());
    graph.set_terminal_nodes(terminal.into_iter().map(NodeId::new).collect());

    // 读回路径同样要过设计 §8.3 的两条约束校验。这是 validate() 在本子项目内的
    // 唯一调用点：图从库中读回后投入使用前必须成立。
    graph.validate().map_err(graph_err)?;

    Ok(Some(graph))
}

fn graph_err(e: crate::graph::GraphError) -> PersistError {
    PersistError::Database(e.to_string())
}

fn parse_json(s: &str) -> Result<serde_json::Value, PersistError> {
    serde_json::from_str(s).map_err(|e| PersistError::Database(e.to_string()))
}

/// `Node::constraints` / `Node::capabilities` 是 `Vec<String>`，
/// 不是 `serde_json::Value`，故不能走 `parse_json`。
fn parse_strings(s: &str) -> Result<Vec<String>, PersistError> {
    serde_json::from_str(s).map_err(|e| PersistError::Database(e.to_string()))
}

fn text(v: &Value) -> Result<String, PersistError> {
    match v {
        Value::Text(s) => Ok(s.clone()),
        other => Err(PersistError::Database(format!("列应为文本，实际 {other:?}"))),
    }
}

fn int(v: &Value) -> Result<i64, PersistError> {
    match v {
        Value::Int(i) => Ok(*i),
        other => Err(PersistError::Database(format!("列应为整数，实际 {other:?}"))),
    }
}

fn state_str(s: NodeState) -> &'static str {
    match s {
        NodeState::Pending => "pending",
        NodeState::Ready => "ready",
        NodeState::Queued => "queued",
        NodeState::Running => "running",
        NodeState::Waiting => "waiting",
        NodeState::Blocked => "blocked",
        NodeState::Suspended => "suspended",
        NodeState::Verifying => "verifying",
        NodeState::Completed => "completed",
        NodeState::Failed => "failed",
        NodeState::Cancelled => "cancelled",
        NodeState::Invalidated => "invalidated",
        NodeState::Lost => "lost",
    }
}

fn parse_state(s: &str) -> Result<NodeState, PersistError> {
    Ok(match s {
        "pending" => NodeState::Pending,
        "ready" => NodeState::Ready,
        "queued" => NodeState::Queued,
        "running" => NodeState::Running,
        "waiting" => NodeState::Waiting,
        "blocked" => NodeState::Blocked,
        "suspended" => NodeState::Suspended,
        "verifying" => NodeState::Verifying,
        "completed" => NodeState::Completed,
        "failed" => NodeState::Failed,
        "cancelled" => NodeState::Cancelled,
        "invalidated" => NodeState::Invalidated,
        "lost" => NodeState::Lost,
        other => {
            return Err(PersistError::Database(format!("未知 NodeState: {other}")))
        }
    })
}

fn artifact_type_str(t: ArtifactType) -> &'static str {
    match t {
        ArtifactType::SourceTree => "source_tree",
        ArtifactType::Patch => "patch",
        ArtifactType::TestResult => "test_result",
        ArtifactType::Text => "text",
        ArtifactType::Json => "json",
        ArtifactType::Blob => "blob",
    }
}

fn parse_artifact_type(s: &str) -> Result<ArtifactType, PersistError> {
    Ok(match s {
        "source_tree" => ArtifactType::SourceTree,
        "patch" => ArtifactType::Patch,
        "test_result" => ArtifactType::TestResult,
        "text" => ArtifactType::Text,
        "json" => ArtifactType::Json,
        "blob" => ArtifactType::Blob,
        other => {
            return Err(PersistError::Database(format!(
                "未知 ArtifactType: {other}"
            )))
        }
    })
}

fn edge_kind_str(k: EdgeKind) -> &'static str {
    match k {
        EdgeKind::Data => "data",
        EdgeKind::Control => "control",
        EdgeKind::Dependency => "dependency",
        EdgeKind::Evidence => "evidence",
        EdgeKind::Effect => "effect",
        EdgeKind::Invalidation => "invalidation",
    }
}

fn parse_edge_kind(s: &str) -> Result<EdgeKind, PersistError> {
    Ok(match s {
        "data" => EdgeKind::Data,
        "control" => EdgeKind::Control,
        "dependency" => EdgeKind::Dependency,
        "evidence" => EdgeKind::Evidence,
        "effect" => EdgeKind::Effect,
        "invalidation" => EdgeKind::Invalidation,
        other => {
            return Err(PersistError::Database(format!("未知 EdgeKind: {other}")))
        }
    })
}
