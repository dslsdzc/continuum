//! `task` 子命令的实现（设计下篇第 4.2 节第 1–8 步）。
//!
//! 效应记录的写入（第 4 步与第 6 步的 Journal 部分）是 Task 11，**尚未接线**，故给了
//! `--effect` 即 `Err`，不静默忽略——一个不出声地不记账的驱动看起来像在做事，而调用方
//! 以为自己的意图已被采纳（同 `cli` 模块的立论）。
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
//! 6. 命令退出码非 0 → 仍是失败；
//! 7. **命令成功且给了 `--apply`**：[`arbitrate`] 查策略 → 按 [`mints`] 决定是否铸造
//!    批准值 → 经 Gate 应用。**这一支不清理工作区**（见下）；
//! 8. **未给 `--apply`，或命令退出码非 0**：`discard_task_workspace` 成功之后再
//!    `remove_workspace`，失败则不删记录。
//!
//! # 第 7、8 步的两种收尾**不能类推**（设计第 4.2 节）
//!
//! - **第 7 步裁决为「不铸造」时（含第 1 级 `Deny`）：拒绝集成，但保留工作区**，不
//!   `discard`。命令**成功**了，用户的改动是完整可用的，丢弃它才叫「让用户无从恢复」。
//! - **第 8 步反过来：命令失败也照旧清理**。这一支留下的是**半成品**，而更要紧的是
//!   **记录**——[`save_workspace`] 是裸 `INSERT`（`continuum-workspace/src/persist.rs`），
//!   保留记录会让**同一个 Intent 再也创建不了**，用户得手工删记录与分支才能重试。
//!   故「保留工作区」的代价不止一个目录，只在「改动完整且未被批准」那一支才值得付。
//!
//! 同理，**命令退出码非 0 时不做集成**（第 7 步不走）：第 6 步把各 Effect 记 `FAILED`
//! （Task 11），与「同一件事既记为失败、又把它的文件系统结果收进 Base」自相矛盾。
//!
//! # 后端的取用点（设计第 4.4 节）
//!
//! 第 7、8 步用的 `backend` **取自 [`load_workspace`] 读回的那条记录**：那条记录里写的是
//! 哪个后端，集成就按哪个后端做、`discard` 就按哪个后端回收。不复用创建时返回的那个值，
//! 也不重新探测。
//! 理由见设计第 2.3 节：重建探测挡不住「上层记错了后端、又照错的记法去回收别的资源」，
//! 取用点唯一才是这条要求的前提。命令行里也没有任何入口能给出后端（`--backend` 在解析期
//! 即被拒），两者合起来使「驱动用的后端 ≠ 记录里的后端」不可达。
//!
//! 同一处还从记录里取 Base 与 Task 根的路径。**这两项本 task 没有用例钉**：本次运行里
//! 记录中的 `base_path` 就是命令行给的、`path` 就是手里那个句柄的根，换成任一个都观察不到
//! 差异（等价变异体）。它们由设计第 4.4 节「取用点唯一」保证，不由用例保证——**不要把
//! 上面那句设计引用读成对照片**。
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
use continuum_policy::{
    Condition, Decision, ExplicitApproval, Level, Policy, PolicyContext, Scope, decide,
    load_policies,
};
use continuum_runtime::cli::TaskArgs;
use continuum_sandbox::{Sandbox, SandboxError};
use continuum_workspace::{
    BaseWorkspace, GateError, IntegrationGate, IntentId, TaskWorkspace, WorkspaceBackend,
    WorkspaceError, WorkspaceRecord, approve_integration, create_task_workspace, detect_backend,
    discard_task_workspace, in_user_namespace, load_workspace, remove_workspace, save_workspace,
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
        // 落库失败 ⇒ 出现「有工作区无记录」。记录还没有，第 8 步那条按记录回收的路
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

    // 第 7 步：命令**成功**且给了 `--apply` → 查策略、集成。**这一支不清理工作区**，
    // 成功与被拒都不清理（设计第 4.2 节）。命令失败时第 7 步不走（同上）。
    if args.apply && execution.is_ok() {
        return apply_recorded_integration(&db, args).map_err(|e| TaskError::Context {
            context: format!(
                "集成未完成（Intent {}）；工作区与记录原样保留，未清理",
                args.intent.as_str()
            ),
            source: Box::new(e),
        });
    }

    // 第 8 步：未给 `--apply`，**或命令退出码非 0** → 不论执行成败都收尾（见模块文档
    // 「两种收尾不能类推」：这一支留下的记录会让同一个 Intent 再也创建不了）。
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
/// ——第 8 步对非 0 退出码同样生效，处置在调用方。它也**不是**集成的理由：第 7 步以
/// 「命令退出码为 0」为条件，故这一支不集成。
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

/// 第 8 步：按落库记录放弃工作区，成功之后再删记录。
///
/// **`backend` 取自记录**（设计第 4.4 节）：记录里写的是哪个后端，`discard` 就按哪个后端
/// 回收。这一项有判别力——把它换成写死的某个后端，worktree 与 overlay 两个方向都会以
/// 另一条错误失败（见 `tests/task_cli.rs` 的
/// `a_failed_discard_keeps_the_workspace_record` 与 M5）。同一处取出的 `base_path` 与
/// `path` 则**没有判别力**（本次运行里它们与命令行/句柄同值，换掉不可观察），理由与不补
/// 用例的处置见模块文档「后端的取用点」一节。
///
/// `discard` 失败时直接返回，`tx` 随之析构回滚，记录因此留着——这就是「失败则不删记录」。
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

/// 第 7 步：查策略 → 按裁决决定是否铸造批准值 → 经 Gate 应用。
///
/// **这一支不清理工作区**：成功与被拒都不清理（设计第 4.2 节）。被拒时改动是完整可用的，
/// 丢弃它会让用户无从恢复；成功时也一样——集成把改动并进 Base 是一条独立的通道，
/// 收不收尾由第 8 步单独规定，而第 8 步只在「未给 `--apply`」时走。
///
/// **`backend` 一律取自记录**（设计第 4.4 节），与第 8 步同一处口径。
///
/// 铸造出的批准值只对**这一次**集成有效（[`approve_integration`] 的文档）：`apply_patch`
/// 在动第一个字节之前会重算摘要并比对，故「铸造」与「应用」之间若有人改了 Base 或 Task，
/// 集成会以 [`GateError::ApprovalMismatch`] 失败而不是把错的改动落下去。
fn apply_recorded_integration(db: &Db, args: &TaskArgs) -> Result<(), TaskError> {
    let tx = db.begin()?;
    let record = load_workspace(&tx, &args.intent)?.ok_or_else(|| TaskError::RecordMissing {
        intent: args.intent.as_str().to_owned(),
    })?;

    // `--approve` 给没给是裁决用的**事实**，也是 [`mints`] 用的**开关**。取一次存在
    // 局部变量里：两处若各取一次（或一处写成 `true`），「事实」与「开关」就会分岔，
    // 而分岔的两侧都编译得过。
    let approved = args.approve;
    let policies = load_policies(&tx)?;
    let ctx = PolicyContext {
        explicit_current: approved.then_some(ExplicitApproval),
        // 其余五事实本项目暂无来源（设计第 5.6 节：`task_class` 由驱动注入，但没规定
        // 注什么）。**填 `None` 而不是编一个值**：编出来的值会让引用它的规则开始匹配，
        // 而设计第 5.6 节点名这是本层最需要留意的变更风险。
        ..PolicyContext::default()
    };

    let decision = arbitrate(&policies, &ctx);
    if !mints(decision, approved) {
        // `tx` 随之析构回滚。此处本就没有写，回滚只是别把写锁留着。
        return Err(TaskError::IntegrationRefused {
            decision: decision_name(decision),
            path: record.path.display().to_string(),
        });
    }

    let base = BaseWorkspace::new(record.base_path.clone())?;
    let task = TaskWorkspace::new_outside(&base, record.path.clone(), record.intent_id.clone())?;
    let approval = approve_integration(&base, &task, record.backend)?;
    // 审计行由 Gate 在同一个事务里写（设计第 6.4 节），故事务在这里才提交。
    IntegrationGate::new(&base).apply_patch(&tx, &task, record.backend, now_millis(), &approval)?;
    tx.commit()?;
    Ok(())
}

/// 第 2 级 `ExplicitCurrent` 的那条规则：`--approve` 给出时成立的 `Allow`。
///
/// **它是内建的、不落库的规则**（设计第 5.2 节：第 2 级在下篇由驱动的显式确认占位，
/// P4 就位后移交），由 [`arbitrate`] 在裁决时放进给 `decide` 的列表里，**不是从 `policy`
/// 表读来的**。
///
/// # `scope` 对它无意义
///
/// [`Scope`] 只有 `User | Project`，而本规则既不属于用户也不属于项目——它是内建的。
/// 此处填 [`Scope::User`] 只是因为该字段必填，**不是「这条规则是用户的」**。
/// `decide` **不读 `scope`**（`continuum_policy::engine` 的文档：裁决只按层级与决策取严），
/// 故这个取值不影响任何裁决结果——这一条由 `continuum-policy` 自己的用例
/// `crates/continuum-policy/tests/arbitration.rs::scope_does_not_participate_in_arbitration`
/// 从两个方向钉住（同层内对调 `Scope` 结果不变）。要给它一个名副其实的取值须给 `Scope` 加变体，那是改
/// 一个跨 crate 的公开类型（`policy` 表的 `scope` 列还有落库编码），须另行裁定，
/// 不在这里就地扩。
///
/// # 条件必须有，且必须是「`explicit_current` 成立」
///
/// 一条**无条件**的第 2 级 `Allow` 会在 `--approve` 未给出时照样获胜，把第 3–5 级的
/// `Deny` 一律推翻，即 fail-open（设计第 5.5 节的取舍只在 `--approve` 给出时成立）。
/// 本函数的用例 `the_explicit_current_rule_allows_only_when_the_flag_is_given`
/// 从两侧钉住它：给出时 `Allow`、未给出时**不成立**。
fn explicit_current_rule() -> Policy {
    let condition = Condition::parse(&serde_json::json!({
        "all": [{"fact": "explicit_current", "eq": true}]
    }))
    // 字面量条件必然合法：事实名与比较符都在各自的封闭集合内，取值是布尔。
    // 写成 `expect` 而不是在运行期兜底——这条规则是编译期就定死的常量，
    // 它若解析不了，那是本文件写错了，不该在用户的调用上表现为一条策略静默失效。
    .expect("内建的第 2 级规则条件是合法谓词");
    Policy {
        level: Level::ExplicitCurrent,
        condition,
        decision: Decision::Allow,
        scope: Scope::User,
    }
}

/// 第 7 步的裁决：把第 2 级那条规则放进表里，**一次**裁决完。
///
/// 判定与 `decide` 分开写是刻意的（计划 Task 10 第 3 步）：`decide` 只裁决，而
/// 「`--approve` 能越过什么」这条规则在驱动这一侧，故「第 1 级不可越」有一个单独的落点，
/// 不会混进通用裁决里被顺手改掉。
///
/// **第 2 级那条规则必须留在表里**：把它排除在外、把「越过」留给 [`mints`] 去做，
/// 是设计第 5.7 节的另一种接法；本子项目在 Task 5 已裁定不这么做（理由见 [`mints`]，
/// 那边也是「照抄 §5.7 会 fail-open」的完整推导所在）。
fn arbitrate(policies: &[Policy], ctx: &PolicyContext) -> Decision {
    let mut table = policies.to_vec();
    table.push(explicit_current_rule());
    decide(&table, ctx)
}

/// 由**一次裁决的结果**决定是否铸造批准值（设计第 5.7 节的表，**本接法**）。
///
/// # 为什么不能照抄 §5.7 那张表
///
/// §5.7 的表对应的是另一种接法：裁决时**把第 2 级排除在外**，越过在第 5.7 节的映射里做。
/// 本子项目在 Task 5 已裁定采用**另一种接法**——第 2 级那条规则就留在传给 `decide` 的表里
/// （见 [`arbitrate`]）。两种接法在「铸造与否」上结果一致，但**返回的 `Decision` 不同**：
/// 例如第 3 级 `Deny` + `--approve`，接法 A 返回 `Deny`、本接法返回 `Allow`。
///
/// **照抄 §5.7 的 `Deny` 那一行（「有 `--approve` 才铸造」）是 fail-open**：本接法在
/// 「第 1 级 `Deny` + `--approve`」时也返回 `Deny`，照那行读就会铸造——而第 1 级是设计里
/// 唯一一条命令开关越不过的防线（第 5.5 节）。
///
/// # 本接法的映射
///
/// - `Allow` → 铸造。只可能来自「第 2 级那条规则成立」（即 `--approve` 已给出），
///   或第 1 级的一条 `Allow`（它高于第 2 级，且不拦任何东西）。
/// - `RequireApproval` → 有 `--approve` 才铸造。**这一支不是死代码**：`--approve` 未给出
///   时它来自第 3–5 级；**已给出时**它来自一条与第 2 级同层且更严的落库规则，或来自
///   第 1 级的一条 `RequireApproval`（第 1 级高于第 2 级）。故 `approved` 这个入参在
///   这一支上是可观察的，`mints` 的取值表逐格钉住它。
///
///   注意最后那句「不是死代码」说的是**这一整支**，不是 `approved == true` 那一格：
///   第 3–5 级的 `RequireApproval` 在 `--approve` 已给出时**确实**会被第 2 级的 `Allow`
///   越过而返回 `Allow`。剩下两条能返回 `RequireApproval` 的路（第 1 级、第 2 级同层
///   更严）各有照片，见
///   `a_require_approval_verdict_survives_the_flag_at_the_first_two_levels`——
///   没有那张照片，「已给出时它来自……」就是一句无对照的断言。
/// - `Deny` + `--approve` **未**给出 → 不铸造。
/// - `Deny` + `--approve` **已**给出 → 不铸造。第 2 级那条规则在表里时，`--approve`
///   已给出却仍返回 `Deny`，只可能出自第 1 级 `Deny`（它高于第 2 级），或出自
///   「调用方没把第 2 级规则放进表里」（此时 `Deny` 只是「没有更高的规则放行」）。
///   **两种都不该铸造。**
///
/// 「无任何规则匹配时默认 `Deny`」这一路（设计第 5.3 节）由此自动落到「给出 `--approve`
/// 才放行」：未给出即 `Deny` 不铸造，给出则那条第 2 级规则成立、裁决为 `Allow` 而铸造——
/// 与设计第 5.5 节的第三行一致。
fn mints(decision: Decision, approved: bool) -> bool {
    match decision {
        Decision::Allow => true,
        Decision::RequireApproval => approved,
        Decision::Deny => false,
    }
}

/// 裁决值的中文名，只用于错误信息（标识符仍是英文，见项目的语言口径）。
fn decision_name(decision: Decision) -> &'static str {
    match decision {
        Decision::Allow => "允许",
        Decision::RequireApproval => "要求批准",
        Decision::Deny => "禁止",
    }
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
    /// 策略裁决为「不铸造批准值」，故拒绝集成。
    ///
    /// **工作区与记录原样保留**（设计第 4.2 节）：命令成功了，改动是完整可用的，
    /// 丢弃它会让用户无从恢复。`path` 就是那份改动所在之处。
    ///
    /// `decision` 取 [`decision_name`] 的中文名，使调用方能分辨是「要求批准」还是
    /// 「禁止」——前者的处置是给出 `--approve` 重跑，后者（第 1 级）则无解。
    #[error("策略裁决为 {decision}，拒绝集成；改动仍在 Task 工作区 {path}，未丢弃")]
    IntegrationRefused {
        decision: &'static str,
        path: String,
    },
    /// Integration Gate 层的失败（批准值失配、后端拒绝、审计行落库失败）。
    #[error("Integration Gate 失败：{0}")]
    Gate(#[from] GateError),
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

#[cfg(test)]
mod tests {
    use super::*;

    /// 一条条件恒真（空合取）的规则：任何上下文都成立。
    ///
    /// 用它而不是带事实的条件：本模块要钉的是**裁决之后的映射**与**第 2 级那条规则
    /// 放进表里没有**，条件怎么求值是 `continuum-policy` 的范围（`tests/condition.rs`）。
    fn always(level: Level, decision: Decision) -> Policy {
        Policy {
            level,
            condition: Condition::parse(&serde_json::json!({"all": []}))
                .expect("空合取是合法条件"),
            decision,
            scope: Scope::User,
        }
    }

    /// `--approve` 已给出的上下文；其余五事实缺省（驱动暂无来源，见实现处的说明）。
    fn approved() -> PolicyContext {
        PolicyContext {
            explicit_current: Some(ExplicitApproval),
            ..PolicyContext::default()
        }
    }

    /// `--approve` 未给出的上下文。
    fn unapproved() -> PolicyContext {
        PolicyContext::default()
    }

    /// 内建的那条第 2 级规则：只在 `--approve` 给出时成立，且它确实是第 2 级的 `Allow`。
    ///
    /// **两侧都要**：只断言「给出时 `Allow`」的话，一条**无条件**的第 2 级 `Allow`
    /// 同样通过，而它会把第 3–5 级的 `Deny` 一律推翻——正是设计第 5.5 节点名的 fail-open。
    #[test]
    fn the_explicit_current_rule_allows_only_when_the_flag_is_given() {
        let rule = explicit_current_rule();
        assert_eq!(rule.level, Level::ExplicitCurrent, "这条规则必须占第 2 级");
        assert_eq!(rule.decision, Decision::Allow, "`--approve` 的裁决是放行");

        assert_eq!(
            decide(std::slice::from_ref(&rule), &approved()),
            Decision::Allow,
            "--approve 给出时第 2 级规则应成立"
        );
        assert_eq!(
            decide(std::slice::from_ref(&rule), &unapproved()),
            Decision::Deny,
            "无条件成立即 fail-open：--approve 未给出时它必须不成立（无匹配默认 Deny）"
        );
    }

    /// 裁决值 → 是否铸造的**六格取值表**（三决策 × `--approve` 给没给）。
    ///
    /// 逐格写死，不用 `Decision::ALL` 遍历：遍历只证明「每个变体各占一格」，
    /// 证明不了格子的取值对不对——而这张表正是「照抄设计第 5.7 节会 fail-open」的落点。
    #[test]
    fn the_mapping_from_a_decision_to_minting_has_six_cells() {
        // 第 5 级 `Allow` 不必 `--approve`（设计第 5.7 节第一行）。
        assert!(mints(Decision::Allow, false), "Allow → 铸造，与 --approve 无关");
        assert!(mints(Decision::Allow, true), "Allow → 铸造，与 --approve 无关");
        // 要求批准：给出才铸造。
        assert!(
            !mints(Decision::RequireApproval, false),
            "RequireApproval 而未给出 --approve → 不铸造"
        );
        assert!(
            mints(Decision::RequireApproval, true),
            "RequireApproval 且已给出 --approve → 铸造；这一格非空，理由见 `mints` 的文档"
        );
        // 禁止：两个方向都不铸造。
        assert!(!mints(Decision::Deny, false), "Deny 而未给出 --approve → 不铸造");
        assert!(
            !mints(Decision::Deny, true),
            "Deny 且已给出 --approve → 仍不铸造：此时它只可能出自第 1 级，\
             或出自调用方没把第 2 级规则放进表里"
        );
    }

    /// 第 1 级 `Deny` 越不过 `--approve`，而第 3 级 `Deny` 越得过——**两个方向同一条用例**。
    ///
    /// 这正是设计第 9 节「`--approve` 的边界」那一行，也是本项目要求「缺一不可」的那对。
    /// 缺了下半段，一个**恒不铸造**的实现也能通过上半段；缺了上半段，照抄 §5.7 的
    /// 「有 `--approve` 才铸造」在 fail-open 方向上通过。
    #[test]
    fn a_system_safety_deny_is_not_overridden_by_the_flag() {
        let system_deny = vec![always(Level::SystemSafety, Decision::Deny)];
        let user_deny = vec![always(Level::UserPersistent, Decision::Deny)];

        let decision = arbitrate(&system_deny, &approved());
        assert_eq!(
            decision,
            Decision::Deny,
            "第 1 级高于第 2 级，--approve 已给出也仍返回 Deny"
        );
        assert!(!mints(decision, true), "第 1 级 Deny 一律不铸造");

        let decision = arbitrate(&user_deny, &approved());
        assert_eq!(
            decision,
            Decision::Allow,
            "第 2 级的 Allow 高于第 3 级的 Deny，--approve 已给出时返回 Allow"
        );
        assert!(mints(decision, true), "第 3 级 Deny 可由 --approve 越过");
    }

    /// 第 3–5 级的 `RequireApproval`：给出 `--approve` 才放行（设计第 5.7 节第二行）。
    #[test]
    fn a_require_approval_rule_needs_the_flag() {
        let table = vec![always(Level::UserPersistent, Decision::RequireApproval)];

        let decision = arbitrate(&table, &unapproved());
        assert_eq!(decision, Decision::RequireApproval);
        assert!(!mints(decision, false), "未给出 --approve 时要求批准即拒绝");

        let decision = arbitrate(&table, &approved());
        assert_eq!(
            decision,
            Decision::Allow,
            "给出 --approve 后第 2 级的 Allow 压过第 3 级的 RequireApproval"
        );
        assert!(mints(decision, true));
    }

    /// `--approve` **已给出**时仍返回 `RequireApproval` 的两条路，各一张照片。
    ///
    /// 这是 `mints` 文档里「`RequireApproval` 那一支不是死代码」的落点，也纠正一个
    /// 容易写错的绝对说法：给出 `--approve` 后**只有第 3–5 级**的 `RequireApproval`
    /// 会被第 2 级的 `Allow` 越过；下面两条路照样返回 `RequireApproval`，
    /// 故 `mints(RequireApproval, true)` 这一格可观察。
    ///
    /// 两条路都必须**真能造出来**：`save_policy` 不校验层级来源（`persist.rs`），
    /// `Level` 六个变体都在封闭集合内（`rule.rs`），故「库里只存第 3–4 级」不是被
    /// 强制的不变量——本 task 的端到端用例自己就往库里放第 5 级规则。
    #[test]
    fn a_require_approval_verdict_survives_the_flag_at_the_first_two_levels() {
        // 路一：第 1 级 `RequireApproval`。第 1 级高于第 2 级，故 `--approve` 越不过。
        let first_level = vec![always(Level::SystemSafety, Decision::RequireApproval)];
        assert_eq!(
            arbitrate(&first_level, &approved()),
            Decision::RequireApproval,
            "第 1 级高于第 2 级的 Allow，--approve 已给出也仍返回 RequireApproval"
        );
        assert!(
            mints(Decision::RequireApproval, true),
            "这一格可达：第 1 级的 RequireApproval 给出 --approve 也要铸造"
        );

        // 路二：一条与第 2 级同层、更严的落库规则。同层取更严，压过内建的 Allow。
        let same_level = vec![always(Level::ExplicitCurrent, Decision::RequireApproval)];
        assert_eq!(
            arbitrate(&same_level, &approved()),
            Decision::RequireApproval,
            "同层取更严：RequireApproval 压过内建的 Allow"
        );
        assert!(mints(Decision::RequireApproval, true));
    }

    /// 第 5 级 `Runtime Default` 的 `Allow`：不传 `--approve` 也放行。
    #[test]
    fn a_runtime_default_allow_mints_without_the_flag() {
        let table = vec![always(Level::RuntimeDefault, Decision::Allow)];
        let decision = arbitrate(&table, &unapproved());
        assert_eq!(decision, Decision::Allow);
        assert!(mints(decision, false), "库里放一条第 5 级 Allow 即可免 --approve 集成");
    }

    /// 无任何规则匹配：默认 `Deny`，但给出 `--approve` 仍放行（设计第 5.5 节第三行）。
    ///
    /// 这一支由第 2 级那条规则**自动**落到位，故它是「规则确实被放进表里」的又一张照片。
    #[test]
    fn with_no_rule_at_all_the_flag_still_decides() {
        let empty: Vec<Policy> = Vec::new();

        let decision = arbitrate(&empty, &unapproved());
        assert_eq!(decision, Decision::Deny, "无匹配默认 Deny（设计第 5.3 节）");
        assert!(!mints(decision, false), "无匹配且未给出 --approve → 不铸造");

        let decision = arbitrate(&empty, &approved());
        assert_eq!(
            decision,
            Decision::Allow,
            "无匹配但给出 --approve：第 2 级那条规则成立，故放行"
        );
        assert!(mints(decision, true));
    }
}
