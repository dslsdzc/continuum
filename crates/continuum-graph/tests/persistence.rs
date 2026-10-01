use continuum_artifact::{
    load_artifact, p1_artifact_migrations, save_artifact, Artifact, ArtifactId, ArtifactType,
    ContentHash, PrivacyClass,
};
use continuum_graph::{
    load_graph, mark_node_lost, p1_graph_migrations, save_graph, AdfirGraph, ContractIdRef,
    EdgeKind, GraphId, Node, NodeId, NodeState,
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
    // 非空值：这两项曾不落库，读回时静默归零
    n.constraints = vec!["c1".to_owned()];
    n.capabilities = vec!["cap1".to_owned()];
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
        back.node(&NodeId::new("n1")).unwrap().constraints,
        vec!["c1".to_owned()],
        "constraints 不得在读回时归零"
    );
    assert_eq!(
        back.node(&NodeId::new("n1")).unwrap().capabilities,
        vec!["cap1".to_owned()],
        "capabilities 不得在读回时归零"
    );
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
    save_artifact(&tx, &artifact, "e1", 1_000).unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let back = load_artifact(&tx, &ArtifactId::new("a1"))
        .unwrap()
        .expect("应能读回");
    tx.commit().unwrap();

    assert_eq!(back, artifact);
}

#[test]
fn artifact_creation_emits_an_event_in_the_same_transaction() {
    // §16 的三个外发事件中的第三个：Artifact 入库时写 ArtifactCreated。
    // 与 apply_transition 同一条规矩——元数据与事件同一事务（§318）。
    let (_d, db) = db();

    let tx = db.begin().unwrap();
    save_artifact(&tx, &sample_artifact(), "artifact/a1", 1_000).unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let rows = tx
        .query(
            "SELECT event_type, occurred_at, payload FROM events WHERE node_id IS NULL",
            &[],
        )
        .unwrap();
    tx.commit().unwrap();

    assert_eq!(rows.len(), 1, "入库应当且只当产生一条事件：{rows:?}");
    assert_eq!(text_of(&rows[0][0]), "artifact.created");
    assert_eq!(int_of(&rows[0][1]), 1_000);
    assert!(
        text_of(&rows[0][2]).contains("\"artifact_id\":\"a1\""),
        "payload 应点名是哪个 Artifact，实际 {}",
        text_of(&rows[0][2])
    );
}

#[test]
fn artifact_metadata_and_its_event_are_discarded_together() {
    // 上一条用例只演示了正向（同一事务写两样并提交）。这里是另一半：只要调用方
    // 不提交，元数据与事件都不得留下——二者确实是同一条事务里的两行，而不是
    // 「元数据走传入的 tx、事件另起一条自行提交的连接」。
    //
    // 用例名保留 "in the same transaction" 的说法：正反两条合起来才配得上它，
    // 故不改为纯正向的名字。
    let (_d, db) = db();

    let tx = db.begin().unwrap();
    save_artifact(&tx, &sample_artifact(), "artifact/a1", 1_000).unwrap();
    drop(tx);

    let tx = db.begin().unwrap();
    let artifacts = tx.query("SELECT id FROM artifact", &[]).unwrap();
    let events = tx.query("SELECT event_id FROM events", &[]).unwrap();
    tx.commit().unwrap();

    assert!(artifacts.is_empty(), "丢弃事务后元数据不得留下：{artifacts:?}");
    assert!(events.is_empty(), "丢弃事务后事件不得留下：{events:?}");
}

#[test]
fn loading_an_unknown_graph_returns_none() {
    let (_d, db) = db();
    let tx = db.begin().unwrap();
    assert!(load_graph(&tx, &GraphId::new("nope")).unwrap().is_none());
    tx.commit().unwrap();
}

fn text_of(v: &Value) -> String {
    match v {
        Value::Text(s) => s.clone(),
        other => panic!("列应为文本，实际 {other:?}"),
    }
}

