//! `task` 子命令的实现（设计下篇第 4.2 节第 1–8 步）。
//!
//! # 序列（设计第 4.2 节）
//!
//! 1. `detect_backend`；
//! 2. 若为 overlay 且不在用户命名空间内 → 把**自身**经 `unshare -Urm` 重新执行，
//!    环境变量 [`IN_NAMESPACE_ENV`] 标识已进入以防递归。**这一步排在创建任何东西之前**，
//!    否则会建出一个子进程看不见的工作区（设计上篇第 5 节）；
//! 3. `create_task_workspace` → `WorkspaceRecord::from_workspaces` → `save_workspace`；
//! 4. **先查一次策略**判本次集成（`decision`，第 7 步复用）；查各 `--effect` 的幂等键；
//!    **再逐条问策略铸能力**——铸不出即拒绝整条命令（**强制点 (2)**，
//!    [`authorize_declared_effects`]）；最后对每个 `--effect` 写
//!    `PLANNED → AUTHORIZED → EXECUTING`（[`record_declared_effects`]）。全部**提交之后**
//!    才执行命令（`§268`：执行前写入）；
//! 5. `Sandbox::spawn` 跑 `--exec`，工作目录由 `spawn` 设为 Task 根；
//! 6. 按命令的退出形态写终态：退出码 0 → `COMMITTED`，非 0 → `FAILED`
//!    （[`finish_declared_effects`]）；
//! 7. **命令成功且给了 `--apply`**：按第 4 步那次裁决决定是否铸造批准值
//!    （[`mints`]）→ 经 Gate 应用。**这一支不清理工作区**（见下）；
//! 8. **未给 `--apply`，或命令退出码非 0**：`discard_task_workspace` 成功之后再
//!    `remove_workspace`，失败则不删记录。
//!
//! # 强制点 (2)：执行之前的逐条效应校验（设计第 4.1、4.2 节）
//!
//! 驱动按「命令即效应」运行命令，**命令跑起来就是副作用**，故校验排在 `Sandbox::spawn`
//! 之前。检查对象是命令声明的那几条 `--effect`：**逐条**问策略，据裁决铸
//! [`Capability`](continuum_capability::Capability)；**任意一条铸不出即拒绝整条命令**
//! （不是只拒绝那一条）。
//!
//! ## 与「策略只查一次」不冲突：问的是两个问题
//!
//! P2 那条裁定（见上文「策略只查一次」）的对象是**集成**的裁决：`--apply` 那一支第 7 步
//! 复用第 4 步的那一次，上下文里 `effect_type` 缺省。**本处是逐条效应的裁决**，上下文里
//! `effect_type` 填着这一条效应的类型（[`policy_context_for_effect`]）。两者问的不是同一
//! 件事——「这次集成准不准」对「这条效应准不准」——两处各自裁一次、互不复用同一个
//! [`Decision`]。**策略表只从库里读一次**（第 4 步那一读），两个问题用同一张表各裁一次；
//! 「只查一次」要防的是同一个裁决有两个产生点，不是同一张表被问两个问题。
//!
//! ## 裁决到「铸不铸」的映射只有一个产生点
//!
//! 三个臂（`Allow` / `RequireApproval` / `Deny`）的判定**复用** [`mints`]——那张六格表的
//! 唯一落点。本处**不重写**它，也不引入 `Verdict` / `Grant` 之类的中间裁决类型
//! （Task 2 已按「签发点不重判」删除它们）：映射的产物就是「铸出的一枚
//! [`Capability`](continuum_capability::Capability)，或一次拒绝」。
//!
//! ## 铸出的能力：持有至命令结束，随后丢弃
//!
//! 每条声明铸一枚，装进 [`AuthorizedEffect`]（效应 + 准它的能力）由 [`run`] 的局部变量
//! 持有，直到 [`run`] 返回才随作用域析构。**今天没有消费方**——消费方是连接器
//! （子项目 B），由它在做副作用时收下（设计 §4.2、§10 第 7 条）。**丢弃不是遗留物**：
//! 这枚值的存在本身就是「这条效应过了校验」的载体；驱动不要拿它去做别的（尤其**不得**
//! 把它或其内容塞进命令的环境——§51 要求 Agent 不直接拿到凭据）。
//!
//! ## 拒绝时的工作区清理：走第 4 步已有的那条路
//!
//! 本校验排在**写效应记录之前**，故拒绝时一条 `EXECUTING` 记录都还没落库；[`run`] 走的
//! 是第 4 步出错时的同一条清理路径（[`discard_recorded_workspace`]），不另立一条。
//! **这一条位置是刻意的**：若把校验排在第 4 步提交**之后**，那几条 `EXECUTING` 记录已经
//! 落库，而本仓没有任何删除 `effect` 行的路径（`grep 'DELETE FROM effect' crates/` 无命中
//! ——本仓确有别的 `DELETE`，如 `continuum-workspace/src/persist.rs` 删 `workspace` 行，
//! 故引用必须限定到 `effect` 表才成立；[`IntegrationGate::discard`] 只回收工作区、不碰
//! Journal，见 `continuum-workspace/src/gate.rs`）——拒绝之后同一个 Intent 会因幂等键
//! 再也创建不了，正是第 4 步错误路径要防的那件事。
//!
//! # 策略只查一次，排在写效应之前
//!
//! 设计第 6.7 节要求 `authorization` 写入「批准值是否给出、以及策略的裁决结果」，
//! 第 6.3 节把写 `PLANNED`（含 `authorization`）排在第 4 步，而第 4.2 节把「查策略」
//! 排在第 7 步（命令之后）——三处里第 4.2 节的第 7 步是孤立者。**本实现以第 6.3 节
//! 为准**：裁决在 [`record_declared_effects`] 里做一次，第 7 步复用同一个 [`Decision`]，
//! 不再第二次查。两条理由：
//!
//! - `AUTHORIZED` 这个状态名应当真的意味着「策略已授权」，而不是「稍后会授权」；
//! - **单一来源**——查两次会让同一个裁决有两个产生点，两处可以各自漂移。
//!
//! **故策略裁决早于命令执行。** 今天这没有可观察的差别：`PolicyContext` 除
//! `explicit_current` 外全是 `None`（设计第 5.6 节），早查与晚查给出同一结果。**将来若
//! `duration_ms` 之类真的被注入，这一点要重新审视**——命令的时长只有跑完才知道，那样的
//! 裁决必须晚于命令，本节、第 6.3 节与第 4.2 节随之重写。
//!
//! # `authorization` 只记录、不校验（设计第 6.7 节）
//!
//! 见 [`authorization_field`]。**此边界在此显式声明**，否则该字段会被读成「此处已强制」。
//!
//! # `--effect` 的落库取值
//!
//! - `Effect.id` 与 `idempotency_key` **同源**：同一三元组（意图 id / 类型 / 目标）、
//!   同一个函数 [`effect_key`]，不是一个字段各派一次。设计第 6.5 节只规定幂等键由该
//!   三元组派生；`id` 是表的主键（第 8 节），同样用它派生，不给「这条记录是谁」再立
//!   第二个来源；
//! - `parameters` 填 `{}`：设计第 6.1 节只说它是 JSON，**未规定内容，故本子项目不填**，
//!   不编造值；
//! - `planned_at` / `updated_at` 取驱动自取的 [`now_millis`]——effect crate 不收时钟
//!   （`continuum_effect::recovery` 的 `now` 同样由调用方给），这一刻由驱动给。
//!
//! # 幂等键先查、已存在即拒绝整条命令
//!
//! 执行任何命令之前先查键（第 6.5 节）。**拒绝而非静默跳过**：静默跳过会让调用方以为
//! 命令执行了。任意一条 `--effect` 的键已存在即拒绝**整条**命令——不是只跳过那一条。
//! 拒绝时命令一步都没跑，工作区与记录按第 8 步的次序清理，否则同一个 Intent 再也创建不了。
//!
//! # 第 7、8 步的两种收尾**不能类推**（设计第 4.2 节）
//!
//! - **第 7 步裁决为「不铸造」时（含第 1 级 `Deny`）：拒绝集成，但保留工作区**，不
//!   `discard`。命令**成功**了，用户的改动是完整可用的，丢弃它才叫「让用户无从恢复」。
//!   **但「保留」不等于「可经本驱动重跑」**：第二次运行的第 3 步
//!   `create_task_workspace` 必然失败——worktree 后端撞 `fatal: a branch named
//!   '<分支>' already exists`，overlay 后端显式拒绝已存在的 Intent 目录；即便跨过
//!   第 3 步，第一次声明过的 `--effect` 的幂等键也会在第 4 步再拒一次。故保留的实际
//!   含义是**用户进该 Task 工作区自行处理（手工 git）**；本驱动目前没有重跑路径。
//! - **第 8 步反过来：命令失败也照旧清理**。这一支留下的是**半成品**，而更要紧的是
//!   **记录**——[`save_workspace`] 是裸 `INSERT`（`continuum-workspace/src/persist.rs`），
//!   保留记录会让**同一个 Intent 再也创建不了**，用户得手工删记录与分支才能重试。
//!   故「保留工作区」的代价不止一个目录，只在「改动完整且未被批准」那一支才值得付。
//!
//! 同理，**命令退出码非 0 时不做集成**（第 7 步不走）：第 6 步把各 Effect 记 `FAILED`，
//! 与「同一件事既记为失败、又把它的文件系统结果收进 Base」自相矛盾。
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
use continuum_capability::{AuthorizedEffect, CapabilityKind, mint};
use continuum_effect::{
    Effect, EffectId, EffectState, EffectType, advance, find_by_idempotency_key, record_planned,
};
use continuum_persist::{Db, PersistError};
use continuum_policy::{
    Condition, Decision, ExplicitApproval, Level, Policy, PolicyContext, Scope, decide,
    load_policies,
};
use continuum_runtime::cli::{EffectSpec, TaskArgs};
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

