use std::path::Path;
use std::process::Command;

fn run(db: &Path) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_continuum-runtime"))
        .args(["recover", "--db"])
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

#[test]
fn startup_applies_migrations_and_runs_five_phases() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let out = run(&path);
    // P0 两条 + P1 两条 + P2 三条（workspace、policy、effect）+ P3 一条（tool）。
    assert!(out.contains("迁移应用 8 项"), "实际输出:\n{out}");
    assert!(out.contains("跳过记录 0 条"), "实际输出:\n{out}");
    for phase in [
        "load durable state",
        "reconcile incomplete effects",
        "reconcile running nodes",
        "mark lost executions",
        "resume eligible tasks",
    ] {
        assert!(out.contains(phase), "输出缺少阶段 {phase}:\n{out}");
    }
}

#[test]
fn second_startup_applies_no_migration() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    run(&path);
    let out = run(&path);
    assert!(out.contains("迁移应用 0 项"), "实际输出:\n{out}");
}

#[test]
fn startup_reports_skipped_records() {
    // 空库上「跳过记录 0 条」恒为 0，区分不了「真读出 0」与「写死 0」。
    // 先写入一条无法解码的事件（schema_version 为文本），
    // 再启动，断言该计数确实是从库里读出来的。
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");

    {
        let db = continuum_persist::Db::open(&path).unwrap();
        db.migrate().unwrap();
        let tx = db.begin().unwrap();
        tx.execute(
            "INSERT INTO events
               (event_id, event_type, schema_version, occurred_at, intent_id, node_id, ignorable, payload)
             VALUES (?1, ?2, ?3, ?4, NULL, NULL, 0, ?5)",
            &[
                continuum_persist::Value::text("e1"),
                continuum_persist::Value::text("node.started"),
                // schema_version 为文本，解码失败 → 计入跳过
                continuum_persist::Value::text("one"),
                continuum_persist::Value::Int(1),
                continuum_persist::Value::text("{}"),
            ],
        )
        .unwrap();
        tx.commit().unwrap();
    }

    let out = run(&path);
    assert!(out.contains("跳过记录 1 条"), "实际输出:\n{out}");
    // 上面的 Db::open 只带 P0 内置迁移，故本次启动补应用 P1 两条、P2 三条与 P3 一条。
    // 本用例要证明的是跳过计数确实从库里读出，迁移数只是顺带断言。
    assert!(out.contains("迁移应用 6 项"), "实际输出:\n{out}");
}

#[test]
fn startup_applies_p1_migrations() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let out = run(&path);
    // P0 两条 + P1 两条 + P2 三条（workspace、policy、effect）+ P3 一条（tool）。
    // 本计数也是「迁移编号不重复」的守卫：编号撞上已应用的那条时，
    // `migrate` 会把它当作已应用而跳过，条数随之少一。
    assert!(out.contains("迁移应用 8 项"), "实际输出:\n{out}");
}

#[test]
fn recovery_marks_running_nodes_as_lost() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    run(&path);

    // 造一个 RUNNING 节点，再启动一次，断言它被标记为 LOST
    {
        let db = continuum_persist::Db::open(&path).unwrap();
        let tx = db.begin().unwrap();
        tx.execute(
            "INSERT INTO adfir_graph (id, version, contract_id, entry_nodes, terminal_nodes)
             VALUES (?1, ?2, ?3, '[]', '[]')",
            &[
                continuum_persist::Value::text("g1"),
                continuum_persist::Value::Int(1),
                continuum_persist::Value::text("c1"),
            ],
        )
        .unwrap();
        tx.execute(
            "INSERT INTO adfir_node
               (graph_id, node_id, operator_id, operator_version, state,
                execution_policy, verification_policy, constraints, capabilities)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            &[
                continuum_persist::Value::text("g1"),
                continuum_persist::Value::text("n1"),
                continuum_persist::Value::text("op"),
                continuum_persist::Value::Int(1),
                continuum_persist::Value::text("running"),
                continuum_persist::Value::text("null"),
                continuum_persist::Value::text("null"),
                continuum_persist::Value::text("[]"),
                continuum_persist::Value::text("[]"),
            ],
        )
        .unwrap();
        tx.commit().unwrap();
    }

    let out = run(&path);
    assert!(out.contains("标记 LOST 1 个节点"), "实际输出:\n{out}");

    let db = continuum_persist::Db::open(&path).unwrap();
    let tx = db.begin().unwrap();
    let rows = tx
        .query("SELECT state FROM adfir_node WHERE node_id = 'n1'", &[])
        .unwrap();
    match &rows[0][0] {
        continuum_persist::Value::Text(s) => assert_eq!(s, "lost"),
        other => panic!("state 应为文本，实际 {other:?}"),
    }
}

