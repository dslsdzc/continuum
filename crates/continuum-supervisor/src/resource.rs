//! 资源消费面：checkpoint 端口与来源到工作负载声明的映射（设计 §4.7）。

use std::fmt;

use crate::task::AuthorizedTask;

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
