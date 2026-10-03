//! Runtime 入口：解析子命令并分派（设计下篇第 4.1 节）。
//!
//! 迁移集合为 P0 内置迁移加 P1、P2 各层迁移；恢复钩子由各层注册。

use continuum_persist::{run_recovery, Db, PersistError, RecoveryRegistry};
use continuum_runtime::cli::{self, Command};
use std::path::Path;
use std::process::ExitCode;

mod recovery;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match cli::parse(args) {
        Ok(Command::Recover(a)) => match startup(&a.db) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("启动失败: {e}");
                ExitCode::FAILURE
            }
        },
        // `task` 的实现要到 Task 9–12 才有。此处**不伪造任何行为**：不建工作区、
        // 不落库、不启动子进程，只声明尚未接线并返回非零退出码。
        // 替换它的是 **Task 12** 的派发（`task_cmd`）。在那之前，一个「看起来在
        // 做事」的实现会让调用方以为命令跑过了。
        Ok(Command::Task(_)) => {
            eprintln!("「task」子命令尚未接线（由 Task 12 接入）");
            ExitCode::FAILURE
        }
        Err(e) => {
            eprintln!("参数错误：{e}");
            eprintln!("{}", cli::USAGE);
            ExitCode::FAILURE
        }
    }
}

/// `recover` 子命令的实现：打开数据库、应用迁移、执行 §319 恢复五阶段。
///
/// **这是今天的全部实现，不是终态**：Task 12 会在别处补上本子项目的恢复装配
/// （注册 effect 的恢复钩子、加入 policy 的迁移），那时本函数会被接进 `task_cmd`
/// 所在的那套派发里。在此之前它已是真实现——三个既有用例正依赖它的产出。
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
