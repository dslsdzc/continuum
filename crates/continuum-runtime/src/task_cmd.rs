//! `task` 子命令的实现（设计下篇第 4.2 节第 1–6 步与第 8 步）。
//!
//! 第 7 步（`--apply` 的策略裁决与集成）是 Task 10，效应记录的写入是 Task 11；两者
//! **尚未接线**，故给了对应的选项即 `Err`，不静默忽略——一个不出声地不记账、不集成的
//! 驱动看起来像在做事，而调用方以为自己的意图已被采纳（同 `cli` 模块的立论）。
//!
//! # 序列（设计第 4.2 节）
//!
//! 1. `detect_backend`；
//! 2. 若为 overlay 且不在用户命名空间内 → 把**自身**经 `unshare -Urm` 重新执行，
//!    环境变量 [`IN_NAMESPACE_ENV`] 标识已进入以防递归。**这一步排在创建任何东西之前**，
//!    否则会建出一个子进程看不见的工作区（设计上篇第 5 节）；
//! 3. `create_task_workspace` → `WorkspaceRecord::from_workspaces` → `save_workspace`；
//! 4. 效应记录的写入次序在 Task 11，本 task 不做任何 Journal 操作；
//! 5. `Sandbox::spawn` 跑 `--exec`，工作目录由 `spawn` 设为 Task 根；
//! 6. 命令退出码非 0 → 仍是失败，且**仍走第 8 步的清理**（失败也要收尾）；
//! 7. 未给 `--apply`：`discard_task_workspace` 成功之后再 `remove_workspace`，
//!    失败则不删记录。
//!
//! # 后端一律取自落库记录（设计第 4.4 节）
//!
//! 第 7 步用的 `backend`、以及 `discard` 要的 Base 与 Task 根，**全部取自
//! [`load_workspace`] 读回的那条记录**，不从命令行参数或请求里现取，也不复用创建时
//! 返回的那个后端。理由见设计第 2.3 节：重建探测挡不住「上层记错了后端、又照错的记法
//! 去回收别的资源」，取用点唯一才是这条要求的前提。命令行里也没有任何入口能给出后端
//! （`--backend` 在解析期即被拒），两者合起来使「驱动用的后端 ≠ 记录里的后端」不可达。
//!
//! # 失败收尾的两个方向
//!
//! [`discard_task_workspace`] 成功之后再 [`remove_workspace`]，失败则不删记录。两个方向
//! 的不一致都不在当期报警，故次序由这里固定：
//!
//! - **有记录无工作区**（先删记录、放弃失败）：`load_workspace` 给出的后端与路径不再
//!   对应任何实物，按它去回收只会失败，而记录本身看不出已作废；
//! - **有工作区无记录**（先放弃、删记录失败）：更隐蔽，要到同一 Intent 再次创建、撞上
//!   仍然存在的分支（worktree）或 Intent 目录（overlay）时才显形。
//!
//! 本文件里另有一处会主动回收：第 3 步 `save_workspace` 失败时，本次刚建出的工作区
//! 还没有记录可用（「有工作区无记录」），故就地按手中已有的句柄放弃它——那时后端与路径
//! 都还在手里，回收不必经记录。

use crate::runtime_migrations;
use crate::sandbox_select::{self, SandboxSelectError};
use continuum_persist::{Db, PersistError};
use continuum_runtime::cli::TaskArgs;
use continuum_sandbox::{Sandbox, SandboxError};
use continuum_workspace::{
    BaseWorkspace, IntentId, TaskWorkspace, WorkspaceBackend, WorkspaceError, WorkspaceRecord,
    create_task_workspace, detect_backend, discard_task_workspace, in_user_namespace, load_workspace,
    remove_workspace, save_workspace,
};
use std::ffi::OsString;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

/// 标识「本进程已经是被重新执行出来的那一份」，防止第 2 步的 re-exec 无限递归。
pub const IN_NAMESPACE_ENV: &str = "CONTINUUM_IN_NAMESPACE";

