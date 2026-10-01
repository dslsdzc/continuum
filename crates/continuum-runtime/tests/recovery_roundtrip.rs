//! F1 守卫：`save_graph` 写出的节点状态编码必须与恢复钩子读取的编码一致，
//! 且钩子写出的状态必须能被 `load_graph` 读回。
//!
//! 既有用例全部绕开这条路径：`startup.rs` 用手写 SQL 插状态，与钩子同源，
//! 两处一起写错不会有用例变红；`continuum-graph/tests/persistence.rs` 只往返
//! Ready/Completed，不经过恢复钩子。本用例把 `save_graph` 的产物直接交给
//! 真实启动流程（`continuum-runtime` 二进制，钩子由 `main.rs` 注册）。

use continuum_graph::{load_graph, p1_graph_migrations, save_graph, GraphId, NodeId, NodeState};
use continuum_persist::{builtin_migrations, Db, Migration, Value};
use std::path::Path;
use std::process::Command;

fn run(db: &Path) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_continuum-runtime"))
        .arg(db)
        .output()
        .expect("continuum-runtime 无法执行");
    assert!(
        out.status.success(),
        "启动失败: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("输出不是 UTF-8")
}

fn open(path: &Path) -> Db {
    let mut migrations: Vec<Migration> = builtin_migrations();
    migrations.extend(continuum_artifact::p1_artifact_migrations());
    migrations.extend(p1_graph_migrations());
    let db = Db::open_with(path, migrations).expect("打开数据库失败");
    db.migrate().expect("迁移失败");
    db
}

/// 造一张含 RUNNING 节点的图，并**经 `save_graph` 落库**：写入方是
/// `persist::state_str`，正是本用例要覆盖的一侧。
///
/// runtime 的直接依赖集合是冻结的（见 `tests/dependency_direction.rs`），
/// 拿不到 `continuum_operator::OperatorId` 便无法直接构造 `Node`，
/// 故先借 SQL 落一张 PENDING 图，用 `load_graph` 取回 `AdfirGraph` 对象，
/// 改状态后由 `save_graph` 重写——最终落库的仍是 `state_str` 的产物。
fn save_running_graph(db: &Db) {
    let tx = db.begin().unwrap();
    tx.execute(
        "INSERT INTO adfir_graph (id, version, contract_id, entry_nodes, terminal_nodes)
         VALUES ('g1', 1, 'c1', '[]', '[]')",
        &[],
    )
    .unwrap();
    tx.execute(
        "INSERT INTO adfir_node
           (graph_id, node_id, operator_id, operator_version, state,
            execution_policy, verification_policy, constraints, capabilities)
         VALUES ('g1', 'n1', 'op', 1, 'pending', 'null', 'null', '[]', '[]')",
        &[],
    )
    .unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let mut graph = load_graph(&tx, &GraphId::new("g1"))
        .unwrap()
        .expect("夹具图应能读回");
    tx.commit().unwrap();
    graph
        .node_mut(&NodeId::new("n1"))
        .expect("节点应存在")
        .state = NodeState::Running;

    // save_graph 只做 INSERT，先清掉夹具行
    let tx = db.begin().unwrap();
    tx.execute("DELETE FROM adfir_node WHERE graph_id = 'g1'", &[])
        .unwrap();
    tx.execute("DELETE FROM adfir_graph WHERE id = 'g1'", &[])
        .unwrap();
    save_graph(&tx, &graph).unwrap();
    tx.commit().unwrap();
}

#[test]
fn recovery_reads_what_save_graph_wrote_and_load_graph_reads_it_back() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let db = open(&path);
    save_running_graph(&db);
    drop(db);

    let out = run(&path);
    assert!(out.contains("标记 LOST 1 个节点"), "实际输出:\n{out}");

    let db = open(&path);
    let tx = db.begin().unwrap();
    let rows = tx
        .query(
            "SELECT attempt, state FROM node_attempt WHERE graph_id = 'g1' AND node_id = 'n1'",
            &[],
        )
        .unwrap();
    assert_eq!(rows.len(), 1, "恢复应记一条 attempt，实际 {rows:?}");
    match (&rows[0][0], &rows[0][1]) {
        (Value::Int(attempt), Value::Text(state)) => {
            assert_eq!(*attempt, 1);
            assert_eq!(state, "lost");
        }
        other => panic!("attempt/state 列类型不符，实际 {other:?}"),
    }

    // 恢复跑过之后图仍须读得回来
    let back = load_graph(&tx, &GraphId::new("g1"))
        .expect("恢复后应能读回图")
        .expect("图应存在");
    tx.commit().unwrap();
    assert_eq!(
        back.node(&NodeId::new("n1")).expect("节点应存在").state,
        NodeState::Lost
    );
}
