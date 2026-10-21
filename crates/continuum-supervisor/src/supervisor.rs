//! Background Supervisor：任务过滤、选择与资源控制面（设计 §4.4／§4.7）。

use continuum_intent_scheduler::{DecisionBasis, IntentDecision};
use continuum_semantics::IntentId;

use crate::task::AuthorizedTask;

/// `ContinueSession` 的控制值。不是 P7C 的 `ResourceAction`，也不是相位。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContinueDisposition {
    /// `Running`／`Verifying`／`Evolving`：会话已在运行，交给执行器继续。
    Continue,
    /// `Waiting`／`Suspended`：P7B 的恢复分支；不要求既有会话。
    Resume,
}

/// `decide` 的结果：只三臂。
#[derive(Debug, Clone)]
pub enum SupervisorDecision {
    /// P7B 选中了一枚 READY Intent，且它有对应的 `AuthorizedTask`。
    Task {
        task: AuthorizedTask,
        decision: IntentDecision,
    },
    /// P7B 选中了一枚活跃 Intent；不迁移状态。
    ContinueSession {
        intent: IntentId,
        disposition: ContinueDisposition,
        decision: IntentDecision,
    },
    /// P7B 明确 `NoIntent`；既有会话与后台任务必须保留。
    NoIntent { decision: IntentDecision },
}

impl SupervisorDecision {
    pub fn intent_decision(&self) -> &IntentDecision {
        match self {
            Self::Task { decision, .. }
            | Self::ContinueSession { decision, .. }
            | Self::NoIntent { decision } => decision,
        }
    }

    pub fn selected_intent(&self) -> Option<&IntentId> {
        match self {
            Self::Task { task, .. } => task.intent().map(|intent| &intent.id),
            Self::ContinueSession { intent, .. } => Some(intent),
            Self::NoIntent { .. } => None,
        }
    }

    pub fn basis(&self) -> &DecisionBasis {
        self.intent_decision().basis()
    }

    pub fn continue_disposition(&self) -> Option<ContinueDisposition> {
        match self {
            Self::ContinueSession { disposition, .. } => Some(*disposition),
            Self::Task { .. } | Self::NoIntent { .. } => None,
        }
    }
}
