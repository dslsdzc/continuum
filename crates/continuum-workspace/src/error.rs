//! Workspace 边界层的错误类型。

use std::path::PathBuf;

/// Workspace 操作失败的原因。
///
/// 变体集合横跨只读强制的三层：类型层（[`WorkspaceError::EscapesRoot`]、
/// [`WorkspaceError::Overlaps`]）、
/// 后端层（[`WorkspaceError::BackendUnavailable`]、[`WorkspaceError::IoFailed`]、
/// [`WorkspaceError::GitFailed`]）与 Gate 层（[`WorkspaceError::NotInNamespace`]、
/// [`WorkspaceError::GateRefused`]）。
/// `GitFailed` 由 worktree 后端使用；`NotInNamespace`、`GateRefused` 尚无调用点，
/// 由后续 task 的沙箱与 Integration Gate 使用；
/// 在此一次定型是为了让错误面稳定，避免各 task 各自增改变体。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
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
    /// 命令不在本 Workspace 的命名空间内。
    #[error("命令不在本 Workspace 的命名空间内")]
    NotInNamespace,
    /// `git` 命令执行失败。
    ///
    /// `stderr` 通常是 git 原样输出的内容，但后端在「失败之后用户仓库里可能仍有
    /// 残留」的路径（分支回收失败、`discard` 的任一步失败）会在其前面追加一行
    /// `（continuum-workspace：…）`，把分支名与路径一并给出——那两种情形下调用方
    /// 只能手工收拾，错误里必须给得出收拾所需的信息。`code` 始终是 git 的退出码。
    #[error("git 命令失败（退出码 {code}）：{stderr}")]
    GitFailed { code: i32, stderr: String },
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
