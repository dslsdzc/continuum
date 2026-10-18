mod support;

use continuum_intent_scheduler::{
    DecisionBasis, IntentDecision, IntentDecisionError, IntentScheduler,
};
use continuum_semantics::IntentStatus;

fn arm(decision: &IntentDecision) -> &'static str {
    // 不写 `_`：新增第三臂会编译失败（J1-a 的穷尽解构）。
    match decision {
        IntentDecision::Advance { .. } => "advance",
        IntentDecision::NoIntent { .. } => "no_intent",
    }
}

#[test]
fn decision_has_exactly_two_arms_and_no_intent_has_no_identifier() {
    let no = IntentDecision::NoIntent { basis: DecisionBasis::new("none") };
    assert_eq!(arm(&no), "no_intent");
    assert!(no.advanced().is_none());
    assert!(no.is_no_intent());
    assert_eq!(no.basis().as_str(), "none");

    let yes = support::advance("i1", "chosen");
    assert_eq!(arm(&yes), "advance");
    assert_eq!(yes.advanced().map(|id| id.as_str()), Some("i1"));
}

#[test]
fn an_advance_outside_the_candidate_set_is_rejected() {
    let candidates = [support::intent("ready", IntentStatus::Ready)];
    let inputs = support::inputs(&candidates);
    let policy = support::FixedPolicy::returning(support::advance("ghost", "chosen"));

    match IntentScheduler::decide(&inputs, &policy) {
        Err(IntentDecisionError::InvalidAdvance { intent }) => {
            assert_eq!(intent.as_str(), "ghost")
        }
        other => panic!("期望 InvalidAdvance，实际 {other:?}"),
    }
}

#[test]
fn an_advance_to_a_completed_candidate_is_rejected() {
    let candidates = [support::intent("done", IntentStatus::Completed)];
    let inputs = support::inputs(&candidates);
    let policy = support::FixedPolicy::returning(support::advance("done", "chosen"));

    match IntentScheduler::decide(&inputs, &policy) {
        Err(IntentDecisionError::InvalidAdvance { intent }) => {
            assert_eq!(intent.as_str(), "done")
        }
        other => panic!("期望 InvalidAdvance，实际 {other:?}"),
    }
}

#[test]
fn a_legal_advance_is_returned_unchanged() {
    let candidates = [support::intent("ready", IntentStatus::Ready)];
    let inputs = support::inputs(&candidates);
    let policy = support::FixedPolicy::returning(support::advance("ready", "chosen"));

    let decision = IntentScheduler::decide(&inputs, &policy).expect("合法 Advance 应通过");
    assert_eq!(decision.advanced().map(|id| id.as_str()), Some("ready"));
    assert_eq!(decision.basis().as_str(), "chosen");
}

#[test]
fn a_policy_error_is_policy_failed_and_keeps_the_basis() {
    let candidates = [support::intent("ready", IntentStatus::Ready)];
    let inputs = support::inputs(&candidates);
    let policy = support::FixedPolicy::failing("model_unavailable");

    match IntentScheduler::decide(&inputs, &policy) {
        Err(IntentDecisionError::PolicyFailed { basis }) => {
            assert_eq!(basis.as_str(), "model_unavailable")
        }
        other => panic!("期望 PolicyFailed，实际 {other:?}"),
    }
}

#[test]
fn an_empty_no_intent_basis_is_normalized_to_no_intent() {
    let candidates = [support::intent("ready", IntentStatus::Ready)];
    let inputs = support::inputs(&candidates);
    let policy = support::FixedPolicy::returning(support::no_intent(""));

    let decision = IntentScheduler::decide(&inputs, &policy).expect("NoIntent 应通过");
    assert!(decision.is_no_intent());
    assert_eq!(decision.basis().as_str(), "no_intent");
}

#[test]
fn a_non_empty_no_intent_basis_is_returned_unchanged() {
    let candidates = [support::intent("ready", IntentStatus::Ready)];
    let inputs = support::inputs(&candidates);
    let policy = support::FixedPolicy::returning(support::no_intent("user_busy"));

    let decision = IntentScheduler::decide(&inputs, &policy).expect("NoIntent 应通过");
    assert_eq!(decision.basis().as_str(), "user_busy");
}
