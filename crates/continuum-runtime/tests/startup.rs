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

#[test]
fn startup_applies_migrations_and_runs_five_phases() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let out = run(&path);
    assert!(out.contains("迁移应用 2 项"), "实际输出:\n{out}");
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
    assert!(out.contains("迁移应用 0 项"), "实际输出:\n{out}");
}
