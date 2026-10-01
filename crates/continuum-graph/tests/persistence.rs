use continuum_artifact::{
    load_artifact, p1_artifact_migrations, save_artifact, Artifact, ArtifactId, ArtifactType,
    ContentHash, PrivacyClass,
};
use continuum_graph::{
    load_graph, p1_graph_migrations, save_graph, AdfirGraph, ContractIdRef, EdgeKind, GraphId, Node,
    NodeId, NodeState,
};
use continuum_operator::{OperatorId, OperatorVersion};
use continuum_persist::{builtin_migrations, Db, Migration, Value};
use continuum_port::{Direction, Port, PortId};
use serde_json::json;

fn db() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let mut migrations: Vec<Migration> = builtin_migrations();
    migrations.extend(p1_artifact_migrations());
    migrations.extend(p1_graph_migrations());
    let db = Db::open_with(&path, migrations).unwrap();
    db.migrate().unwrap();
    (dir, db)
}

fn sample_graph() -> AdfirGraph {
    let mut g = AdfirGraph::new(GraphId::new("g1"), ContractIdRef::new("c1"));
    let mut n = Node::new(NodeId::new("n1"), OperatorId::new("op"), OperatorVersion::new(1));
    n.state = NodeState::Ready;
    g.add_node(n).unwrap();
    g.add_port("n1", Port::new(PortId::new("o1"), Direction::Output, "o", ArtifactType::Patch))
        .unwrap();
    g.add_port("n1", Port::new(PortId::new("i1"), Direction::Input, "i", ArtifactType::SourceTree))
        .unwrap();
    let mut n2 = Node::new(NodeId::new("n2"), OperatorId::new("op2"), OperatorVersion::new(2));
    n2.state = NodeState::Completed;
    g.add_node(n2).unwrap();
    g.add_port("n2", Port::new(PortId::new("o2"), Direction::Output, "o", ArtifactType::SourceTree))
        .unwrap();
    g.add_port("n2", Port::new(PortId::new("i2"), Direction::Input, "i", ArtifactType::Patch))
        .unwrap();
    g.connect(&PortId::new("o2"), &PortId::new("i1"), EdgeKind::Data)
        .unwrap();
    // n2 的输入无入边、n1 的输出无出边，故这一对满足设计 §8.3 的两条约束
    g.set_entry_nodes(vec![NodeId::new("n2")]);
    g.set_terminal_nodes(vec![NodeId::new("n1")]);
    g
}

fn sample_artifact() -> Artifact {
    Artifact {
        id: ArtifactId::new("a1"),
        artifact_type: ArtifactType::Patch,
        content_hash: ContentHash::of(b"diff"),
        size: 4,
        producer_node: Some("n1".to_owned()),
        input_artifacts: vec![],
        metadata: json!({"k": "v"}),
        provenance: json!({}),
        privacy_class: PrivacyClass::Personal,
        version: 1,
    }
}

#[test]
fn graph_round_trips_through_the_database() {
    let (_d, db) = db();
    let graph = sample_graph();

    let tx = db.begin().unwrap();
    save_graph(&tx, &graph).unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let back = load_graph(&tx, &GraphId::new("g1")).unwrap().expect("应能读回");
    tx.commit().unwrap();

    assert_eq!(back.id, graph.id);
    assert_eq!(back.version, graph.version);
    assert_eq!(back.nodes().len(), 2);
    assert_eq!(back.edges().len(), 1);
    assert_eq!(back.node(&NodeId::new("n1")).unwrap().state, NodeState::Ready);
    assert_eq!(back.node(&NodeId::new("n2")).unwrap().state, NodeState::Completed);
    assert_eq!(
        back.port(&PortId::new("o1")).unwrap().1.artifact_type(),
        ArtifactType::Patch
    );
    assert_eq!(back.edges()[0].kind, EdgeKind::Data);
    assert_eq!(back.entry_nodes(), graph.entry_nodes());
    assert_eq!(back.terminal_nodes(), graph.terminal_nodes());
    // load_graph 内部调用过 validate()，能读回即说明两条约束成立
}

#[test]
fn artifact_round_trips_through_the_database() {
    let (_d, db) = db();
    let artifact = sample_artifact();

    let tx = db.begin().unwrap();
    save_artifact(&tx, &artifact).unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let back = load_artifact(&tx, &ArtifactId::new("a1"))
        .unwrap()
        .expect("应能读回");
    tx.commit().unwrap();

    assert_eq!(back, artifact);
}

#[test]
fn loading_an_unknown_graph_returns_none() {
    let (_d, db) = db();
    let tx = db.begin().unwrap();
    assert!(load_graph(&tx, &GraphId::new("nope")).unwrap().is_none());
    tx.commit().unwrap();
}

#[test]
fn load_graph_rejects_a_stored_graph_violating_entry_constraints() {
    // 该用例是 load_graph 内 validate() 调用的唯一守卫：
    // 没有它，删掉那行调用不会有任何用例变红。
    let (_d, db) = db();
    let graph = sample_graph();
    let tx = db.begin().unwrap();
    save_graph(&tx, &graph).unwrap();
    // 把入口改成 n1——它的输入端口 i1 有入边（来自 o2），违反设计 §8.3
    tx.execute(
        "UPDATE adfir_graph SET entry_nodes = ?1 WHERE id = ?2",
        &[Value::text(r#"["n1"]"#), Value::text("g1")],
    )
    .unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let err = load_graph(&tx, &GraphId::new("g1")).expect_err("违反入口约束的图必须被拒绝");
    assert!(err.to_string().contains("入口"), "实际: {err}");
}
