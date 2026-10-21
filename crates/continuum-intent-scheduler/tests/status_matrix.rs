mod support;

use continuum_intent_scheduler::{BaselineIntentPolicy, IntentDecision, IntentScheduler};
use continuum_semantics::IntentStatus;

#[test]
fn seven_non_candidate_statuses_yield_no_intent() {
    let statuses = [
        IntentStatus::Proposed,
        IntentStatus::Specifying,
        IntentStatus::Planning,
        IntentStatus::Review,
        IntentStatus::Completed,
        IntentStatus::Failed,
        IntentStatus::Cancelled,
    ];
    for status in statuses {
        let candidates = [support::intent("x", status)];
        let inputs = support::inputs(&candidates);
        let decision = IntentScheduler::decide(&inputs, &BaselineIntentPolicy)
            .unwrap_or_else(|error| panic!("{status:?} 不应产生错误面：{error:?}"));
        assert!(matches!(decision, IntentDecision::NoIntent { .. }), "{status:?} 不得被推进");
        assert!(decision.advanced().is_none());
        assert!(!decision.basis().as_str().is_empty());
    }
}

#[test]
fn six_advanceable_statuses_pass_the_decide_check() {
    let statuses = [
        IntentStatus::Ready,
        IntentStatus::Running,
        IntentStatus::Waiting,
        IntentStatus::Suspended,
        IntentStatus::Verifying,
        IntentStatus::Evolving,
    ];
    for status in statuses {
        let candidates = [support::intent("x", status)];
        let inputs = support::inputs(&candidates);
        let policy = support::FixedPolicy::returning(support::advance("x", "legal"));
        let decision = IntentScheduler::decide(&inputs, &policy)
            .unwrap_or_else(|error| panic!("{status:?} 应可推进：{error:?}"));
        assert_eq!(decision.advanced().map(|id| id.as_str()), Some("x"), "{status:?}");
        // Evolving 只按状态合法性构造；本用例不代表存在活跃会话。P7D 必须在候选集构造时过滤。
    }
}

#[test]
fn baseline_selects_each_active_status_under_user_focus() {
    let active_statuses = [
        IntentStatus::Running,
        IntentStatus::Waiting,
        IntentStatus::Suspended,
        IntentStatus::Verifying,
        IntentStatus::Evolving,
    ];
    for status in active_statuses {
        let candidates = [
            support::intent("ready", IntentStatus::Ready),
            support::intent("active", status),
        ];
        let mut inputs = support::inputs(&candidates);
        inputs.user_focus =
            continuum_intent_scheduler::UserFocus::Intent(continuum_semantics::IntentId::from_str("active"));

        let decision = IntentScheduler::decide(&inputs, &BaselineIntentPolicy).expect("基线应成功");
        assert_eq!(
            decision.advanced().map(|id| id.as_str()),
            Some("active"),
            "{status:?} 不得被基线过滤掉；若被过滤，聚焦命中失败后会落到 READY"
        );
        assert_eq!(decision.basis().as_str(), "user_focus");
        // Evolving：本组只证明基线不按状态过滤；P7D 必须只在确有对应活跃会话时把它放进候选集。
    }
}
