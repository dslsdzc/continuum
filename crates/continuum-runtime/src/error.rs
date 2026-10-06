//! Runtime 的两个错误类型：工具调用路径的 [`TaskError`]，与模型调用路径的
//! [`ModelCallError`]。
//!
//! **两个类型同处一个文件是有意的**（设计 §10.2 把 `ModelCallError` 的落点定在这里）：
//! 本文件是 F 与 G 两个子项目共写的文件，G 只**增补**，不动 F 已放进去的 `TaskError`。
//! 两者不是「同一件事两个类型」——它们各自对应一条路径的失败面，且**互不包装**。
//!
//! # `TaskError`
//!
//! **本类型原先定义在 bin 的 `task_cmd.rs` 里**，Task 2 把它整块移进 lib：工具调用路径
//! 是一个 **lib** 函数（设计 §6.4），而它要返回同一批失败——共享函数在 bin ⇒ lib 调不到；
//! 共享函数在 lib 而返回 bin 的 `TaskError` ⇒ lib 叫不出这个名字。**变体、字段、
//! `#[error(...)]` 文案一个不改**，只是定义位置搬家。
//!
//! **被否掉的替代**（设计 §3.2，记此免得后来者重提）：另建一个 lib 侧的
//! `ToolCallPathError` + bin 侧逐变体映射——那会给同一批失败造出**第二个错误类型**，
//! 正是本项目判为 Critical 的「同一件事两个类型」。

use continuum_core::ProviderError;
use continuum_graph::failure::FailureClass;
use continuum_model_registry::RoutingError;
use continuum_persist::PersistError;
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
}

/// G 这一侧的失败面。**五枚逐条对应设计 §6.1 的表**。
///
/// **它不压平上游的错**：`Routing` 带出 D 的错、`Provider` 带出类别，两者都不是
/// 「一枚同名的变体」——那会是同一件事两个类型（本仓一贯判为缺陷的形状）。
/// 照片在 `tests/model_call.rs` 的 `the_error_type_carries_the_provider_error_verbatim`
/// （逐变体回读 `source`，`Cancelled` 单独一枚且不带类别）。
///
/// **分类与转换不在这里**：那张表与 `ProviderError` → 本类型的转换在
/// [`crate::model_call`]——本文件是 F 与 G 共写的，误改的形状是两边在同一段文字上
/// 各改各的，故 G 的增量落在不与 F 共写的模块里。
#[derive(Debug, Error)]
pub enum ModelCallError {
    /// D 的排序失败。**带出 D 的错，不压平成一枚同名的变体**——那会是同一件事两个类型。
    /// 从 G 这条路径到得了的只有 `NoEligibleCandidate`，其余三枚的可达性逐条记在设计 §4.5。
    ///
    /// **那句可达性今天没有照片**：`Routing` 在本 task 里还没有产生方（候选集的四段流程
    /// 由后续 task 落地），故「其余三枚到不了」要等实现成形才拍得到——设计 §11 把
    /// 那三处的照片列在 Task 11，本 task 只定形状。
    #[error("模型排序失败：{0}")]
    Routing(#[source] RoutingError),
    /// D 的表读失败（`list_registered` / `load_profile`）。
    #[error("模型登记表读取失败：{0}")]
    Storage(#[source] PersistError),
    /// 这一次调用在截止内没有完成（设计 §3.5）。
    ///
    /// **纯数据变体**：从构造到读回之间没有本 crate 的代码，故本 task 不为它写运行期断言
    /// （照设计 §12 的同一裁决；它的照片随 `call` 的截止用例一起落地）。
    #[error("模型调用在 {elapsed_ms} ms 内没有完成")]
    Deadline { elapsed_ms: u64 },
    /// 适配器调用失败，**带出它的类别**。
    #[error("模型适配器调用失败（{class:?}）：{source}")]
    Provider {
        class: FailureClass,
        source: ProviderError,
    },
    /// 调用方发起的取消（来源是 `ProviderError::Cancelled`）。**它不是失败，故不带类别**。
    ///
    /// **不带 `class` 不是一个字段的取舍**：它是「一次正常中止不会被记成一次失败」
    /// 这条判据在类型上的落点——若这里也带 `class`，调用方会在一个没有失败的地方
    /// 读到 `FailureClass`。照片是编译期的：`tests/model_call.rs` 里那枚只给 `source`
    /// 的字面量，`Cancelled` 一旦多出 `class` 字段就编不过。
    #[error("模型调用被调用方取消：{source}")]
    Cancelled { source: ProviderError },
}