/// 运行 `task` 子命令。
///
/// `argv` 是**本次调用的原始参数**（不含 argv[0]），第 2 步重新执行自身时原样转交：
/// 从解析结果重建命令行会丢掉调用方实际写的形状（同一个值可以有多种写法），而 re-exec
/// 要的正是「把这一模一样的调用再跑一遍，只是换进命名空间里」。
pub fn run(args: &TaskArgs, argv: &[OsString]) -> Result<(), TaskError> {
    // 尚未接线的选项先挡掉：给了就报错，不静默忽略（见模块文档）。判定排在任何 I/O 与
    // re-exec 之前——一条注定要失败的调用不该先建库、先起进程。
    if !args.effects.is_empty() {
        return Err(TaskError::EffectNotWired {
            count: args.effects.len(),
        });
    }
    if args.apply {
        return Err(TaskError::ApplyNotWired);
    }

    // 第 1 步：判后端。
    let base = BaseWorkspace::new(args.base.clone())?;
    let backend = detect_backend(&base);

    // 第 2 步：判定排在创建任何东西之前。已在用户命名空间内（或以 root 运行而挂得上，
    // 见 `continuum_workspace::overlay`）时不 re-exec；环境变量是防递归的第二道保险。
    if backend == WorkspaceBackend::Overlay
        && std::env::var_os(IN_NAMESPACE_ENV).is_none()
        && !in_user_namespace()
    {
        return reexecute_in_namespace(argv);
    }

    // 第 3 步：建工作区并落库。
    let db = open_db(&args.db)?;
    let (task, created_backend) = create_task_workspace(&base, &args.intent)?;
    let record = WorkspaceRecord::from_workspaces(&base, &task, created_backend, now_millis());
    if let Err(e) = save_record(&db, &record) {
        // 落库失败 ⇒ 出现「有工作区无记录」。记录还没有，第 7 步那条按记录回收的路
        // 走不到，故这里用手里的句柄就地放弃；此后若回收也失败，两条事实都带出去。
        return Err(match discard_task_workspace(&base, &task, created_backend) {
            Ok(()) => e,
            Err(cleanup) => TaskError::Context {
                context: format!("落库失败（{e}）之后，放弃本次建出的工作区也失败"),
                source: Box::new(TaskError::Workspace(cleanup)),
            },
        });
    }

    // 第 4 步：`--effect` 的 Journal 写入在 Task 11；上面已把带 `--effect` 的调用拒掉，
    // 故此处不做任何 Journal 操作。

    // 第 5 步：在沙箱内执行。选中的机制不另作报告——驱动不在正常路径上写 stdout/stderr，
    // 子进程自己的输出即是调用方看到的东西；返回它只为装配与纯函数的答案能相互核对。
    //
    // 机制选择失败也算「执行没成」，走同一条收尾：工作区与记录都不留。留下的代价不只是
    // 一个目录——`save_workspace` 不覆盖同名记录，同一 Intent 会因此再也创建不了。
    let execution = match sandbox_select::select_for_this_machine(args.sandbox) {
        Ok((_mechanism, sandbox)) => run_in_sandbox(&sandbox, &task, &args.exec),
        Err(e) => Err(TaskError::from(e)),
    };

    // 第 6 步与第 7 步：不论执行成败都收尾。
    let cleanup = discard_recorded_workspace(&db, &args.intent);

    match (execution, cleanup) {
        (Ok(()), Ok(())) => Ok(()),
        (Ok(()), Err(cleanup)) => Err(TaskError::Context {
            context: "命令已执行，但随后的清理失败".to_owned(),
            source: Box::new(cleanup),
        }),
        (Err(execution), Ok(())) => Err(execution),
        (Err(execution), Err(cleanup)) => Err(TaskError::Context {
            context: format!("执行失败（{execution}）之后，清理也失败"),
            source: Box::new(cleanup),
        }),
    }
}

/// 打开数据库并应用迁移。
///
/// 迁移集合与 `recover` 共用 [`crate::runtime_migrations`]：`task` 需要 `workspace` 表
/// （第 3 步落库），而两处各写一份清单会让「注册的集合」有两个来源。
fn open_db(path: &Path) -> Result<Db, TaskError> {
    let db = Db::open_with(path, runtime_migrations())?;
    db.migrate()?;
    Ok(db)
}

/// 落库一条 Workspace 记录（第 3 步）。事务在这里开、这里提交——第 5 步启动子进程时要
/// 结束事务，否则那条 `BEGIN IMMEDIATE` 会在子进程运行的全程占着写锁。
fn save_record(db: &Db, record: &WorkspaceRecord) -> Result<(), TaskError> {
    let tx = db.begin()?;
    save_workspace(&tx, record)?;
    tx.commit()?;
    Ok(())
}

/// 第 5 步：在 Task 根内启动 `--exec` 的命令，等它结束。
///
/// 命令退出码非 0 是 `Err`（[`TaskError::CommandFailed`]），但**不是**「不清理」的理由
/// ——第 6 步要求失败也收尾，处置在调用方。
fn run_in_sandbox(
    sandbox: &Sandbox,
    task: &TaskWorkspace,
    exec: &[String],
) -> Result<(), TaskError> {
    // `cli` 已保证 `--exec` 之后至少有一项（命令本身），故这里不会为空。
    let (program, rest) = exec
        .split_first()
        .expect("cli 已保证 --exec 至少消费到一个 token");
    let mut cmd = Command::new(program);
    cmd.args(rest);

    let mut child = sandbox.spawn(task, cmd)?;
    let status = child.wait().map_err(|e| TaskError::WaitFailed {
        reason: e.to_string(),
    })?;
    if status.success() {
        return Ok(());
    }
    // 被信号杀死时没有退出码；取 -1 仅作占位（与 `continuum-workspace` 的 `run` 同法）。
    Err(TaskError::CommandFailed {
        code: status.code().unwrap_or(-1),
    })
}

