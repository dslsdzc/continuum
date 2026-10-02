//! Workspace 边界层的错误类型。

use std::path::PathBuf;

/// Workspace 操作失败的原因。
///
/// 变体集合横跨只读强制的三层：类型层（[`WorkspaceError::EscapesRoot`]、
/// [`WorkspaceError::Overlaps`]）、
/// 后端层（[`WorkspaceError::BackendUnavailable`]、[`WorkspaceError::IoFailed`]、
/// [`WorkspaceError::GitFailed`]）与 Gate 层（[`WorkspaceError::NotInNamespace`]、
/// [`WorkspaceError::GateRefused`]）。
/// 其中 `GitFailed`、`NotInNamespace`、`GateRefused` 在本 task 中尚无调用点，
/// 由后续 task 的 worktree 后端与 Integration Gate 使用；
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
    #[error("git 命令失败（退出码 {code}）：{stderr}")]
    GitFailed { code: i32, stderr: String },
    /// Integration Gate 拒绝本次写入。
    #[error("Integration Gate 拒绝：{reason}")]
    GateRefused { reason: String },
}
