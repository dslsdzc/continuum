use continuum_artifact::ArtifactType;
use continuum_graph::{
    AdfirGraph, ContractIdRef, EdgeKind, GraphError, GraphId, Node, NodeId,
};
use continuum_operator::{OperatorId, OperatorVersion};
use continuum_port::{Direction, Port, PortId};

fn node(id: &str, op: &str) -> Node {
    Node::new(NodeId::new(id), OperatorId::new(op), OperatorVersion::new(1))
}

fn out_port(id: &str, t: ArtifactType) -> Port {
    Port::new(PortId::new(id), Direction::Output, id, t)
}

fn in_port(id: &str, t: ArtifactType) -> Port {
    Port::new(PortId::new(id), Direction::Input, id, t)
}

fn empty_graph() -> AdfirGraph {
    AdfirGraph::new(GraphId::new("g1"), ContractIdRef::new("c1"))
}

#[test]
fn compatible_ports_connect() {
    let mut g = empty_graph();
    g.add_node(node("n1", "a")).unwrap();
    g.add_node(node("n2", "b")).unwrap();
    g.add_port("n1", out_port("o1", ArtifactType::Patch)).unwrap();
    g.add_port("n2", in_port("i1", ArtifactType::Patch)).unwrap();

    g.connect(&PortId::new("o1"), &PortId::new("i1"), EdgeKind::Data)
        .expect("同类型端口应能连接");
    assert_eq!(g.edges().len(), 1);
}

#[test]
fn incompatible_ports_are_rejected_at_construction() {
    // §239 的 MUST：运行时拒绝不兼容连接
    let mut g = empty_graph();
    g.add_node(node("n1", "a")).unwrap();
    g.add_node(node("n2", "b")).unwrap();
    g.add_port("n1", out_port("o1", ArtifactType::SourceTree)).unwrap();
    g.add_port("n2", in_port("i1", ArtifactType::Patch)).unwrap();

    let err = g
        .connect(&PortId::new("o1"), &PortId::new("i1"), EdgeKind::Data)
        .expect_err("类型不同必须被拒绝");
    assert!(matches!(err, GraphError::PortMismatch { .. }), "实际 {err:?}");
    assert!(g.edges().is_empty(), "被拒绝的连接不得留下边");
}

#[test]
fn connecting_an_unknown_port_is_rejected() {
    let mut g = empty_graph();
    match g.connect(&PortId::new("nope"), &PortId::new("nope2"), EdgeKind::Data) {
        Err(GraphError::UnknownPort { .. }) => {}
        other => panic!("未知端口必须被拒绝，实际 {other:?}"),
    }
}

#[test]
fn connecting_the_same_edge_twice_is_rejected() {
    // adfir_edge 表无主键，重复行会原样往返落库；而 edges_to / edges_from 按行计数，
    // 重复边会让 P2 的调度侧双倍计数。故重复边必须在构造层拦下。
    let mut g = empty_graph();
    g.add_node(node("n1", "a")).unwrap();
    g.add_node(node("n2", "b")).unwrap();
    g.add_port("n1", out_port("o1", ArtifactType::Patch)).unwrap();
    g.add_port("n2", in_port("i1", ArtifactType::Patch)).unwrap();

    g.connect(&PortId::new("o1"), &PortId::new("i1"), EdgeKind::Data)
        .expect("首次连接应成功");
    let err = g
        .connect(&PortId::new("o1"), &PortId::new("i1"), EdgeKind::Data)
        .expect_err("同一对端口以同一 kind 连两次必须被拒绝");
    assert!(
        matches!(err, GraphError::DuplicateEdge { .. }),
        "实际 {err:?}"
    );
    assert_eq!(g.edges().len(), 1, "被拒绝的重复边不得留下第二条");
}

#[test]
fn the_same_port_pair_may_carry_different_edge_kinds() {
    // 重复的判据是「两端节点、两端端口、kind 全同」，kind 是边身份的一部分：
    // 同一对端口之间并存 DATA 与 EVIDENCE 是合法的。
    let mut g = empty_graph();
    g.add_node(node("n1", "a")).unwrap();
    g.add_node(node("n2", "b")).unwrap();
    g.add_port("n1", out_port("o1", ArtifactType::Patch)).unwrap();
    g.add_port("n2", in_port("i1", ArtifactType::Patch)).unwrap();

    g.connect(&PortId::new("o1"), &PortId::new("i1"), EdgeKind::Data)
        .unwrap();
    g.connect(&PortId::new("o1"), &PortId::new("i1"), EdgeKind::Evidence)
        .expect("同一对端口上的不同 kind 不是重复边");
    assert_eq!(g.edges().len(), 2);
}

