//! Supervisor 的错误面（设计 §4.9）。

use std::fmt;

use continuum_arbiter::{SuspensionError, SuspensionPhase};
use continuum_intent_scheduler::IntentDecisionError;
use continuum_memory::MemoryError;
use continuum_semantics::IntentId;

use crate::resource::CheckpointError;
use crate::task::{TaskError, TaskSource};

/// 决策、授权、资源、checkpoint 与记忆的失败面。十臂，不派生 `Clone`。
#[derive(Debug)]
pub enum SupervisorError {
    Task(TaskError),
    Decision(IntentDecisionError),
    UnauthorizedAdvance { intent: IntentId },
    NoActiveSession { intent: IntentId },
    UnsupportedAdvance { intent: IntentId },
    NoSuspensionState { source: TaskSource },
    Resource(SuspensionError),
    Checkpoint {
        error: CheckpointError,
        phase: SuspensionPhase,
    },
    Memory(MemoryError),
    InvalidBackgroundNote,
}

impl fmt::Display for SupervisorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Task(error) => write!(f, "任务授权失败：{error}"),
            Self::Decision(error) => write!(f, "选择失败：{error}"),
            Self::UnauthorizedAdvance { intent } => {
                write!(f, "选中的 Intent {} 没有授权任务", intent.as_str())
            }
            Self::NoActiveSession { intent } => {
                write!(f, "Intent {} 没有可继续的既有会话", intent.as_str())
            }
            Self::UnsupportedAdvance { intent } => {
                write!(f, "Intent {} 的状态不可分派", intent.as_str())
            }
            Self::NoSuspensionState { source } => {
                write!(f, "任务来源 {} 没有挂起状态", source.as_str())
            }
            Self::Resource(error) => write!(f, "资源控制失败：{error}"),
            Self::Checkpoint { error, phase } => write!(
                f,
                "checkpoint 失败（相位 {}）：{error}",
                phase.as_str()
            ),
            Self::Memory(error) => write!(f, "记忆合成失败：{error}"),
            Self::InvalidBackgroundNote => f.write_str("后台摘要的标签为空"),
        }
    }
}

impl std::error::Error for SupervisorError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Task(error) => Some(error),
            Self::Decision(error) => Some(error),
            Self::Resource(error) => Some(error),
            Self::Checkpoint { error, .. } => Some(error),
            Self::Memory(error) => Some(error),
            Self::UnauthorizedAdvance { .. }
            | Self::NoActiveSession { .. }
            | Self::UnsupportedAdvance { .. }
            | Self::NoSuspensionState { .. }
            | Self::InvalidBackgroundNote => None,
        }
    }
}
