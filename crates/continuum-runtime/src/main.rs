//! Runtime 入口：打开数据库、应用迁移、执行 §319 恢复五阶段。
//!
//! 迁移集合为 P0 内置迁移加 P1、P2 各层迁移；恢复钩子由各层注册。

use continuum_persist::{run_recovery, Db, PersistError, RecoveryRegistry};
use std::path::Path;
use std::process::ExitCode;

mod recovery;

fn main() -> ExitCode {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "continuum.db".to_owned());
    match startup(Path::new(&path)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("启动失败: {e}");
            ExitCode::FAILURE
        }
    }
}

fn startup(path: &Path) -> Result<(), PersistError> {
    let mut migrations = continuum_persist::builtin_migrations();
    migrations.extend(continuum_artifact::p1_artifact_migrations());
    migrations.extend(continuum_graph::p1_graph_migrations());
    migrations.extend(continuum_workspace::p2_workspace_migrations());

    let db = Db::open_with(path, migrations)?;
    let applied = db.migrate()?;
    println!("迁移应用 {applied} 项");

    let hook = recovery::MarkRunningNodesLost::new();
    let marked = hook.counter();

    let mut registry = RecoveryRegistry::new();
    registry.register(Box::new(hook));
    let report = run_recovery(&db, &registry)?;
    println!("跳过记录 {} 条", report.skipped_records);
    println!(
        "标记 LOST {} 个节点",
        marked.load(std::sync::atomic::Ordering::Relaxed)
    );
    for phase in &report.phases {
        println!("{}: {} 钩子", phase.phase.as_str(), phase.hooks_run);
    }
    Ok(())
}
