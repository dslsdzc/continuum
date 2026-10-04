//! `recover` 子命令的实现（设计下篇第 4.1 节）：打开数据库、应用迁移、跑 §319 恢复五阶段。
//!
//! 与 `task_cmd` 对称——本模块承载该子命令的全部逻辑，`main.rs` 只做分发。**装配点
//! 不在这里**：迁移集合（[`crate::runtime_migrations`]）与钩子的注册见本函数，
//! 而迁移清单留在 `main.rs`，因为 `task` 与 `recover` 共用同一份，两处各写一份清单
//! 会让「注册的集合」有两个来源（见 `main.rs` 的说明）。
//!
//! **stdout 的输出形式是既有用例的断言对象**（`tests/startup.rs` 断言「迁移应用 N 项」
//! 与「跳过记录 N 条」，`tests/migrations.rs` 走同一入口）。改本函数的打印时须一并核对
//! 那两处；这也是本模块的注释反复点名输出的原因。

use crate::recovery::MarkRunningNodesLost;
use crate::runtime_migrations;
use continuum_effect::MarkExecutingAsUnknown;
use continuum_persist::{Db, PersistError, RecoveryRegistry, run_recovery};
use std::path::Path;
use std::sync::atomic::Ordering;

/// 打开数据库、应用迁移、注册恢复钩子并跑完五个阶段，产出打到 stdout。
///
/// 注册两个钩子，各占 §319 的一个阶段：
/// - [`MarkRunningNodesLost`]（P1）：`RUNNING` / `VERIFYING` 的节点标 `LOST`；
/// - [`MarkExecutingAsUnknown`]（P2 下篇）：停在 `EXECUTING` 的效应标 `UNKNOWN`。
///   这一支**不猜**成 `COMMITTED` / `FAILED`——进程死在外部操作执行到一半时，
///   外部究竟做没做成本进程无从得知（设计第 6.4 节）。
///
/// 两个计数都从钩子的 `counter()` 读出：`run` 只拿得到 `&self`，计数经内部可变性带出，
/// 报的是**本次**恢复做了多少（见 `continuum_effect::recovery` 的 `counter` 说明）。
pub fn run(path: &Path) -> Result<(), PersistError> {
    let db = Db::open_with(path, runtime_migrations())?;
    let applied = db.migrate()?;
    println!("迁移应用 {applied} 项");

    let nodes = MarkRunningNodesLost::new();
    let lost = nodes.counter();
    let effects = MarkExecutingAsUnknown::new();

    let mut registry = RecoveryRegistry::new();
    registry.register(Box::new(nodes));
    registry.register(Box::new(effects));
    let report = run_recovery(&db, &registry)?;
    println!("跳过记录 {} 条", report.skipped_records);
    // 效应的条数不打印：设计第 4.1 节只要求「各阶段的钩子数与跳过记录数」，而那两样
    // 下面都有。本函数是从 `main.rs` 移过来的，输出形式是**移动时不动的对象**
    // （`tests/startup.rs` 与 `tests/migrations.rs` 都按这份输出写断言），故不新增行。
    println!("标记 LOST {} 个节点", lost.load(Ordering::Relaxed));
    for phase in &report.phases {
        println!("{}: {} 钩子", phase.phase.as_str(), phase.hooks_run);
    }
    Ok(())
}
