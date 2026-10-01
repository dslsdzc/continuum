use continuum_artifact::ArtifactType;
use continuum_graph::{
    propagate_invalidation, AdfirGraph, ContractIdRef, EdgeKind, GraphId, Node, NodeId, NodeState,
};
use continuum_operator::{OperatorId, OperatorVersion};
use continuum_port::{Direction, Port, PortId};

/// 建一条 x → a → b → c 的 DATA 链，另加一条无关的 d → e 链。
/// 所有节点初始为 COMPLETED。
fn chain() -> AdfirGraph {
    let mut g = AdfirGraph::new(GraphId::new("g1"), ContractIdRef::new("c1"));
    for id in ["x", "a", "b", "c", "d", "e"] {
        let mut n = Node::new(NodeId::new(id), OperatorId::new("op"), OperatorVersion::new(1));
        n.state = NodeState::Completed;
        g.add_node(n).unwrap();
        g.add_port(id, Port::new(PortId::new(format!("{id}-out")), Direction::Output, "o", ArtifactType::Text)).unwrap();
        g.add_port(id, Port::new(PortId::new(format!("{id}-in")), Direction::Input, "i", ArtifactType::Text)).unwrap();
    }
    for (from, to) in [("x", "a"), ("a", "b"), ("b", "c"), ("d", "e")] {
        g.connect(
            &PortId::new(format!("{from}-out")),
            &PortId::new(format!("{to}-in")),
            EdgeKind::Data,
        )
        .unwrap();
    }
    g
}

fn state_of(g: &AdfirGraph, id: &str) -> NodeState {
    g.node(&NodeId::new(id)).expect("节点应存在").state
}

fn invalidate(g: &mut AdfirGraph, id: &str) -> Vec<NodeId> {
    propagate_invalidation(g, &NodeId::new(id)).expect("传播应成功")
}

#[test]
fn invalidation_reaches_the_node_and_all_descendants() {
    let mut g = chain();
    let mut hit = invalidate(&mut g, "a");
    hit.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    assert_eq!(
        hit.iter().map(|n| n.as_str()).collect::<Vec<_>>(),
        vec!["a", "b", "c"]
    );
}

#[test]
fn upstream_and_unrelated_branches_are_untouched() {
    // 02 §3.4 判据 3
    let mut g = chain();
    invalidate(&mut g, "a");

    assert_eq!(state_of(&g, "x"), NodeState::Completed, "上游不得被改动");
    assert_eq!(state_of(&g, "d"), NodeState::Completed, "无关分支不得被改动");
    assert_eq!(state_of(&g, "e"), NodeState::Completed, "无关分支不得被改动");
    assert_eq!(state_of(&g, "a"), NodeState::Invalidated);
    assert_eq!(state_of(&g, "b"), NodeState::Invalidated);
    assert_eq!(state_of(&g, "c"), NodeState::Invalidated);
}

#[test]
fn control_edges_are_not_traversed() {
    let mut g = AdfirGraph::new(GraphId::new("g1"), ContractIdRef::new("c1"));
    for id in ["a", "b"] {
        let mut n = Node::new(NodeId::new(id), OperatorId::new("op"), OperatorVersion::new(1));
        n.state = NodeState::Completed;
        g.add_node(n).unwrap();
        g.add_port(id, Port::new(PortId::new(format!("{id}-out")), Direction::Output, "o", ArtifactType::Text)).unwrap();
        g.add_port(id, Port::new(PortId::new(format!("{id}-in")), Direction::Input, "i", ArtifactType::Text)).unwrap();
    }
    g.connect(&PortId::new("a-out"), &PortId::new("b-in"), EdgeKind::Control).unwrap();

    let hit = invalidate(&mut g, "a");
    assert_eq!(hit.iter().map(|n| n.as_str()).collect::<Vec<_>>(), vec!["a"]);
    assert_eq!(state_of(&g, "b"), NodeState::Completed);
}

#[test]
fn evidence_and_effect_edges_are_not_traversed() {
    let mut g = AdfirGraph::new(GraphId::new("g1"), ContractIdRef::new("c1"));
    for id in ["a", "b"] {
        let mut n = Node::new(NodeId::new(id), OperatorId::new("op"), OperatorVersion::new(1));
        n.state = NodeState::Completed;
        g.add_node(n).unwrap();
        g.add_port(id, Port::new(PortId::new(format!("{id}-out")), Direction::Output, "o", ArtifactType::Json)).unwrap();
        g.add_port(id, Port::new(PortId::new(format!("{id}-in")), Direction::Input, "i", ArtifactType::Json)).unwrap();
    }
    g.connect(&PortId::new("a-out"), &PortId::new("b-in"), EdgeKind::Evidence).unwrap();

    let hit = invalidate(&mut g, "a");
    assert_eq!(hit.iter().map(|n| n.as_str()).collect::<Vec<_>>(), vec!["a"]);
    assert_eq!(state_of(&g, "b"), NodeState::Completed);
}

#[test]
fn dependency_and_invalidation_edges_are_traversed() {
    for kind in [EdgeKind::Dependency, EdgeKind::Invalidation] {
        let mut g = AdfirGraph::new(GraphId::new("g1"), ContractIdRef::new("c1"));
        for id in ["a", "b"] {
            let mut n = Node::new(NodeId::new(id), OperatorId::new("op"), OperatorVersion::new(1));
            n.state = NodeState::Completed;
            g.add_node(n).unwrap();
            g.add_port(id, Port::new(PortId::new(format!("{id}-out")), Direction::Output, "o", ArtifactType::Text)).unwrap();
            g.add_port(id, Port::new(PortId::new(format!("{id}-in")), Direction::Input, "i", ArtifactType::Text)).unwrap();
        }
        g.connect(&PortId::new("a-out"), &PortId::new("b-in"), kind).unwrap();

        let hit = invalidate(&mut g, "a");
        assert_eq!(
            hit.iter().map(|n| n.as_str()).collect::<Vec<_>>(),
            vec!["a", "b"],
            "{kind:?} 边应参与失效传播"
        );
    }
}

#[test]
fn propagation_is_idempotent_over_a_diamond() {
    // a → b, a → c, b → d, c → d：d 只应被标记一次
    let mut g = AdfirGraph::new(GraphId::new("g1"), ContractIdRef::new("c1"));
    for id in ["a", "b", "c", "d"] {
        let mut n = Node::new(NodeId::new(id), OperatorId::new("op"), OperatorVersion::new(1));
        n.state = NodeState::Completed;
        g.add_node(n).unwrap();
        g.add_port(id, Port::new(PortId::new(format!("{id}-out")), Direction::Output, "o", ArtifactType::Text)).unwrap();
        g.add_port(id, Port::new(PortId::new(format!("{id}-in")), Direction::Input, "i", ArtifactType::Text)).unwrap();
    }
    for (from, to) in [("a", "b"), ("a", "c"), ("b", "d"), ("c", "d")] {
        g.connect(&PortId::new(format!("{from}-out")), &PortId::new(format!("{to}-in")), EdgeKind::Data).unwrap();
    }

    let mut hit = invalidate(&mut g, "a");
    hit.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    assert_eq!(
        hit.iter().map(|n| n.as_str()).collect::<Vec<_>>(),
        vec!["a", "b", "c", "d"]
    );
}
