//! 边界层：Workspace 抽象与只读强制（§256）。

pub mod base;
pub mod error;
pub mod ids;
pub mod task;

pub use base::BaseWorkspace;
pub use error::WorkspaceError;
pub use ids::IntentId;
pub use task::{TaskWorkspace, WritablePath};
