//! §16 与 §17 的事务边界判据：节点状态迁移与它对应的事件必须在同一事务内提交
//! （`§318`）。
//!
//! 本文件是 P1 内唯一覆盖「迁移触发事件」的用例集。三个外发事件中，
//! `ArtifactCreated` 由 Artifact 入库触发，不在这里，见
//! `crates/continuum-graph/tests/persistence.rs` 的 `artifact_round_trips_...`。

use continuum_artifact::ArtifactType;
use continuum_events::EventType;
use continuum_graph::{
    apply_transition, load_graph, p1_graph_migrations, save_graph, AdfirGraph, ApplyError,
    ContractIdRef, EdgeKind, GraphId, Node, NodeId, NodeState,
};
use continuum_operator::{OperatorId, OperatorVersion};
use continuum_persist::{builtin_migrations, Db, Migration, Value};
use continuum_port::{Direction, Port, PortId};

/// 建库、跑迁移，`save_graph` 一张含单个 PENDING 节点 `n1` 与一对已连好的
/// `text → text` 端口的图。
///
/// 端口对连在两个节点之间：单节点要连成对只能是自环，而 `EdgeKind::Data`
/// 参与环检测（`edge.rs` 只豁免 Evidence 与 Effect），`connect` 会返回 `Cycle`。
/// 因此 `n2` 是端口的对端，不参与本文件的任何断言。
///
/// 图必须能过 `load_graph` 的 `validate()`——`state_of` 正是经它读状态的，
/// 而 §8.3 要求入口节点无入边、终止节点无出边，故 `entry = [n1]`、
/// `terminal = [n2]`。
///
/// 返回 `TempDir` 是必须的：数据库是 WAL 模式，临时目录若在夹具返回前被回收，
/// 后续事务建日志文件会失败。与 `tests/persistence.rs` 的 `db()` 同款。
fn fixture() -> (tempfile::TempDir, Db, String) {
    let dir = tempfile::tempdir().unwrap();
    let mut migrations: Vec<Migration> = builtin_migrations();
    migrations.extend(p1_graph_migrations());
    let db = Db::open_with(&dir.path().join("t.db"), migrations).unwrap();
    db.migrate().unwrap();

    let mut graph = AdfirGraph::new(GraphId::new("g1"), ContractIdRef::new("c1"));

    // 被测节点：全图唯一处于 PENDING 的节点
    let n1 = Node::new(NodeId::new("n1"), OperatorId::new("op"), OperatorVersion::new(1));
    graph.add_node(n1).unwrap();
    graph
        .add_port(
            "n1",
            Port::new(PortId::new("o1"), Direction::Output, "out", ArtifactType::Text),
        )
        .unwrap();

    // 端口的对端
    let mut n2 = Node::new(NodeId::new("n2"), OperatorId::new("op2"), OperatorVersion::new(1));
    n2.state = NodeState::Ready;
    graph.add_node(n2).unwrap();
    graph
        .add_port(
            "n2",
            Port::new(PortId::new("i2"), Direction::Input, "in", ArtifactType::Text),
        )
        .unwrap();

    graph
        .connect(&PortId::new("o1"), &PortId::new("i2"), EdgeKind::Data)
        .unwrap();
    graph.set_entry_nodes(vec![NodeId::new("n1")]);
    graph.set_terminal_nodes(vec![NodeId::new("n2")]);

    let tx = db.begin().unwrap();
    save_graph(&tx, &graph).unwrap();
    tx.commit().unwrap();

    let graph_id = graph.id.as_str().to_owned();
    (dir, db, graph_id)
}

/// 经 `load_graph` 读节点状态：走实现自己的 `parse_state`，用例不复制那份编码。
fn state_of(db: &Db, graph_id: &str, node_id: &str) -> NodeState {
    let tx = db.begin().unwrap();
    let graph = load_graph(&tx, &GraphId::new(graph_id))
        .unwrap()
        .expect("图应能读回");
    tx.commit().unwrap();
    graph
        .node(&NodeId::new(node_id))
        .expect("节点应存在")
        .state
}

/// 读该节点在 `events` 表中的事件，按 `occurred_at` 升序。
///
/// `event_type` 经 `EventType` 的 serde 表示还原：写入侧用的是
/// `EventType::as_str()`（见 `Tx::append_event`），两侧是同一份短名的两个来源，
/// 本函数因此顺带钉住「二者没有分叉」。
fn events_of(db: &Db, graph_id: &str, node_id: &str) -> Vec<EventType> {
    let tx = db.begin().unwrap();
    let rows = tx
        .query(
            "SELECT event_type, node_id, payload FROM events ORDER BY occurred_at, rowid",
            &[],
        )
        .unwrap();
    tx.commit().unwrap();

    rows.iter()
        .filter(|r| text_of(&r[1]) == node_id)
        .filter(|r| {
            let payload: serde_json::Value = serde_json::from_str(&text_of(&r[2]))
                .unwrap_or_else(|e| panic!("事件 payload 不是合法 JSON：{e}"));
            payload["graph_id"] == serde_json::Value::String(graph_id.to_owned())
        })
        .map(|r| {
            let name = text_of(&r[0]);
            serde_json::from_value::<EventType>(serde_json::Value::String(name.clone()))
                .unwrap_or_else(|e| panic!("event_type {name} 不是已知的 EventType：{e}"))
        })
        .collect()
}

