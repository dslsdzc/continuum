//! `task` 子命令失败的原因。
//!
//! **本类型原先定义在 bin 的 `task_cmd.rs` 里**，Task 2 把它整块移进 lib：工具调用路径
//! 是一个 **lib** 函数（设计 §6.4），而它要返回同一批失败——共享函数在 bin ⇒ lib 调不到；
//! 共享函数在 lib 而返回 bin 的 `TaskError` ⇒ lib 叫不出这个名字。**变体、字段、
//! `#[error(...)]` 文案一个不改**，只是定义位置搬家。
//!
//! **被否掉的替代**（设计 §3.2，记此免得后来者重提）：另建一个 lib 侧的
//! `ToolCallPathError` + bin 侧逐变体映射——那会给同一批失败造出**第二个错误类型**，
//! 正是本项目判为 Critical 的「同一件事两个类型」。
//!
//! **F 的 Task 3 加了两个包装变体**（[`TaskError::Capability`] / [`TaskError::ToolCall`]，
//! 设计 §8 的失败面表），它们是**包装**而不是新造的判断：内层的错误类型各有唯一的产生方
//! （`continuum-capability` 的强制点 (1)、`continuum-provider` 的唯一调用入口），
//! 本层只把它们带出去。两个 `#[from]` 目标都在 `Cargo.toml` 的既有依赖里。
//!
//! **F 的 Task 6 加了第三个变体**（[`TaskError::ToolReportedError`]）：它属于设计 §8
//! 「新变体」判据里的**另一类**——不是包装，而是**本路径独有的失败**（工具自报失败），
//! 没有内层错误可带。除它之外本 task 不加任何变体：尤其不为「没有适配器」另立一个
//! （那是 [`ToolCallError::Unregistered`]，包装而不另起词汇），也不加「裁决为 `Deny`」
//! 这类重复 [`crate::tool_call::mints`] 既有判断的变体。

use continuum_capability::CapabilityError;
use continuum_core::tool::ToolId;
use continuum_persist::PersistError;
use continuum_provider::ToolCallError;
use continuum_sandbox::SandboxError;
use continuum_workspace::{GateError, WorkspaceError};
use thiserror::Error;

use crate::sandbox_select::SandboxSelectError;

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
    /// 命令一步都没跑。`decision` 取 [`crate::tool_call::decision_name`] 的中文名，
    /// 使调用方能分辨是「要求批准」还是「禁止」；`effect` / `target` 点名是哪一条声明
    /// ——多效应时报的是**按声明次序第一条**铸不出的。照片：`capability_gate.rs` 的
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
    /// `decision` 取 [`crate::tool_call::decision_name`] 的中文名，使调用方能分辨是
    /// 「要求批准」还是「禁止」。**两者都重跑不了**：第二次运行的第 3 步
    /// `create_task_workspace` 必然失败——worktree 后端撞已存在的分支，overlay 后端
    /// 拒绝已存在的 Intent 目录；即便跨过第 3 步，有 `--effect` 时幂等键也会在第 4 步
    /// 再拒一次。用户须进本错误给出的 `path` 自行处理（手工 git）。**对「禁止」还有
    /// 一层：它本就越不过**（[`crate::tool_call::mints`] 列的三种 `Deny` 来源都越不过），
    /// 得从那条规则本身或被拒的原因入手。
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
    /// 强制点 (1) 的失败（工具未登记、出示/缺失能力、能力失效、读登记项或写审计失败）。
    /// **只是包装**：把 `continuum-capability` 的错误原样带出去，不重判、不合并变体。
    ///
    /// 只有工具调用路径会产生它——命令路径没有工具 id，走不到强制点 (1)。
    #[error("工具授权失败：{0}")]
    Capability(#[from] CapabilityError),
    /// 工具侧唯一入口（`ProviderRegistry::invoke_tool`）的失败：路由未命中
    /// （`Unregistered`）或适配器自己报错（`Provider`）。**只是包装**——F 不另立
    /// 「没有适配器」之类的词汇（同一件事两个变体正是本设计在讲的同一类毛病）。
    #[error("工具调用失败：{0}")]
    ToolCall(#[from] ToolCallError),
    /// 工具跑起来了、但**自报失败**（`ToolResult.is_error`）。
    ///
    /// 与 [`TaskError::ToolCall`] 是两件事：那一个是「provider 没跑成」（传输 / 协议 /
    /// 没有适配器），本变体是「跑成了，结果是错误」。按 C 设计 §7.1 的三分法，两者
    /// 走**不同的通道**（`Ok(is_error: true)` vs `Err`），故不得合并。
    ///
    /// `tool` 是 `--tool` 给的那个 id——本变体是**本路径独有**的失败（设计 §8 的
    /// 「新变体」判据），它不包装任何内层错误：适配器已经跑成功了，没有内层错误可带。
    ///
    /// **只有工具调用路径会产生它**：命令路径跑的是子进程，退出码非 0 由
    /// [`TaskError::CommandFailed`] 承载，没有「工具自报失败」这一回事。
    ///
    /// 照片：`tests/tool_call.rs` 的
    /// `a_tool_that_reports_its_own_failure_marks_the_effects_failed`（终态记 `FAILED`
    /// 与变体两件事各一枚变异体，见那条用例的文档）。
    #[error("工具 {} 自报失败", tool.as_str())]
    ToolReportedError { tool: ToolId },
}
