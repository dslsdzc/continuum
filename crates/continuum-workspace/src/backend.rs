//! 后端的判别与分派。
//!
//! 后端由 **Base 是否为 Git 仓库**决定，上层不感知差异：调用方只经本模块的
//! 三个函数取用 Task Workspace，不直接引用具体后端。后端在创建时确定并随
//! 句柄一同返回，使后续的放弃不必重新探测——Base 中途变成/不再是仓库时，
//! 按创建时记录的判据回收才对得上当时建出来的东西。

use crate::base::BaseWorkspace;
use crate::error::WorkspaceError;
use crate::ids::IntentId;
use crate::task::TaskWorkspace;
use crate::worktree;
use serde::{Deserialize, Serialize};

/// Task Workspace 的实现形态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceBackend {
    /// Base 是 Git 仓库：每个 Intent 一个 worktree 与一条独立分支（§255、§16）。
    Worktree,
    /// Base 不是 Git 仓库：可写覆盖层（§255）。
    Overlay,
}

/// Base 是否为 Git 仓库决定后端。
///
/// 判据是 `<base>/.git` 存在（目录或文件——worktree 中它是文件）。**不使用
/// `git rev-parse`**：它会把父目录的仓库也算进来，而本函数的语义是「Base 自身
/// 是不是一个仓库」。Base 位于某个更大仓库的子目录时，用 worktree 后端会在
/// 那个外层仓库里建分支，任务的可写范围随之扩到 Base 之外。
pub fn detect_backend(base: &BaseWorkspace) -> WorkspaceBackend {
    if base.root().join(".git").exists() {
        WorkspaceBackend::Worktree
    } else {
        WorkspaceBackend::Overlay
    }
}

/// 依后端创建 Task Workspace，并返回本次采用的后端。
///
/// 返回后端是让调用方在放弃时有据可依：判据只在创建时成立，事后 Base 的形态
/// 可能已变（例如用户在建好之后才 `git init`）。
pub fn create_task_workspace(
    base: &BaseWorkspace,
    intent: &IntentId,
) -> Result<(TaskWorkspace, WorkspaceBackend), WorkspaceError> {
    let backend = detect_backend(base);
    let task = match backend {
        WorkspaceBackend::Worktree => worktree::create(base, intent)?,
        WorkspaceBackend::Overlay => return Err(overlay_unavailable("创建")),
    };
    Ok((task, backend))
}

/// 依创建时记录的后端放弃一个 Task Workspace。
///
/// 后端由调用方传入而非重新探测：重新探测会在 Base 形态已变时按**另一个**后端
/// 去回收，留下原后端的残留（worktree 目录或分支）而不报错。
pub fn discard_task_workspace(
    base: &BaseWorkspace,
    task: &TaskWorkspace,
    backend: WorkspaceBackend,
) -> Result<(), WorkspaceError> {
    match backend {
        WorkspaceBackend::Worktree => worktree::discard(base, task),
        WorkspaceBackend::Overlay => Err(overlay_unavailable("放弃")),
    }
}

/// overlay 后端尚未接入时的统一答复。
///
/// 返回 `Err` 而非静默跳过：调用方以为工作区已按 overlay 语义建好/清掉，
/// 实际什么都没发生，是比报错更坏的结局。
fn overlay_unavailable(action: &str) -> WorkspaceError {
    WorkspaceError::BackendUnavailable {
        reason: format!("Base 不是 Git 仓库，需 overlay 后端{action} Task Workspace；该后端尚未接入"),
    }
}
