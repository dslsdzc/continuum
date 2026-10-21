//! P7D Background Supervisor + Session Orchestrator（设计 §2.2）。
//! 本 crate 零表、零迁移、零事件；不执行会话恢复、不写 intent 表。

pub mod error;
pub mod resource;
pub mod session;
pub mod supervisor;
pub mod task;
pub mod trigger;

pub use task::{
    AuthorizedTask, MaintenanceOccurrenceRef, PolicyGrantRef, TaskError, TaskSource,
};
