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
