//! Workspace 边界层的错误类型。

use std::path::PathBuf;

/// Workspace 操作失败的原因。
///
/// 多数变体按只读强制的三层归属：类型层（[`WorkspaceError::EscapesRoot`]、
/// [`WorkspaceError::Overlaps`]、
/// [`WorkspaceError::InvalidIntent`]）、
/// 后端层（[`WorkspaceError::BackendUnavailable`]、[`WorkspaceError::IoFailed`]、
/// [`WorkspaceError::GitFailed`]、[`WorkspaceError::CommandFailed`]）与 Gate 层
/// （[`WorkspaceError::NotInNamespace`]、[`WorkspaceError::GateRefused`]）。
/// [`WorkspaceError::NotInNamespace`] 同时属于两层：它既是 Gate 对命令的命名空间
/// 要求，也是 overlay 后端对**调用进程自身**的要求。
///
/// [`WorkspaceError::Context`] 是例外：它不属任何一层，而是横切各层的包装——
/// 在「失败之后用户仓库里可能仍有残留」的路径上给任意变体附加一层上下文
/// （哪一步、哪个分支、哪个路径），使调用方拿得到收拾残留所需的信息。
///
/// 本枚举不是「一次定型」：`InvalidIntent` 与 `Context` 分别是 worktree 后端的
/// 边界校验与回收路径引入的。故标 `#[non_exhaustive]`——后续同类增补不再要求
/// 外部消费者改 `match`。新增变体时仍应优先考虑既有变体能否表达。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum WorkspaceError {
    /// Base 路径不存在。
    #[error("Base Workspace 路径不存在：{}", path.display())]
    MissingBase { path: PathBuf },
    /// Base 路径存在但不是目录。
    #[error("Base Workspace 路径不是目录：{}", path.display())]
    NotADirectory { path: PathBuf },
    /// 目标路径越出 Task Workspace 根。
    #[error("路径越出 Task Workspace 根：{}", path.display())]
    EscapesRoot { path: PathBuf },
    /// Task Workspace 的根与 Base 重叠：Task 根是 Base 或 Base 的祖先。
    #[error(
        "Task Workspace 的根 {} 与 Base {} 重叠：Task 根不能是 Base 或 Base 的祖先",
        root.display(),
        base.display()
    )]
    Overlaps { root: PathBuf, base: PathBuf },
    /// 后端不可用：目录无法建立、覆盖层无法挂载等。
    #[error("Workspace 后端不可用：{reason}")]
    BackendUnavailable { reason: String },
    /// Task Workspace 内的文件读写失败。
    #[error("路径 {} 的读写失败：{reason}", path.display())]
    IoFailed { path: PathBuf, reason: String },
    /// 不在本 Workspace 所需的命名空间内。
    ///
    /// 两种来源：命令未在本 Workspace 的命名空间内执行；以及——由
    /// [`crate::backend::create_task_workspace`] 在 overlay 后端下报出——调用进程
    /// 自身不在用户与挂载命名空间内，此时建出的覆盖层只对该命名空间可见，
    /// 对将要使用它的子进程不可见，故拒绝而不静默建出一个不可见的工作区。
    #[error("不在本 Workspace 所需的命名空间内")]
    NotInNamespace,
    /// `git` 命令执行失败。
    ///
    /// `stderr` 通常是 git 原样输出的内容，但后端在「失败之后用户仓库里可能仍有
    /// 残留」的路径（分支回收失败、`discard` 的任一步失败）会在其前面追加一行
    /// `（continuum-workspace：…）`，把分支名与路径一并给出——那两种情形下调用方
    /// 只能手工收拾，错误里必须给得出收拾所需的信息。`code` 始终是 git 的退出码。
    #[error("git 命令失败（退出码 {code}）：{stderr}")]
    GitFailed { code: i32, stderr: String },
    /// 外部命令执行失败（overlay 后端的 `mount` / `umount`）。
    ///
    /// 与 [`WorkspaceError::GitFailed`] **分开**：那个变体的语义与文档专指 `git`
    /// （其 `stderr` 还被 worktree 的回收路径用作上下文载体），而挂载命令不是 git
    /// 命令。调用方按变体分派时，两者混为一谈会让「git 失败了」这一分支凭空多出
    /// 一类非 git 的来源。字段形状与 `GitFailed` 一致，便于两者并排处理。
    #[error("命令 {command} 失败（退出码 {code}）：{stderr}")]
    CommandFailed {
        command: String,
        code: i32,
        stderr: String,
    },
    /// Intent 标识不能用作 Task Workspace 的路径分量与分支名。
    #[error("Intent 标识不能用作 Task Workspace 路径分量：{intent}")]
    InvalidIntent { intent: String },
    /// 在既有错误之上附加的一层上下文（哪一步、哪个分支、哪个路径）。
    ///
    /// 用在「失败之后用户仓库里可能仍有残留」的路径上：此时调用方既要看
    /// 创建为什么失败，也要拿到收拾残留所需的分支名与路径。`GitFailed` 自身
    /// 有 `stderr` 这个自由文本字段可承载上下文，故不套本变体；没有这种字段的
    /// 变体（`IoFailed`、`Overlaps` 等）由本变体承载。
    #[error("{context}：{source}")]
    Context {
        context: String,
        source: Box<WorkspaceError>,
    },
    /// Integration Gate 拒绝本次写入。
    #[error("Integration Gate 拒绝：{reason}")]
    GateRefused { reason: String },
}