fn text_of(v: &Value) -> String {
    match v {
        Value::Text(s) => s.clone(),
        other => panic!("列应为文本，实际 {other:?}"),
    }
}

/// PENDING → READY → QUEUED → RUNNING 的公共前缀，最后一段进 RUNNING 并出事件。
fn run_to_running(tx: &continuum_persist::Tx<'_>, graph_id: &str) {
    apply_transition(tx, graph_id, "n1", NodeState::Ready, 1_000, "e1").unwrap();
    apply_transition(tx, graph_id, "n1", NodeState::Queued, 1_100, "e2").unwrap();
    apply_transition(tx, graph_id, "n1", NodeState::Running, 1_200, "e3").unwrap();
}

#[test]
fn transition_and_its_event_commit_together() {
    let (_d, db, graph_id) = fixture();
    let tx = db.begin().unwrap();
    run_to_running(&tx, &graph_id);
    tx.commit().unwrap();

    // 状态确实落库
    assert_eq!(state_of(&db, &graph_id, "n1"), NodeState::Running);
    // 且对应事件同事务写入
    assert_eq!(events_of(&db, &graph_id, "n1"), vec![EventType::NodeStarted]);
}

#[test]
fn illegal_transition_writes_neither_state_nor_event() {
    let (_d, db, graph_id) = fixture();
    // PENDING → COMPLETED 不在迁移表内
    let tx = db.begin().unwrap();
    let err = apply_transition(&tx, &graph_id, "n1", NodeState::Completed, 1_000, "e1").unwrap_err();
    assert!(matches!(err, ApplyError::Illegal(_)), "实际 {err:?}");

    // 事务被丢弃后，状态与事件都不得留下痕迹
    drop(tx);
    assert_eq!(state_of(&db, &graph_id, "n1"), NodeState::Pending);
    assert!(events_of(&db, &graph_id, "n1").is_empty());
}

#[test]
fn rollback_discards_both_state_and_event() {
    // §318 的另一半：合法迁移写完两行之后，只要调用方不提交，两行都不得留下。
    // 若 apply_transition 自行提交（或分两个事务写状态与事件），本用例变红。
    let (_d, db, graph_id) = fixture();
    let tx = db.begin().unwrap();
    run_to_running(&tx, &graph_id);
    drop(tx);

    assert_eq!(state_of(&db, &graph_id, "n1"), NodeState::Pending);
    assert!(events_of(&db, &graph_id, "n1").is_empty());
}

#[test]
fn completed_transition_emits_node_completed() {
    let (_d, db, graph_id) = fixture();
    let tx = db.begin().unwrap();
    // PENDING → READY → QUEUED → RUNNING → VERIFYING → COMPLETED
    run_to_running(&tx, &graph_id);
    apply_transition(&tx, &graph_id, "n1", NodeState::Verifying, 1_300, "e4").unwrap();
    apply_transition(&tx, &graph_id, "n1", NodeState::Completed, 1_400, "e5").unwrap();
    tx.commit().unwrap();

    assert_eq!(
        events_of(&db, &graph_id, "n1"),
        vec![EventType::NodeStarted, EventType::NodeCompleted],
        "五个迁移里只有进入 RUNNING 与 COMPLETED 的两个产生事件"
    );
}

#[test]
fn intermediate_states_emit_no_event() {
    let (_d, db, graph_id) = fixture();
    let tx = db.begin().unwrap();
    apply_transition(&tx, &graph_id, "n1", NodeState::Ready, 1_000, "e1").unwrap();
    tx.commit().unwrap();
    assert!(
        events_of(&db, &graph_id, "n1").is_empty(),
        "READY 不是 §16 列举的外发事件"
    );
}

#[test]
fn unknown_node_is_rejected() {
    let (_d, db, graph_id) = fixture();
    let tx = db.begin().unwrap();
    let err = apply_transition(&tx, &graph_id, "nope", NodeState::Ready, 1_000, "e1").unwrap_err();
    assert!(matches!(err, ApplyError::UnknownNode { .. }), "实际 {err:?}");
    drop(tx);
    assert!(events_of(&db, &graph_id, "nope").is_empty());
}
