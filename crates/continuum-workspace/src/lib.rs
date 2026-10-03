//! 边界层：Workspace 抽象与只读强制（§256）。

pub mod backend;
pub mod base;
pub mod error;
pub mod gate;
pub mod ids;
pub mod persist;
pub mod task;
// 后端实现不对外暴露：上层经 `backend` 的三个函数取用 Task Workspace。
mod overlay;
mod worktree;

pub use backend::{
    WorkspaceBackend, create_task_workspace, detect_backend, discard_task_workspace,
};
pub use base::BaseWorkspace;
pub use error::WorkspaceError;
// Gate 是 Task 与 Base 之间的唯一通道（§257）。五种操作都已实现：只读的 view_diff
// 与 discard，以及写入的 apply_patch、cherry_pick、merge。后三者收 `&GateApproval`
// ——该类型没有公开构造函数，故本子项目内还没有调用点：它的产生点在下篇交给驱动
// （设计 6.2），在此之前三项写入操作在 crate 外调用不到。
pub use gate::{Diff, GateApproval, GateError, IntegrationGate};
pub use ids::IntentId;
pub use persist::{
    WorkspaceRecord, load_workspace, p2_workspace_migrations, remove_workspace, save_workspace,
};
// overlay 后端另有两项出口：驱动要据 `in_user_namespace` 判断是否需要把自身 re-exec 进
// 用户与挂载命名空间，而 `OverlayBackend` 承载该后端的命名空间约束与 `.ai/` 可见性说明。
// 创建与放弃仍只经 `backend` 的三个函数，调用方不直接引用具体后端。
pub use overlay::{OverlayBackend, in_user_namespace};
pub use task::{TaskWorkspace, WritablePath};
