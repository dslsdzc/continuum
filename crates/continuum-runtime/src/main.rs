//! Runtime 入口。P0 阶段只做：打开数据库、应用迁移、执行 §319 恢复五阶段。
//!
//! 各层的恢复钩子注册在 P1 之后接入，本 task 的注册表为空。

use continuum_persist::{run_recovery, Db, PersistError, RecoveryRegistry};
use std::path::Path;
use std::process::ExitCode;

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
    let db = Db::open(path)?;
    let applied = db.migrate()?;
    println!("迁移应用 {applied} 项");

    let registry = RecoveryRegistry::new();
    let report = run_recovery(&db, &registry)?;
    println!("跳过记录 {} 条", report.skipped_records);
    for phase in &report.phases {
        println!("{}: {} 钩子", phase.phase.as_str(), phase.hooks_run);
    }
    Ok(())
}
