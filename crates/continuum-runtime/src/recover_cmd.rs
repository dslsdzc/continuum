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
/// 注册三个钩子，各占 §319 的一个阶段：
/// - [`MarkRunningNodesLost`]（P1）：`RUNNING` / `VERIFYING` 的节点标 `LOST`；
/// - [`MarkExecutingAsUnknown`]（P2 下篇）：停在 `EXECUTING` 的效应标 `UNKNOWN`。
///   这一支**不猜**成 `COMMITTED` / `FAILED`——进程死在外部操作执行到一半时，
///   外部究竟做没做成本进程无从得知（设计第 6.4 节）；
/// - [`continuum_budget::BudgetReservationRecovery`]（P4 本层）：把悬空的预算预留
///   按预扣额结算（设计 §12.3 的第二处）；它挂在 `MarkLostExecutions` 上。
///
/// **`mark lost executions` 这一态此前没有任何钩子注册**：第一个钩子的文档自称覆盖
/// 「`reconcile running nodes` 与 `mark lost executions`」两个阶段，而它的 `phase()`
/// 只返回 `ReconcileRunningNodes`（见 [`crate::recovery`] 的两处文档）——§319 点名的
/// 那一态在注册侧一直空着，上面第三个钩子把它补上（唯一可观察证据：`tests/startup.rs`
/// 的 `the_mark_lost_executions_phase_now_has_exactly_one_hook`）。**`RecoveryPhase`
/// 的五态一个不动、不新增阶段。**
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
    let budget = continuum_budget::BudgetReservationRecovery;

    let mut registry = RecoveryRegistry::new();
    registry.register(Box::new(nodes));
    registry.register(Box::new(effects));
    registry.register(Box::new(budget));
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
