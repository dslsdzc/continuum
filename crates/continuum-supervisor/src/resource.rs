//! 资源消费面：checkpoint 端口与来源到工作负载声明的映射（设计 §4.7）。

use std::fmt;

use continuum_arbiter::WorkloadDeclaration;

use crate::task::{AuthorizedTask, TaskSource};

/// checkpoint 的执行端口。P7D 不实现它；装配方／执行器按任务的真实状态实现。
pub trait CheckpointPort {
    fn checkpoint(&self, task: &AuthorizedTask) -> Result<(), CheckpointError>;
}

/// checkpoint 端口错误；不透明，不裁剪。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckpointError(String);

impl CheckpointError {
    pub fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }

    pub fn message(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CheckpointError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for CheckpointError {}

/// `TaskSource` → `WorkloadDeclaration` 的唯一映射（设计 §4.7）。
/// 前两个布尔恒 false：P7D 只治理后台任务；`maintenance` 只对维护计划为 true。
pub(crate) fn declaration_for(task: &AuthorizedTask) -> WorkloadDeclaration {
    WorkloadDeclaration {
        user_process: false,
        foreground: false,
        maintenance: matches!(task.source(), TaskSource::MaintenanceSchedule),
    }
}