/// 第 7 步：按落库记录放弃工作区，成功之后再删记录。
///
/// 后端、Base 与 Task 根三者**全部取自记录**（设计第 4.4 节）。`discard` 失败时直接
/// 返回，`tx` 随之析构回滚，记录因此留着——这就是「失败则不删记录」。
fn discard_recorded_workspace(db: &Db, intent: &IntentId) -> Result<(), TaskError> {
    let tx = db.begin()?;
    let record = load_workspace(&tx, intent)?.ok_or_else(|| TaskError::RecordMissing {
        intent: intent.as_str().to_owned(),
    })?;

    let base = BaseWorkspace::new(record.base_path.clone())?;
    let task = TaskWorkspace::new_outside(&base, record.path.clone(), record.intent_id.clone())?;
    discard_task_workspace(&base, &task, record.backend)?;

    remove_workspace(&tx, &record.intent_id)?;
    tx.commit()?;
    Ok(())
}

/// 第 2 步：把**自身**经 `unshare -Urm` 重新执行一遍，原样转交参数。
///
/// 用 `current_exe()` 而不是 `argv[0]`：后者可能是相对路径或只靠 `PATH` 找得到的名字，
/// 而 `unshare` 起的子进程不必与父进程同 cwd。参数原样传（见 [`run`]）。
///
/// 子进程的 stdio 继承本进程，故它的输出照常出现在调用方眼前。
fn reexecute_in_namespace(argv: &[OsString]) -> Result<(), TaskError> {
    let exe = std::env::current_exe().map_err(|e| TaskError::ReExecFailed {
        reason: format!("无法取到自身的可执行文件路径：{e}"),
    })?;
    let status = Command::new("unshare")
        .arg("-Urm")
        .arg(&exe)
        .args(argv)
        .env(IN_NAMESPACE_ENV, "1")
        .status()
        .map_err(|e| TaskError::ReExecFailed {
            reason: format!("无法执行 unshare -Urm：{e}"),
        })?;
    if status.success() {
        return Ok(());
    }
    Err(TaskError::ReExecFailed {
        reason: format!(
            "unshare -Urm 起的子进程以退出码 {} 结束",
            status
                .code()
                .map_or_else(|| "（被信号终止）".to_owned(), |c| c.to_string())
        ),
    })
}

/// 当前时刻，Unix 毫秒。
///
/// 时钟早于 Unix 纪元时取 0 而非报错：`created_at` 只用于记录，回退的时钟不该让一条
/// 本来能跑的调用失败。
fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// `task` 子命令失败的原因。
#[derive(Debug, Error)]
pub enum TaskError {
    /// 给了 `--effect`，而效应记录的写入尚未接线。
    ///
    /// **Task 11 接线后在派发处删除本变体与它的用例。**
    #[error(
        "效应记录尚未接线（Task 11）：本命令带 {count} 条 --effect，驱动不会写入 Journal，故拒绝运行"
    )]
    EffectNotWired { count: usize },
    /// 给了 `--apply`，而策略裁决与集成尚未接线。
    ///
    /// **Task 10 接线后删除本变体与它的用例。** 不静默忽略的理由与上一条相同：用户要求
    /// 集成，而驱动既不集成、也不保留工作区（未接线时唯一能走的第 7 步会把它丢弃），
    /// 那正是「看起来在做事」并顺带毁掉用户的改动。
    #[error("集成尚未接线（Task 10）：本命令给了 --apply，驱动不会问策略、也不会集成，故拒绝运行")]
    ApplyNotWired,
    /// 沙箱机制的选择失败（两种都不可用，或显式指定的那个不可用）。
    #[error("沙箱机制选择失败：{0}")]
    SandboxSelect(#[from] SandboxSelectError),
    /// Workspace 层的失败。
    #[error("Workspace 操作失败：{0}")]
    Workspace(#[from] WorkspaceError),
    /// 沙箱层的失败（机制不可用、施加隔离失败、子进程起不来）。
    #[error("沙箱操作失败：{0}")]
    Sandbox(#[from] SandboxError),
    /// 数据库层的失败。
    #[error("数据库操作失败：{0}")]
    Persist(#[from] PersistError),
    /// 落库之后读不回记录：本次运行刚写过它，缺失说明它被别处删掉了。此时**不清理**
    /// 工作区——按一条不存在的记录去回收，只会把别的资源当成自己的。
    #[error(
        "工作区记录读不回（Intent {intent}）：本次运行刚写过它，缺失说明记录被别处删掉了；\
         工作区原样留着，不按猜出来的后端回收"
    )]
    RecordMissing { intent: String },
    /// `--exec` 的命令以非零退出码结束。
    #[error("命令以退出码 {code} 失败")]
    CommandFailed { code: i32 },
    /// 等待子进程结束失败。
    #[error("等待子进程结束失败：{reason}")]
    WaitFailed { reason: String },
    /// 第 2 步重新执行自身失败。
    #[error("重新执行自身（unshare -Urm）失败：{reason}")]
    ReExecFailed { reason: String },
    /// 在既有错误之上附加的一层上下文（哪一步、还剩什么）。
    #[error("{context}：{source}")]
    Context {
        context: String,
        source: Box<TaskError>,
    },
}
