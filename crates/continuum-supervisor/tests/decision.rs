//! Task 5：`SupervisorDecision` 的投影与 `SupervisorError` 的十臂。

mod support;

use std::error::Error;

use continuum_arbiter::{SuspensionError, SuspensionPhase};
use continuum_intent_scheduler::{DecisionBasis, IntentDecision, IntentDecisionError};
use continuum_memory::MemoryError;
use continuum_semantics::IntentId;
use continuum_supervisor::{
    CheckpointError, ContinueDisposition, SupervisorDecision, SupervisorError, TaskError,
    TaskSource,
};

fn advance(intent: &str) -> IntentDecision {
    IntentDecision::Advance {
        intent: IntentId::from_str(intent),
        basis: DecisionBasis::new("test_basis"),
    }
}

fn arm_name(error: &SupervisorError) -> &'static str {
    match error {
        SupervisorError::Task(_) => "task",
        SupervisorError::Decision(_) => "decision",
        SupervisorError::UnauthorizedAdvance { .. } => "unauthorized_advance",
        SupervisorError::NoActiveSession { .. } => "no_active_session",
        SupervisorError::UnsupportedAdvance { .. } => "unsupported_advance",
        SupervisorError::NoSuspensionState { .. } => "no_suspension_state",
        SupervisorError::Resource(_) => "resource",
        SupervisorError::Checkpoint { .. } => "checkpoint",
        SupervisorError::Memory(_) => "memory",
        SupervisorError::InvalidBackgroundNote => "invalid_background_note",
    }
}

#[test]
fn supervisor_decision_projects_the_policy_result() {
    let decision = advance("intent-2");
    let selected = SupervisorDecision::ContinueSession {
        intent: IntentId::from_str("intent-2"),
        disposition: ContinueDisposition::Resume,
        decision,
    };
    assert_eq!(
        selected
            .intent_decision()
            .advanced()
            .expect("Advance 必须带 IntentId")
            .as_str(),
        "intent-2"
    );
    assert_eq!(
        selected
            .selected_intent()
            .expect("ContinueSession 必须给出 IntentId")
            .as_str(),
        "intent-2"
    );
    assert_eq!(
        selected.continue_disposition(),
        Some(ContinueDisposition::Resume)
    );
    assert_eq!(selected.basis().as_str(), "test_basis");

    let no_intent = SupervisorDecision::NoIntent {
        decision: IntentDecision::NoIntent {
            basis: DecisionBasis::new("none"),
        },
    };
    assert!(no_intent.selected_intent().is_none());
    assert!(no_intent.continue_disposition().is_none());
    assert_eq!(no_intent.basis().as_str(), "none");
}

#[test]
fn supervisor_error_has_ten_arms_and_sources() {
    let errors = [
        SupervisorError::Task(TaskError::EmptyPolicyGrant),
        SupervisorError::Decision(IntentDecisionError::PolicyFailed {
            basis: DecisionBasis::new("model unavailable"),
        }),
        SupervisorError::UnauthorizedAdvance {
            intent: IntentId::from_str("intent-1"),
        },
        SupervisorError::NoActiveSession {
            intent: IntentId::from_str("intent-1"),
        },
        SupervisorError::UnsupportedAdvance {
            intent: IntentId::from_str("intent-1"),
        },
        SupervisorError::NoSuspensionState {
            source: TaskSource::Policy,
        },
        SupervisorError::Resource(SuspensionError::CheckpointFailed),
        SupervisorError::Checkpoint {
            error: CheckpointError::new("x"),
            phase: SuspensionPhase::Throttled,
        },
        SupervisorError::Memory(MemoryError::UnknownJob),
        SupervisorError::InvalidBackgroundNote,
    ];
    assert_eq!(errors.len(), 10);
    for error in &errors {
        assert!(!error.to_string().is_empty());
        assert!(!arm_name(error).is_empty());
    }

    assert!(
        SupervisorError::Task(TaskError::EmptyPolicyGrant)
            .source()
            .is_some()
    );
    assert!(
        SupervisorError::NoActiveSession {
            intent: IntentId::from_str("intent-1")
        }
        .source()
        .is_none()
    );
    assert!(
        SupervisorError::NoSuspensionState {
            source: TaskSource::Policy
        }
        .source()
        .is_none()
    );
    assert!(
        SupervisorError::Checkpoint {
            error: CheckpointError::new("x"),
            phase: SuspensionPhase::Throttled,
        }
        .source()
        .is_some()
    );
    assert!(SupervisorError::Memory(MemoryError::UnknownJob).source().is_some());
    assert!(
        SupervisorError::Resource(SuspensionError::CheckpointFailed)
            .source()
            .is_some()
    );
}

#[test]
fn checkpoint_error_keeps_the_message_verbatim() {
    let error = CheckpointError::new("磁盘已满");
    assert_eq!(error.message(), "磁盘已满");
    assert!(error.to_string().contains("磁盘已满"));
}
