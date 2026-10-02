//! 边界层：Workspace 抽象与只读强制（§256）。

pub mod backend;
pub mod base;
pub mod error;
pub mod ids;
pub mod task;
// 后端实现不对外暴露：上层经 `backend` 的三个函数取用 Task Workspace。
mod worktree;

pub use backend::{
    WorkspaceBackend, create_task_workspace, detect_backend, discard_task_workspace,
};
pub use base::BaseWorkspace;
pub use error::WorkspaceError;
pub use ids::IntentId;
pub use task::{TaskWorkspace, WritablePath};
