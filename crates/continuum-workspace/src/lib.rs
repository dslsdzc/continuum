//! 边界层：Workspace 抽象与只读强制（§256）。

pub mod backend;
pub mod base;
pub mod error;
pub mod ids;
pub mod task;
// 后端实现不对外暴露：上层经 `backend` 的三个函数取用 Task Workspace。
mod overlay;
mod worktree;

pub use backend::{
    WorkspaceBackend, create_task_workspace, detect_backend, discard_task_workspace,
};
pub use base::BaseWorkspace;
pub use error::WorkspaceError;
pub use ids::IntentId;
// overlay 后端另有两项出口：驱动要据 `in_user_namespace` 判断是否需要把自身 re-exec 进
// 用户与挂载命名空间，而 `OverlayBackend` 承载该后端的命名空间约束与 `.ai/` 可见性说明。
// 创建与放弃仍只经 `backend` 的三个函数，调用方不直接引用具体后端。
pub use overlay::{OverlayBackend, in_user_namespace};
pub use task::{TaskWorkspace, WritablePath};