fn int_of(v: &Value) -> i64 {
    match v {
        Value::Int(i) => *i,
        other => panic!("列应为整数，实际 {other:?}"),
    }
}

#[test]
fn mark_node_lost_writes_the_columns_the_encoder_produces() {
    // F10 的守卫：`adfir_node.state` 与 `node_attempt` 两列的取值只能有一个来源。
    // `mark_node_lost` 与 `save_graph` 写的是同一列，落库字符串必须逐字相同——
    // 合并前那处 Critical 正是两处各写一套编码而分叉。
    let (_d, db) = db();

    let mut graph = AdfirGraph::new(GraphId::new("g1"), ContractIdRef::new("c1"));
    // 崩溃时正在运行的节点：恢复流程的处理对象
    let mut crashed = Node::new(NodeId::new("n1"), OperatorId::new("op"), OperatorVersion::new(1));
    crashed.state = NodeState::Running;
    graph.add_node(crashed).unwrap();
    // 参照节点：由 save_graph 写成 LOST，其落库字符串即编码侧的产物
    let mut reference = Node::new(NodeId::new("n2"), OperatorId::new("op"), OperatorVersion::new(1));
    reference.state = NodeState::Lost;
    graph.add_node(reference).unwrap();

    let tx = db.begin().unwrap();
    save_graph(&tx, &graph).unwrap();
    // 该节点已有一次历史尝试：写入必须追加为 2，不得覆盖
    tx.execute(
        "INSERT INTO node_attempt (graph_id, node_id, attempt, state, failure_class)
         VALUES ('g1', 'n1', 1, 'failed', 'transient')",
        &[],
    )
    .unwrap();
    mark_node_lost(&tx, "g1", "n1").unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let rows = tx
        .query(
            "SELECT node_id, state FROM adfir_node WHERE graph_id = 'g1' ORDER BY node_id",
            &[],
        )
        .unwrap();
    let states: Vec<(String, String)> = rows
        .iter()
        .map(|r| (text_of(&r[0]), text_of(&r[1])))
        .collect();
    assert_eq!(states.len(), 2, "实际 {states:?}");
    let marked = states[0].1.clone(); // n1：mark_node_lost 写的
    let by_save = states[1].1.clone(); // n2：save_graph 写的同一种状态
    assert_eq!(
        marked, by_save,
        "两条写入路径对同一列必须产出同一字符串，否则该列的编码就有了两个来源"
    );

    let rows = tx
        .query(
            "SELECT attempt, state, failure_class FROM node_attempt
             WHERE graph_id = 'g1' AND node_id = 'n1' ORDER BY attempt",
            &[],
        )
        .unwrap();
    assert_eq!(rows.len(), 2, "既有尝试不得被覆盖，新尝试须追加：{rows:?}");
    assert_eq!(int_of(&rows[0][0]), 1);
    assert_eq!(text_of(&rows[0][1]), "failed");
    assert_eq!(text_of(&rows[0][2]), "transient");
    assert_eq!(int_of(&rows[1][0]), 2, "恢复应追加 attempt = 2");
    assert_eq!(text_of(&rows[1][1]), marked);
    // 该列由 failure_class_str 唯一产出，与 FailureClass 的 SCREAMING_SNAKE_CASE
    // serde 表示不同：P2 读该表时须按同一套写 parse_failure_class。
    assert_eq!(text_of(&rows[1][2]), "unknown");

    // 写入的状态必须能被 load_graph 读回（state_str 与 parse_state 同一套）
    let back = load_graph(&tx, &GraphId::new("g1"))
        .unwrap()
        .expect("图应能读回");
    tx.commit().unwrap();
    assert_eq!(back.node(&NodeId::new("n1")).unwrap().state, NodeState::Lost);
    assert_eq!(back.node(&NodeId::new("n2")).unwrap().state, NodeState::Lost);
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
