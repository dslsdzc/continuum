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
    // P0 两条 + P1 两条 + P2 三条（workspace、policy、effect）+ P3 两条（tool；模型三张表）
    // + P4 三条（alias；语义七表；预算树与账本）。
    assert!(out.contains("迁移应用 12 项"), "实际输出:\n{out}");
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
    // 上面的 Db::open 只带 P0 内置迁移，故本次启动补应用 P1 两条、P2 三条、P3 两条与
    // P4 三条（共 10 项）。本用例要证明的是跳过计数确实从库里读出，迁移数只是顺带断言。
    assert!(out.contains("迁移应用 10 项"), "实际输出:\n{out}");
}

#[test]
fn startup_applies_p1_migrations() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let out = run(&path);
    // P0 两条 + P1 两条 + P2 三条（workspace、policy、effect）+ P3 两条（tool；模型三张表）
    // + P4 三条（alias；语义七表；预算树与账本）。
    // 本计数也是「迁移编号不重复」的守卫：编号撞上已应用的那条时，
    // `migrate` 会把它当作已应用而跳过，条数随之少一。
    assert!(out.contains("迁移应用 12 项"), "实际输出:\n{out}");
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

/// §319 点了名、却一直没有钩子注册的那一态（`mark lost executions`）被补上的
/// **唯一可观察证据**：预算恢复钩子 `BudgetReservationRecovery` 注册在该阶段上。
///
/// 红条件（取反档）：把钩子注册到 `ReconcileRunningNodes` → 本行变
/// `mark lost executions: 0 钩子`，且 `reconcile running nodes: 2 钩子`。
#[test]
fn the_mark_lost_executions_phase_now_has_exactly_one_hook() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let out = run(&path);
    assert!(
        out.contains("mark lost executions: 1 钩子"),
        "实际输出:\n{out}"
    );
}

/// 三张新表由**真实启动**的库建出，且**裸查**各能读到 0 行。
///
/// 「表存在」与「表读得到」是两回事：只查 `sqlite_master` 的存在性会漏掉
/// 「表建了、但读不回」这一路；本用例对每张表直接做一次 `SELECT *`——
/// 空库上读回 0 行。
///
/// 红条件（移除档）：`runtime_migrations()` 少注册任一条 → 那张表不存在，
/// 裸查以 `no such table` 直接失败。
#[test]
fn the_three_new_tables_are_created_by_the_runtime_binary() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    run(&path);

    let db = continuum_persist::Db::open(&path).unwrap();
    let tx = db.begin().unwrap();
    for table in ["alias", "intent", "budget_node"] {
        let rows = tx.query(&format!("SELECT * FROM {table}"), &[]).unwrap();
        assert_eq!(
            rows.len(),
            0,
            "空库上 {table} 应读到 0 行，实得 {} 行",
            rows.len()
        );
    }
}

/// 端到端一条（把 Task 7 的纪律在**生产链**上再钉一次）：经真实启动的库写一条
/// 表外 `scope`，`load_alias` 读回的是**具体** `Err`（`UnknownScope` 带原始取值），
/// 不猜成某枚作用域。
///
/// 红条件两档：装配里少注册迁移 100（**移除档**）→ 裸 `INSERT` 先以
/// `no such table: alias` 失败；读回侧把表外取值取默认态（**放宽档**）→
/// 具体 `Err` 的断言变红。
#[test]
fn an_alias_scope_out_of_table_still_reads_back_as_an_err_through_the_runtime_path() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    run(&path);

    let db = continuum_persist::Db::open(&path).unwrap();
    let tx = db.begin().unwrap();
    // 裸 INSERT：`save_alias` 收的是 `AliasScope`，从类型上写不出表外取值，
    // 故这一行只能绕过它造出来（同 `continuum-canonical/tests/alias.rs` 的处置）——
    // 它是「库里的历史行」这一路的语料。
    tx.execute(
        "INSERT INTO alias (alias, target, scope) VALUES (?1, ?2, ?3)",
        &[
            continuum_persist::Value::text("bad"),
            continuum_persist::Value::text("e-1"),
            continuum_persist::Value::text("global"),
        ],
    )
    .unwrap();

    let err = continuum_canonical::load_alias(&tx, "bad").unwrap_err();
    assert_eq!(
        err,
        continuum_canonical::AliasError::UnknownScope {
            value: "global".to_owned(),
        },
        "表外 scope 应读回带原始取值的 UnknownScope，实得 {err:?}"
    );

    tx.commit().unwrap();
}