#[test]
fn cycles_through_data_edges_are_rejected() {
    let mut g = empty_graph();
    g.add_node(node("n1", "a")).unwrap();
    g.add_node(node("n2", "b")).unwrap();
    g.add_port("n1", out_port("o1", ArtifactType::Text)).unwrap();
    g.add_port("n1", in_port("i1", ArtifactType::Text)).unwrap();
    g.add_port("n2", out_port("o2", ArtifactType::Text)).unwrap();
    g.add_port("n2", in_port("i2", ArtifactType::Text)).unwrap();

    g.connect(&PortId::new("o1"), &PortId::new("i2"), EdgeKind::Data).unwrap();
    let err = g
        .connect(&PortId::new("o2"), &PortId::new("i1"), EdgeKind::Data)
        .expect_err("成环的连接必须被拒绝");
    assert!(matches!(err, GraphError::Cycle { .. }), "实际 {err:?}");
    assert_eq!(g.edges().len(), 1);
}

#[test]
fn evidence_edges_do_not_participate_in_cycle_detection() {
    // 设计第 8.3 节：EVIDENCE 与 EFFECT 不参与环检测
    let mut g = empty_graph();
    g.add_node(node("n1", "a")).unwrap();
    g.add_node(node("n2", "b")).unwrap();
    g.add_port("n1", out_port("o1", ArtifactType::Json)).unwrap();
    g.add_port("n1", in_port("i1", ArtifactType::Json)).unwrap();
    g.add_port("n2", out_port("o2", ArtifactType::Json)).unwrap();
    g.add_port("n2", in_port("i2", ArtifactType::Json)).unwrap();

    g.connect(&PortId::new("o1"), &PortId::new("i2"), EdgeKind::Data).unwrap();
    g.connect(&PortId::new("o2"), &PortId::new("i1"), EdgeKind::Evidence)
        .expect("EVIDENCE 边成环应被允许");
}

#[test]
fn adding_a_port_to_an_unknown_node_is_rejected() {
    let mut g = empty_graph();
    match g.add_port("nope", out_port("o1", ArtifactType::Text)) {
        Err(GraphError::UnknownNode { .. }) => {}
        other => panic!("未知节点必须被拒绝，实际 {other:?}"),
    }
}

#[test]
fn two_ports_with_the_same_id_are_rejected() {
    let mut g = empty_graph();
    g.add_node(node("n1", "a")).unwrap();
    g.add_port("n1", out_port("o1", ArtifactType::Text)).unwrap();
    match g.add_port("n1", in_port("o1", ArtifactType::Text)) {
        Err(GraphError::DuplicatePort { .. }) => {}
        other => panic!("端口 id 重复必须被拒绝，实际 {other:?}"),
    }
}

#[test]
fn reversed_orientation_is_rejected() {
    // 边有方向（from_node → to_node），失效传播与调度排序按它解读。
    // 以输入端为起点、输出端为终点必须被拒绝。
    let mut g = empty_graph();
    g.add_node(node("n1", "a")).unwrap();
    g.add_node(node("n2", "b")).unwrap();
    g.add_port("n1", out_port("o1", ArtifactType::Text)).unwrap();
    g.add_port("n2", in_port("i1", ArtifactType::Text)).unwrap();

    let err = g
        .connect(&PortId::new("i1"), &PortId::new("o1"), EdgeKind::Data)
        .expect_err("反向连接必须被拒绝");
    assert!(
        matches!(err, GraphError::WrongOrientation { .. }),
        "实际 {err:?}"
    );
    assert!(g.edges().is_empty(), "被拒绝的连接不得留下边");
}

#[test]
fn entry_node_with_incoming_edges_is_rejected() {
    let mut g = empty_graph();
    g.add_node(node("n1", "a")).unwrap();
    g.add_node(node("n2", "b")).unwrap();
    g.add_port("n1", out_port("o1", ArtifactType::Text)).unwrap();
    g.add_port("n2", in_port("i1", ArtifactType::Text)).unwrap();
    g.connect(&PortId::new("o1"), &PortId::new("i1"), EdgeKind::Data)
        .unwrap();

    g.set_entry_nodes(vec![NodeId::new("n2")]);
    let err = g.validate().expect_err("入口节点的输入端口有入边时必须拒绝");
    assert!(
        matches!(err, GraphError::InvalidEntryNode { .. }),
        "实际 {err:?}"
    );

    g.set_entry_nodes(vec![NodeId::new("n1")]);
    g.validate().expect("n1 无输入端口，应通过");
}

#[test]
fn terminal_node_with_outgoing_edges_is_rejected() {
    let mut g = empty_graph();
    g.add_node(node("n1", "a")).unwrap();
    g.add_node(node("n2", "b")).unwrap();
    g.add_port("n1", out_port("o1", ArtifactType::Text)).unwrap();
    g.add_port("n2", in_port("i1", ArtifactType::Text)).unwrap();
    g.connect(&PortId::new("o1"), &PortId::new("i1"), EdgeKind::Data)
        .unwrap();

    g.set_terminal_nodes(vec![NodeId::new("n1")]);
    let err = g.validate().expect_err("出口节点的输出端口有出边时必须拒绝");
    assert!(
        matches!(err, GraphError::InvalidTerminalNode { .. }),
        "实际 {err:?}"
    );

    g.set_terminal_nodes(vec![NodeId::new("n2")]);
    g.validate().expect("n2 无输出端口，应通过");
}
