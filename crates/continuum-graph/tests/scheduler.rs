use continuum_artifact::ArtifactType;
use continuum_graph::{
    apply_blocking, select_runnable, AdfirGraph, ContractIdRef, EdgeKind, GraphId, Node, NodeId,
    NodeState, SchedulerConfig,
};
use continuum_operator::{OperatorId, OperatorVersion};
use continuum_port::{Direction, Port, PortId};

fn graph(ids: &[&str]) -> AdfirGraph {
    let mut g = AdfirGraph::new(GraphId::new("g1"), ContractIdRef::new("c1"));
    // `for &id in ids`：ids 是 &[&str]，直接迭代得到 &&str
    for &id in ids {
        let mut n = Node::new(NodeId::new(id), OperatorId::new("op"), OperatorVersion::new(1));
        n.state = NodeState::Pending;
        g.add_node(n).unwrap();
        g.add_port(id, Port::new(PortId::new(format!("{id}-out")), Direction::Output, "o", ArtifactType::Text)).unwrap();
        g.add_port(id, Port::new(PortId::new(format!("{id}-in")), Direction::Input, "i", ArtifactType::Text)).unwrap();
    }
    g
}

fn link(g: &mut AdfirGraph, from: &str, to: &str, kind: EdgeKind) {
    g.connect(
        &PortId::new(format!("{from}-out")),
        &PortId::new(format!("{to}-in")),
        kind,
    )
    .unwrap();
}

fn set_state(g: &mut AdfirGraph, id: &str, state: NodeState) {
    g.node_mut(&NodeId::new(id)).expect("节点应存在").state = state;
}

fn names(v: &[NodeId]) -> Vec<String> {
    let mut out: Vec<String> = v.iter().map(|n| n.as_str().to_owned()).collect();
    out.sort();
    out
}

fn config(n: usize) -> SchedulerConfig {
    SchedulerConfig { max_parallel: n }
}

#[test]
fn default_parallel_limit_is_one() {
    assert_eq!(SchedulerConfig::default().max_parallel, 1);
}

#[test]
fn a_node_without_incoming_execution_edges_is_ready() {
    let g = graph(&["a"]);
    assert_eq!(names(&select_runnable(&g, &config(4))), vec!["a"]);
}

#[test]
fn a_node_waits_until_its_input_source_completes() {
    let mut g = graph(&["a", "b"]);
    link(&mut g, "a", "b", EdgeKind::Data);
    set_state(&mut g, "a", NodeState::Running);

    assert_eq!(names(&select_runnable(&g, &config(4))), Vec::<String>::new());
    set_state(&mut g, "a", NodeState::Completed);
    assert_eq!(names(&select_runnable(&g, &config(4))), vec!["b"]);
}

#[test]
fn control_and_dependency_edges_gate_execution_too() {
    for kind in [EdgeKind::Control, EdgeKind::Dependency] {
        let mut g = graph(&["a", "b"]);
        link(&mut g, "a", "b", kind);
        set_state(&mut g, "a", NodeState::Running);
        assert!(
            select_runnable(&g, &config(4)).is_empty(),
            "{kind:?} 边应约束执行顺序"
        );
    }
}

#[test]
fn evidence_and_effect_edges_do_not_gate_execution() {
    for kind in [EdgeKind::Evidence, EdgeKind::Effect] {
        let mut g = graph(&["a", "b"]);
        link(&mut g, "a", "b", kind);
        set_state(&mut g, "a", NodeState::Running);
        assert_eq!(
            names(&select_runnable(&g, &config(4))),
            vec!["b"],
            "{kind:?} 边不应约束执行顺序"
        );
    }
}

#[test]
fn parallel_limit_caps_the_returned_set() {
    let g = graph(&["a", "b", "c"]);
    assert_eq!(select_runnable(&g, &config(2)).len(), 2);
    assert_eq!(select_runnable(&g, &config(1)).len(), 1);
}

#[test]
fn completed_nodes_are_not_returned_for_execution() {
    let mut g = graph(&["a", "b"]);
    link(&mut g, "a", "b", EdgeKind::Data);
    set_state(&mut g, "a", NodeState::Completed);

    assert_eq!(
        names(&select_runnable(&g, &config(8))),
        vec!["b"],
        "已 COMPLETED 的节点不应再次进入可执行集合"
    );
}

#[test]
fn independent_nodes_are_returned_together_up_to_the_limit() {
    let g = graph(&["a", "b", "c"]);
    assert_eq!(
        names(&select_runnable(&g, &config(8))),
        vec!["a", "b", "c"],
        "无排序依赖的节点应同批返回"
    );
}

#[test]
fn blocking_marks_descendants_of_failed_predecessors() {
    let mut g = graph(&["a", "b", "c"]);
    link(&mut g, "a", "b", EdgeKind::Control);
    link(&mut g, "b", "c", EdgeKind::Control);
    set_state(&mut g, "a", NodeState::Failed);

    let blocked = apply_blocking(&mut g);
    assert_eq!(names(&blocked), vec!["b", "c"]);
    assert_eq!(g.node(&NodeId::new("b")).unwrap().state, NodeState::Blocked);
    assert_eq!(g.node(&NodeId::new("c")).unwrap().state, NodeState::Blocked);
}