/// 强制点 (2) 铸出的能力活多久（Unix 毫秒），即签发时取 `now_millis() + 本值`。
///
/// §51 要求凭据「short-lived」，而**规范与设计都没有规定这个数**——它是本阶段的决定，
/// 集中在此一处、不散落。取值理由：
///
/// - 这枚能力只需覆盖**本条命令的执行**，以及由它签出的凭据（设计 §5.2：凭据的到期
///   不晚于能力的 `expiry`，故能力的寿命是那个上限）；
/// - 15 分钟对一条任务命令是宽的余量，又远短于长期令牌，符合「short-lived」的意；
/// - **驱动今天不自己校时**（[`mint`] 不读时钟，见其文档），故这个数在本次运行中不可
///   观察：一条跑得比它久的命令不会在运行中被拦下，而是让下游签出的凭据更早到期——
///   那是 fail-closed 的方向。
///
/// 连接器（子项目 B）真的消费 [`AuthorizedEffect`] 时，这个数应按那时能给出的依据
/// （命令的实际时长上界、凭据源的轮换周期）重新定；届时也只改这一处。
const CAPABILITY_LIFETIME_MS: i64 = 15 * 60 * 1000;

/// 运行 `task` 子命令。
///
/// `argv` 是**本次调用的原始参数**（不含 argv[0]），第 2 步重新执行自身时原样转交：
/// 从解析结果重建命令行会丢掉调用方实际写的形状（同一个值可以有多种写法），而 re-exec
/// 要的正是「把这一模一样的调用再跑一遍，只是换进命名空间里」。
pub fn run(args: &TaskArgs, argv: &[OsString]) -> Result<(), TaskError> {
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
        return Err(match discard_unrecorded_workspace(&db, &base, &task, created_backend) {
            Ok(()) => e,
            Err(cleanup) => TaskError::Context {
                context: format!("落库失败（{e}）之后，放弃本次建出的工作区也失败"),
                source: Box::new(cleanup),
            },
        });
    }

    // 第 4 步：查一次策略、逐条效应校验（强制点 (2)），再写各效应的
    // `PLANNED → AUTHORIZED → EXECUTING`。失败（幂等键已存在、某条效应铸不出能力、
    // 库层错误）时命令一步都没跑，故工作区与记录按第 8 步的次序清理——留着记录会让
    // 同一个 Intent 再也创建不了（`save_workspace` 是裸 `INSERT`）。
    //
    // 第 4 步返回的批准值只给第 7 步用（`decision`）；铸出的各枚能力由 `_authorized`
    // 持有着，直到本次运行结束才随作用域析构——消费方是连接器（子项目 B），
    // 见模块文档「铸出的能力」。
    let (decision, _authorized) = match record_declared_effects(&db, args) {
        Ok(result) => result,
        Err(e) => {
            return Err(match discard_recorded_workspace(&db, &args.intent) {
                Ok(()) => e,
                Err(cleanup) => TaskError::Context {
                    context: format!("第 4 步（效应登记）失败（{e}）之后，清理工作区也失败"),
                    source: Box::new(cleanup),
                },
            });
        }
    };

    // 第 5 步：在沙箱内执行。选中的机制不另作报告——驱动不在正常路径上写 stdout/stderr，
    // 子进程自己的输出即是调用方看到的东西；返回它只为装配与纯函数的答案能相互核对。
    //
    // 机制选择失败也算「执行没成」（第 6 步据此记 `FAILED`），走同一条收尾：工作区与
    // 记录都不留。留下的代价不只是一个目录——`save_workspace` 不覆盖同名记录，同一
    // Intent 会因此再也创建不了。
    let execution = match sandbox_select::select_for_this_machine(args.sandbox) {
        Ok((_mechanism, sandbox)) => run_in_sandbox(&sandbox, &task, &args.exec),
        Err(e) => Err(TaskError::from(e)),
    };

    // 第 6 步：按命令的退出形态写终态（设计第 6.3 节）。三种出口都由退出形态判定，
    // 不需要知道命令内部做了什么。**写终态排在集成之前**：`COMMITTED` 说的是命令跑成了，
    // 与第 7 步集不集成是两回事。
    //
    // 终态写不进去意味着**这次运行的结局没有被记录**。此时集成，等于把 Base 的改动挂在
    // 一个没有记录的授权上——与「记为 `FAILED` 却把结果收进 Base」是同一类自相矛盾。
    // 故不集成、并按第 8 步收尾，把失败报出去（这一支与「命令退出码非 0 时不集成」同源，
    // 理由不是「fail-closed」而是「结局没有记录」）。
    //
    // 按第 8 步清理在这一支上无需另设分支：DB 写失败时 `remove_workspace` 多半也会失败，
    // 于是实际退化为「工作区与记录都保留」——两条路在这里自然收敛。
    let terminal = if execution.is_ok() {
        EffectState::Committed
    } else {
        EffectState::Failed
    };
    if let Err(e) = finish_declared_effects(&db, args, terminal) {
        return Err(match discard_recorded_workspace(&db, &args.intent) {
            Ok(()) => e,
            Err(cleanup) => TaskError::Context {
                context: format!("效应记录的终态写入失败（{e}）之后，清理工作区也失败"),
                source: Box::new(cleanup),
            },
        });
    }

    // 第 7 步：命令**成功**且给了 `--apply` → 按第 4 步那次裁决决定是否集成。
    // **这一支不清理工作区**，成功与被拒都不清理（设计第 4.2 节）。命令失败时第 7 步
    // 不走（同上）。
    if args.apply && execution.is_ok() {
        // 这层上下文**只**记「哪一步、哪个 Intent」，不替错误断言工作区还在不在：
        // 第 7 步的错误不都是那一类——[`TaskError::RecordMissing`] 的含义恰恰是
        // **记录已经不在了**，「记录原样保留」对它为假；而 `Gate` 那一支「工作区保留」
        // 成立却没被论证过。确实成立的那一支（裁决不铸造）把保证写在
        // [`TaskError::IntegrationRefused`] 自己的 `Display` 里，更靠近它成立的地方。
        return apply_recorded_integration(&db, args, decision).map_err(|e| TaskError::Context {
            context: format!(
                "第 7 步（策略裁决与集成）失败（Intent {}）",
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

/// 第 4 步：查一次策略（集成的裁决）、逐条效应校验（强制点 (2)），再对每个 `--effect`
/// 写 `PLANNED → AUTHORIZED → EXECUTING`。
///
/// 返回本次调用**唯一**的那次集成裁决（供第 7 步复用，见模块文档「策略只查一次」）与
/// 各条效应铸出的能力（[`AuthorizedEffect`]，见模块文档「铸出的能力」）。
///
/// # 为什么先查全部键、再写任何一条
///
/// 任意一条 `--effect` 的幂等键已存在即拒绝**整条**命令（设计第 6.5 节）。两趟走——
/// 先全查、后全写——使「拒绝」不依赖回滚来保证「一条也没写」，读代码时不必推演事务。
/// **幂等键的检查排在强制点 (2) 之前**：它不依赖任何裁决，且是更具体的前置条件；
/// 排在前面也让「同键的第二次调用」报的仍是幂等键冲突（P2 的既有判据）。
///
/// # 强制点 (2) 排在写效应之前
///
/// 铸不出能力即 `Err` 返回，此时一条记录都还没写（事务随作用域回滚，也无需回滚）。排在
/// 写之前是刻意的：本仓没有删除 `effect` 行的路径，若记录已落库再拒绝，同一个 Intent
/// 会因幂等键再也创建不了（模块文档「拒绝时的工作区清理」）。
///
/// # 事务在返回前提交
///
/// 第 5 步启动子进程前必须结束事务，否则 `BEGIN IMMEDIATE` 会在子进程运行的全程占着
/// 写锁。更要紧的是**记录要先落盘**：进程若在命令执行中被杀，未提交的事务会随进程
/// 消失，恢复钩子就看不到那条 `EXECUTING`，`§268` 的「执行前写入」也就无从谈起。
///
/// 出错时 `tx` 随作用域析构回滚，调用方（[`run`]）随后清理工作区与记录。
fn record_declared_effects(
    db: &Db,
    args: &TaskArgs,
) -> Result<(Decision, Vec<AuthorizedEffect>), TaskError> {
    let tx = db.begin()?;

    // 集成那条裁决在本步做，不留给第 7 步：`AUTHORIZED` 要真的意味着「策略已授权」，
    // 而 `authorization` 字段（第 6.7 节）要写进这个裁决结果。
    let policies = load_policies(&tx)?;
    let decision = arbitrate(&policies, &policy_context(args.approve));
    let authorization = authorization_field(args.approve, decision);

    // 先查全部幂等键，再写任何一条。
    for spec in &args.effects {
        let key = effect_key(&args.intent, spec);
        if find_by_idempotency_key(&tx, &key)?.is_some() {
            return Err(TaskError::EffectAlreadyRecorded { key });
        }
    }

    // 强制点 (2)：逐条问策略、铸能力；铸不出即拒绝整条命令。与上面那次集成裁决共用
    // 同一张已读出的策略表，但**各裁一次**（上下文不同，见模块文档）。
    let authorized = authorize_declared_effects(&policies, args)?;

    for spec in &args.effects {
        let key = effect_key(&args.intent, spec);
        let now = now_millis();
        let effect = Effect {
            // `id` 与 `idempotency_key` 同源：见模块文档「`--effect` 的落库取值」。
            id: EffectId::new(key.clone()),
            effect_type: spec.effect_type,
            target: spec.target.clone(),
            // 本子项目不填 parameters（设计第 6.1 节未规定内容），故填空对象而不是
            // 编一个值：编出来的值会变成下游读得懂、而实际无意义的输入。
            parameters: serde_json::json!({}),
            authorization: authorization.clone(),
            idempotency_key: key,
            state: EffectState::Planned,
            planned_at: now,
            updated_at: now,
        };
        record_planned(&tx, &effect)?;
        // 三态一次写完（设计第 6.3 节）。每次 `advance` 在同一事务内追加一条审计。
        advance(&tx, &effect.id, EffectState::Authorized, now_millis())?;
        advance(&tx, &effect.id, EffectState::Executing, now_millis())?;
    }

    tx.commit()?;
    Ok((decision, authorized))
}

/// **强制点 (2)**：逐条问策略，铸不出能力即拒绝整条命令（设计第 4.1 节）。
///
/// 每条 `--effect` 用它自己的 [`policy_context_for_effect`]（`effect_type` 已填）裁决
/// 一次；「铸不铸」复用 [`mints`]（六格表唯一的落点，见其文档），**不重写**。铸得出就
/// 用 [`CapabilityKind::for_effect`] 取 kind、以该效应的**目标**为作用域铸一枚能力
/// （§253 的 `git.push:origin/main` 即此形），并配成 [`AuthorizedEffect`]。
///
/// # 拒绝的是**整条**命令
///
/// 按声明次序逐条走，遇到第一条铸不出的即返回 [`TaskError::EffectNotAuthorized`]——
/// 不是只跳过那一条。命令一步都没跑，调用方（[`record_declared_effects`] 的 `tx`）回滚，
/// 上层再清理工作区。
///
/// # 两个 `expect` 的理由
///
/// [`mint`] 唯一的失败是空作用域，而 `--effect` 的空目标在解析期即被拒
/// （`continuum_runtime::cli` 的 `EffectWithEmptyTarget`）；[`AuthorizedEffect::new`]
/// 唯一的失败是效应与能力的 kind 不对应，而这里的 kind 正是由同一条效应经
/// [`CapabilityKind::for_effect`] 派生的。两者都是本文件内部的不变量，写法与
/// [`run_in_sandbox`] 的「cli 已保证」同例，不在用户调用上兜底。
///
/// # 不读时钟
///
/// [`mint`] 不读时钟，`expiry` 由调用方给；这里取 [`now_millis`] 加
/// [`CAPABILITY_LIFETIME_MS`]。铸出的能力**不在这里做时效校验**——`
/// Capability::is_valid_at` 是时效判定的唯一产生点，由消费方（连接器 / 凭据签发）
/// 在用它之前判。
fn authorize_declared_effects(
    policies: &[Policy],
    args: &TaskArgs,
) -> Result<Vec<AuthorizedEffect>, TaskError> {
    let mut authorized = Vec::with_capacity(args.effects.len());
    for spec in &args.effects {
        let decision = arbitrate(
            policies,
            &policy_context_for_effect(args.approve, spec.effect_type),
        );
        if !mints(decision, args.approve) {
            return Err(TaskError::EffectNotAuthorized {
                effect: spec.effect_type.as_str(),
                target: spec.target.clone(),
                decision: decision_name(decision),
            });
        }
        let capability = mint(
            CapabilityKind::for_effect(spec.effect_type),
            spec.target.clone(),
            now_millis() + CAPABILITY_LIFETIME_MS,
        )
        .expect("cli 已保证 --effect 的目标非空，mint 不会失败");
        authorized.push(
            AuthorizedEffect::new(spec.effect_type, capability)
                .expect("kind 由 for_effect 从同一条效应派生，配对必然成立"),
        );
    }
    Ok(authorized)
}

/// 第 6 步：把各效应记为命令退出形态对应的终态。
///
/// `to` 只可能是 [`EffectState::Committed`]（退出码 0）或 [`EffectState::Failed`]
/// （非 0）——由 [`run`] 一处判定。崩溃那一支不走这里：进程都没了，记录停在
/// `EXECUTING`，由恢复钩子转 `UNKNOWN`（设计第 6.3、6.4 节）。
///
/// 没有声明任何 `--effect` 时不碰库：一个空事务没有意义，也会让「无效应的命令」
/// 凭空多一次写锁。
fn finish_declared_effects(
    db: &Db,
    args: &TaskArgs,
    to: EffectState,
) -> Result<(), TaskError> {
    if args.effects.is_empty() {
        return Ok(());
    }
    let tx = db.begin()?;
    for spec in &args.effects {
        // 记录的 `id` 与幂等键同源（[`effect_key`]），故这里由同一次派生取回 id。
        let id = EffectId::new(effect_key(&args.intent, spec));
        advance(&tx, &id, to, now_millis())?;
    }
    tx.commit()?;
    Ok(())
}

/// 效应的身份与幂等键（设计第 6.1、6.5 节）：由 意图 id / 类型 / 目标 派生。
///
/// **`Effect.id` 与 `idempotency_key` 取同一个值**——同一三元组、同一个函数，
/// 不是一个字段各派一次。设计第 6.5 节只规定幂等键由该三元组派生；`id` 是表的主键
/// （第 8 节），本子项目同样用它派生，以免给「这条记录是谁」再立第二个来源。
///
/// # 为什么带长度前缀
///
/// `intent` 与 `target` 都是自由文本（`IntentId::new` 只收字符串，不做校验），只靠
/// 分隔符拼接不是单射：`("a", publish, "b:publish:c")` 与 `("a:publish:b", publish, "c")`
/// 会拼出同一个串，两条不同的声明被当成同一条，第二条被静默拒绝。长度前缀让三段的
/// 分界可判定——键形如 `<意图字节长>:<意图>:<类型>:<目标>`，类型取自封闭枚举、不含
/// 冒号，故目标即最后一段的全部。
fn effect_key(intent: &IntentId, spec: &EffectSpec) -> String {
    format!(
        "{}:{}:{}:{}",
        intent.as_str().len(),
        intent.as_str(),
        spec.effect_type.as_str(),
        spec.target
    )
}

/// `authorization` 字段的内容（设计第 6.7 节）：**只记录、不校验**的不透明串。
///
/// 写入两件事：`--approve` 是否给出、以及策略的裁决结果。裁决取 [`Decision::as_str`]
/// 的编码——与 `policy` 表同一份，不在这里手抄字面量。
///
/// # 本子项目到此为止
///
/// **生产代码**不读回、不校验本字段（测试会读它，以钉住写入的内容与格式）。它是留给
/// 对账与审计的记录，不是一道强制。**本字段记的仍是集成那次裁决**（[`policy_context`]，
/// `effect_type` 缺省），与强制点 (2) 逐条效应的裁决（[`policy_context_for_effect`]，
/// `effect_type` 已填）是两个问题、两处各裁一次。**P3 起驱动确实有了 Capability 的
/// 输入**（强制点 (2) 已接上，见模块文档），但那条路径**不读回本字段**、也不靠它——
/// 本字段本身仍不被任何生产代码校验。校验属 Capability（P3）与 Authority（长期）的
/// 职责，其中 Capability 那一半落在 [`authorize_declared_effects`]。
/// **不要把本函数或这个字段读成「此处已强制」**（设计第 6.7 节要求显式声明此边界）。
fn authorization_field(approved: bool, decision: Decision) -> String {
    format!("approve={approved};policy={}", decision.as_str())
}

/// 裁决用的上下文。`--approve` 给没给是唯一有来源的事实；其余五事实本项目暂无来源
/// （设计第 5.6 节：`task_class` 由驱动注入，但没规定注什么），填 `None` 而不是编一个
/// 值——编出来的值会让引用它的规则开始匹配，那正是第 5.6 节点名的变更风险。
///
/// **这是集成那次裁决的上下文**（第 4 步的 `decision`，第 7 步复用），`effect_type` 缺省。
/// 逐条效应那次用 [`policy_context_for_effect`]——两者问的不是同一件事，见模块文档
/// 「与『策略只查一次』不冲突」。
fn policy_context(approved: bool) -> PolicyContext {
    PolicyContext {
        explicit_current: approved.then_some(ExplicitApproval),
        ..PolicyContext::default()
    }
}

/// 逐条效应那次裁决的上下文（强制点 (2)）。
///
/// 与 [`policy_context`] 的唯一差别是 `effect_type` 填着这一条效应的类型——那是
/// [`PolicyContext::effect_type`] 这个字段存在的理由（其文档原文：「本次待记的效应
/// 类型」）。**其余四个事实同样缺省**，理由与 [`policy_context`] 相同。
///
/// 这个字段可观察：一条按 `{"fact":"effect_type","eq":"charge"}` 限定的规则只在
/// `--effect` 为 `charge` 时成立，而集成那次裁决（`effect_type` 缺省）不会被它匹配。
fn policy_context_for_effect(approved: bool, effect_type: EffectType) -> PolicyContext {
    PolicyContext {
        explicit_current: approved.then_some(ExplicitApproval),
        effect_type: Some(effect_type),
        ..PolicyContext::default()
    }
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
/// **放弃经 [`IntegrationGate::discard`]，不直接调 [`discard_task_workspace`]**（设计上篇
/// 第 6.4 节）：`discard` 与那三个写入 Base 的操作一样要写一条审计记录，而**层在 Gate**
/// ——收下事务、在回收成功之后补记 `AuditKind::ExternalEffects` 那一行的都是它。绕过它
/// 走底层函数，每次清理都会少一条设计要求的审计记录（本 task 之前正是如此，见
/// `p2-followups.md`）。Gate 内部仍调 [`discard_task_workspace`]，故 overlay 的
/// 「`umount` 未成功就绝不往下走」那几条守卫不被旁路。
///
/// **`backend` 取自记录**（设计第 4.4 节）：记录里写的是哪个后端，`discard` 就按哪个后端
/// 回收。这一项有判别力——把它换成写死的某个后端，worktree 与 overlay 两个方向都会以
/// 另一条错误失败（见 `tests/task_cli.rs` 的
/// `a_failed_discard_keeps_the_workspace_record` 与 M5）。同一处取出的 `base_path` 与
/// `path` 则**没有判别力**（本次运行里它们与命令行/句柄同值，换掉不可观察），理由与不补
/// 用例的处置见模块文档「后端的取用点」一节。
///
/// `discard` 失败时直接返回，`tx` 随之析构回滚，审计行与记录的删除都不落库——这就是
/// 「失败则不删记录」。
fn discard_recorded_workspace(db: &Db, intent: &IntentId) -> Result<(), TaskError> {
    let tx = db.begin()?;
    let record = load_workspace(&tx, intent)?.ok_or_else(|| TaskError::RecordMissing {
        intent: intent.as_str().to_owned(),
    })?;

    let base = BaseWorkspace::new(record.base_path.clone())?;
    let task = TaskWorkspace::new_outside(&base, record.path.clone(), record.intent_id.clone())?;
    IntegrationGate::new(&base).discard(&tx, &task, record.backend, now_millis())?;

    remove_workspace(&tx, &record.intent_id)?;
    tx.commit()?;
    Ok(())
}

/// 落库失败那一支的放弃：手上有句柄，但**库里没有本 Intent 的记录**。
///
/// 与 [`discard_recorded_workspace`] 同法经 [`IntegrationGate::discard`]（审计行的产生点
/// 只有一个），差别只在「没有记录可删」——没有记录就无从 [`load_workspace`]，故这里收
/// 手边的句柄而不是 Intent。
///
/// **`db.begin()` 失败时退回裸 [`discard_task_workspace`]**：那时审计行写不进任何地方
/// （连接已不可用），而本支要避免的恰恰是「有工作区无记录」——为了一条无论如何都写不成
/// 的审计行把工作区留下，方向反了。**这一支没有用例**：造不出「已打开的连接无法开始
/// 事务」，据实记为未被任何用例覆盖的代码路径（与 [`TaskError::RecordMissing`] 同类）。
fn discard_unrecorded_workspace(
    db: &Db,
    base: &BaseWorkspace,
    task: &TaskWorkspace,
    backend: WorkspaceBackend,
) -> Result<(), TaskError> {
    let tx = match db.begin() {
        Ok(tx) => tx,
        Err(_) => return discard_task_workspace(base, task, backend).map_err(TaskError::from),
    };
    IntegrationGate::new(base).discard(&tx, task, backend, now_millis())?;
    tx.commit()?;
    Ok(())
}

/// 第 7 步：按第 4 步那次裁决决定是否铸造批准值 → 经 Gate 应用。
///
/// **`decision` 由调用方传入，本函数不查策略**：裁决在第 4 步做过一次（理由见模块文档
/// 「策略只查一次」），这里复用同一个值。再查一次会让同一个裁决有两个产生点。
///
/// **这一支不清理工作区**：成功与被拒都不清理（设计第 4.2 节）。被拒时改动是完整可用的，
/// 丢弃它会让用户无从恢复；成功时也一样——集成把改动并进 Base 是一条独立的通道，
/// 收不收尾由第 8 步单独规定，而**第 8 步只在「未给 `--apply`」或「命令退出码非 0」时走**，
/// 本函数成功返回后不会再回到那里。
///
/// **`backend` 一律取自记录**（设计第 4.4 节），与第 8 步同一处口径。
///
/// 铸造出的批准值只对**这一次**集成有效（[`approve_integration`] 的文档）：`apply_patch`
/// 在动第一个字节之前会重算摘要并比对，故「铸造」与「应用」之间若有人改了 Base 或 Task，
/// 集成会以 [`GateError::ApprovalMismatch`] 失败而不是把错的改动落下去。
fn apply_recorded_integration(
    db: &Db,
    args: &TaskArgs,
    decision: Decision,
) -> Result<(), TaskError> {
    let tx = db.begin()?;
    let record = load_workspace(&tx, &args.intent)?.ok_or_else(|| TaskError::RecordMissing {
        intent: args.intent.as_str().to_owned(),
    })?;

    // `--approve` 给没给既是第 4 步裁决用的**事实**，也是 [`mints`] 用的**开关**。
    // 两处取的是同一个 `args.approve`：若一处写成 `true`，「事实」与「开关」就会分岔，
    // 而分岔的两侧都编译得过。
    let approved = args.approve;
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
/// `--approve` 已给出时那条第 2 级内建 `Allow` 会参与夺冠（它是第 2 级），故裁决值只能
/// 来自比它更高（第 1 级）或与它同层更严的规则；未给出时它不成立，裁决值由落库规则或
/// 「无匹配默认 `Deny`」给出。下面逐种裁决**列全来源**——纪律要求「绝对措辞须有对应
/// 用例」，故每一支来源都要有照片，见各条末尾。
///
/// - **`Allow` → 铸造。** 夺冠的那条规则是 `Allow`，来源只有两类：
///   1. **落库的一条 `Allow`**——层级不限（[`explicit_current_rule`] 已说明 `save_policy`
///      不校验层级来源）。**无 `--approve` 时的主路径正是落库的第 3–5 级 `Allow`，
///      即设计第 5.7 节第一行**；照片：`a_runtime_default_allow_mints_without_the_flag`
///      与端到端的 `an_allowed_integration_is_applied_without_the_flag`。
///   2. **第 2 级内建的那条**（[`explicit_current_rule`]），即 `--approve` 已给出时；
///      照片：`with_no_rule_at_all_the_flag_still_decides`。
/// - **`RequireApproval` → 有 `--approve` 才铸造。** 夺冠的那条规则是 `RequireApproval`：
///   - `--approve` **未**给出：只能来自落库的某条 `RequireApproval`（层级不限，理由同上；
///     设计上落库的是第 3、4 级、第 5 级内建，但 `save_policy` 不拦第 1 级）；照片：
///     `a_require_approval_rule_needs_the_flag` 的前半段。
///   - `--approve` **已**给出：落库的第 3–6 级会被第 2 级的 `Allow` 越过（同一张照片的
///     后半段），但仍有两处返回 `RequireApproval`——**落库的第 1 级**，或**与第 2 级同层
///     的一条**（同层取更严，压过内建的 `Allow`）；两处各一张照片，见
///     `a_require_approval_verdict_survives_the_flag_at_the_first_two_levels`。故
///     `approved` 这个入参在这一支上可观察，六格表逐格钉住它。
/// - **`Deny` + `--approve` 未给出 → 不铸造。**（夺冠的是一条落库 `Deny`，或无任何规则
///   匹配而落到默认 `Deny`。）
/// - **`Deny` + `--approve` 已给出 → 不铸造。** 此时那条内建 `Allow` 在表里，却仍返回
///   `Deny`，来源只有三种：
///   1. **落库的第 1 级 `Deny`**（高于第 2 级）；照片：
///      `a_system_safety_deny_is_not_overridden_by_the_flag`。
///   2. **落库一条与第 2 级同层的 `Deny`**（`ExplicitCurrent`；同层取更严，压过内建的
///      `Allow`）——可达性论据同第 1 条来源：`save_policy` 不校验层级来源；照片：
///      `a_same_level_persisted_deny_survives_the_flag`。
///   3. **调用方没把第 2 级规则放进表里**（违反 [`decide`] 的前置条件）：此时 `Deny` 只是
///      「没有更高的规则放行」，`decide` 无从知道 `--approve` 的存在。**本驱动不走这条**
///      （[`arbitrate`] 恒把那条规则放进表里），故它没有照片——列在这里是为了让「`Deny`
///      的三种含义」在本层是完整的，三种的处置见 `engine.rs` 的说明。
///   **三种都不该铸造**：第 1 种是命令开关越不过的那条防线；后两种若铸造，就是把
///   「更严的同层规则」或「一条都没放行」读成了批准。
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
    /// 某条 `--effect` 的幂等键已有记录，拒绝运行**整条**命令（设计第 6.5 节）。
    ///
    /// **拒绝而非静默跳过**：静默跳过会让调用方以为命令执行了。命令一步都没跑，
    /// 工作区与记录按第 8 步的次序清理。
    #[error(
        "幂等键 {key} 已有记录：拒绝运行整条命令（已存在的记录不静默跳过——\
         跳过会让调用方以为命令执行了）"
    )]
    EffectAlreadyRecorded { key: String },
    /// 某条 `--effect` 的策略裁决**铸不出能力**，拒绝运行**整条**命令（强制点 (2)，
    /// 设计第 4.1 节）。
    ///
    /// 命令一步都没跑。`decision` 取 [`decision_name`] 的中文名，使调用方能分辨是
    /// 「要求批准」还是「禁止」；`effect` / `target` 点名是哪一条声明——多效应时报的是
    /// **按声明次序第一条**铸不出的。照片：`capability_gate.rs` 的
    /// `the_first_unmintable_effect_in_declaration_order_is_reported`（两条都铸不出，
    /// 把次序对调一次，报的就换成另一条；据此也钉住了「不是报最后一条」）。
    ///
    /// 与 [`TaskError::IntegrationRefused`] 是两件事：那一个是集成那次裁决（`--apply`
    /// 那一支，命令**已经跑完**），本变体是逐条效应的裁决（命令**一步没跑**）。
    #[error(
        "效应 {effect}:{target} 的策略裁决为 {decision}，铸不出能力：\
         拒绝运行整条命令（命令一步都没跑）"
    )]
    EffectNotAuthorized {
        effect: &'static str,
        target: String,
        decision: &'static str,
    },
    /// 策略裁决为「不铸造批准值」，故拒绝集成。
    ///
    /// **工作区与记录原样保留**（设计第 4.2 节）：命令成功了，改动是完整可用的，
    /// 丢弃它会让用户无从恢复。`path` 就是那份改动所在之处。
    ///
    /// `decision` 取 [`decision_name`] 的中文名，使调用方能分辨是「要求批准」还是
    /// 「禁止」。**两者都重跑不了**：第二次运行的第 3 步 `create_task_workspace` 必然
    /// 失败——worktree 后端撞已存在的分支，overlay 后端拒绝已存在的 Intent 目录；即便
    /// 跨过第 3 步，有 `--effect` 时幂等键也会在第 4 步再拒一次。用户须进本错误给出的
    /// `path` 自行处理（手工 git）。**对「禁止」还有一层：它本就越不过**（[`mints`] 列的
    /// 三种 `Deny` 来源都越不过），得从那条规则本身或被拒的原因入手。
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
    use continuum_effect::EffectType;

    /// 幂等键对三段是**单射**：设计第 6.5 节的键由 意图 id / 类型 / 目标 派生，
    /// 若拼接有歧义，两条不同的声明会被当成同一条，第二条被静默拒绝。
    ///
    /// 用例给出一对**真的会撞**的三元组（`a` + `publish` + `b:publish:c` 与
    /// `a:publish:b` + `publish` + `c`；`IntentId::new` 不校验，含冒号的意图是收下的）。
    /// 带长度前缀时两者分得开；去掉长度前缀、改用普通分隔符拼接时本用例变红——
    /// 这就是 [`effect_key`] 那条「长度前缀使拼接是单射」论据的对照片。
    #[test]
    fn the_effect_key_separates_the_intent_from_the_target() {
        let spec = |target: &str| EffectSpec {
            effect_type: EffectType::Publish,
            target: target.to_owned(),
        };
        let first = effect_key(&IntentId::new("a"), &spec("b:publish:c"));
        let second = effect_key(&IntentId::new("a:publish:b"), &spec("c"));
        assert_ne!(
            first, second,
            "两条不同的效应声明派生出同一个幂等键：第二条会被当成已登记而静默拒绝"
        );
    }

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
            "Deny 且已给出 --approve → 仍不铸造。三种来源（落库第 1 级、落库同层更严、\
             调用方没放第 2 级规则）都不该铸造，见 `mints` 的文档"
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

    /// 落库一条**与第 2 级同层**的 `Deny`：`--approve` 已给出也仍返回 `Deny`。
    ///
    /// 这是 `mints` 文档里「`Deny` + `--approve` 已给出」三种来源中的第 2 种的落点。
    /// 同层取更严（`severity(Deny) > severity(Allow)`），故它压过内建的那条第 2 级 `Allow`。
    #[test]
    fn a_same_level_persisted_deny_survives_the_flag() {
        let table = vec![always(Level::ExplicitCurrent, Decision::Deny)];
        assert_eq!(
            arbitrate(&table, &approved()),
            Decision::Deny,
            "同层取更严：落库的第 2 级 Deny 压过内建的 Allow，--approve 越不过"
        );
        assert!(!mints(Decision::Deny, true), "同层更严的 Deny 一律不铸造");
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