#[test]
fn recovery_handles_same_named_nodes_in_different_graphs() {
    // node_id 只在图内唯一。两张图各有 n1 时，node_attempt 的主键若不含
    // graph_id，第二条插入会撞键。Task 11 里无处可写这条用例（它不写该表）。
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    run(&path);

    {
        let db = continuum_persist::Db::open(&path).unwrap();
        let tx = db.begin().unwrap();
        for graph in ["g1", "g2"] {
            tx.execute(
                "INSERT INTO adfir_graph (id, version, contract_id, entry_nodes, terminal_nodes)
                 VALUES (?1, 1, 'c1', '[]', '[]')",
                &[continuum_persist::Value::text(graph)],
            )
            .unwrap();
            tx.execute(
                "INSERT INTO adfir_node
                   (graph_id, node_id, operator_id, operator_version, state,
                    execution_policy, verification_policy, constraints, capabilities)
                 VALUES (?1, 'n1', 'op', 1, 'running', 'null', 'null', '[]', '[]')",
                &[continuum_persist::Value::text(graph)],
            )
            .unwrap();
        }
        tx.commit().unwrap();
    }

    let out = run(&path);
    assert!(out.contains("标记 LOST 2 个节点"), "实际输出:\n{out}");

    let db = continuum_persist::Db::open(&path).unwrap();
    let tx = db.begin().unwrap();
    let rows = tx.query("SELECT COUNT(*) FROM node_attempt", &[]).unwrap();
    match &rows[0][0] {
        continuum_persist::Value::Int(n) => {
            assert_eq!(*n, 2, "两张图各应有一条 attempt 记录")
        }
        other => panic!("计数应为整数，实际 {other:?}"),
    }
}

#[test]
fn recovery_marks_verifying_nodes_as_lost() {
    // §319 的 mark lost executions：崩溃时处于 VERIFYING 的节点同样丢失。
    // 该用例是钩子里 VERIFYING 分支的唯一守卫——没有它，
    // 把 VERIFYING 从 WHERE 条件里删掉不会有任何用例变红。
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    run(&path);
    {
        let db = continuum_persist::Db::open(&path).unwrap();
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
             VALUES ('g1', 'n1', 'op', 1, 'verifying', 'null', 'null', '[]', '[]')",
            &[],
        )
        .unwrap();
        tx.commit().unwrap();
    }

    let out = run(&path);
    assert!(out.contains("标记 LOST 1 个节点"), "实际输出:\n{out}");
}

#[test]
fn recovery_does_not_touch_same_named_nodes_in_other_graphs() {
    // 钩子的 UPDATE 必须带 graph_id：否则会改到另一张图里同名的已完成节点。
    // 该用例是那条 WHERE 条件的唯一守卫。
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    run(&path);
    {
        let db = continuum_persist::Db::open(&path).unwrap();
        let tx = db.begin().unwrap();
        for (graph, state) in [("g1", "running"), ("g2", "completed")] {
            tx.execute(
                "INSERT INTO adfir_graph (id, version, contract_id, entry_nodes, terminal_nodes)
                 VALUES (?1, 1, 'c1', '[]', '[]')",
                &[continuum_persist::Value::text(graph)],
            )
            .unwrap();
            tx.execute(
                "INSERT INTO adfir_node
                   (graph_id, node_id, operator_id, operator_version, state,
                    execution_policy, verification_policy, constraints, capabilities)
                 VALUES (?1, 'n1', 'op', 1, ?2, 'null', 'null', '[]', '[]')",
                &[
                    continuum_persist::Value::text(graph),
                    continuum_persist::Value::text(state),
                ],
            )
            .unwrap();
        }
        tx.commit().unwrap();
    }

    run(&path);

    let db = continuum_persist::Db::open(&path).unwrap();
    let tx = db.begin().unwrap();
    let rows = tx
        .query("SELECT graph_id, state FROM adfir_node ORDER BY graph_id", &[])
        .unwrap();
    let states: Vec<(String, String)> = rows
        .iter()
        .map(|r| {
            let graph = match &r[0] {
                continuum_persist::Value::Text(s) => s.clone(),
                other => panic!("graph_id 应为文本，实际 {other:?}"),
            };
            let state = match &r[1] {
                continuum_persist::Value::Text(s) => s.clone(),
                other => panic!("state 应为文本，实际 {other:?}"),
            };
            (graph, state)
        })
        .collect();
    assert_eq!(
        states,
        vec![
            ("g1".to_owned(), "lost".to_owned()),
            ("g2".to_owned(), "completed".to_owned())
        ],
        "另一张图里同名的已完成节点不得被改动"
    );
}
