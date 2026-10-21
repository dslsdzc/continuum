//! Task 7：`decide` 只转调 P7B，并按候选当前状态分派。

mod support;

use continuum_intent_scheduler::{DecisionBasis, IntentDecision};
use continuum_semantics::{IntentId, IntentKind, IntentStatus};
use continuum_supervisor::{
    BackgroundSupervisor, ContinueDisposition, SupervisorDecision, SupervisorError,
};

use support::{
    code_only, fn_body, inputs, named_intent, task, FailingPolicy, FixedPolicy,
};

fn advance(intent: &str) -> IntentDecision {
    IntentDecision::Advance {
        intent: IntentId::from_str(intent),
        basis: DecisionBasis::new("fixed"),
    }
}

#[test]
fn decide_maps_ready_to_task() {
    let candidate = named_intent("intent-1", IntentKind::Task, IntentStatus::Ready);
    let candidates = [candidate];
    let tasks = [task(named_intent(
        "intent-1",
        IntentKind::Task,
        IntentStatus::Ready,
    ))];
    let policy = FixedPolicy {
        decision: advance("intent-1"),
    };
    let supervisor = BackgroundSupervisor::default();
    let decision = supervisor
        .decide(&inputs(&candidates), &policy, &tasks, &[])
        .expect("READY 且有任务必须得到 Task");
    assert!(matches!(decision, SupervisorDecision::Task { .. }));
    assert_eq!(
        decision
            .selected_intent()
            .expect("Task 必须给出 IntentId")
            .as_str(),
        "intent-1"
    );
    assert_eq!(decision.basis().as_str(), "fixed");
    assert!(decision.continue_disposition().is_none());
}

#[test]
fn decide_maps_running_verifying_evolving_to_continue() {
    for status in [
        IntentStatus::Running,
        IntentStatus::Verifying,
        IntentStatus::Evolving,
    ] {
        let candidate = named_intent("intent-1", IntentKind::Task, status);
        let candidates = [candidate];
        let sessions = [IntentId::from_str("intent-1")];
        let policy = FixedPolicy {
            decision: advance("intent-1"),
        };
        let supervisor = BackgroundSupervisor::default();
        let decision = supervisor
            .decide(&inputs(&candidates), &policy, &[], &sessions)
            .expect("活跃状态且有会话必须得到 ContinueSession");
        assert!(
            matches!(decision, SupervisorDecision::ContinueSession { .. }),
            "状态 {status:?} 必须映射为 ContinueSession"
        );
        assert_eq!(
            decision.continue_disposition(),
            Some(ContinueDisposition::Continue)
        );
    }
}

#[test]
fn decide_requires_an_existing_session_for_continue() {
    for status in [
        IntentStatus::Running,
        IntentStatus::Verifying,
        IntentStatus::Evolving,
    ] {
        let candidate = named_intent("intent-1", IntentKind::Task, status);
        let candidates = [candidate];
        let policy = FixedPolicy {
            decision: advance("intent-1"),
        };
        let supervisor = BackgroundSupervisor::default();
        let error = supervisor
            .decide(&inputs(&candidates), &policy, &[], &[])
            .expect_err("没有既有会话必须拒绝");
        assert!(
            matches!(
                error,
                SupervisorError::NoActiveSession { ref intent } if intent.as_str() == "intent-1"
            ),
            "状态 {status:?} 必须落 NoActiveSession，实际 {error:?}"
        );
    }
}

#[test]
fn decide_maps_waiting_and_suspended_to_resume_without_session() {
    for status in [IntentStatus::Waiting, IntentStatus::Suspended] {
        let candidate = named_intent("intent-1", IntentKind::Task, status);
        let candidates = [candidate];
        let policy = FixedPolicy {
            decision: advance("intent-1"),
        };
        let supervisor = BackgroundSupervisor::default();
        let decision = supervisor
            .decide(&inputs(&candidates), &policy, &[], &[])
            .expect("恢复分支不要求既有会话");
        assert!(
            matches!(decision, SupervisorDecision::ContinueSession { .. }),
            "状态 {status:?} 必须映射为 ContinueSession"
        );
        assert_eq!(
            decision.continue_disposition(),
            Some(ContinueDisposition::Resume)
        );
    }
}

#[test]
fn decide_rejects_ready_without_authorized_task() {
    let candidate = named_intent("intent-1", IntentKind::Task, IntentStatus::Ready);
    let candidates = [candidate];
    let policy = FixedPolicy {
        decision: advance("intent-1"),
    };
    let supervisor = BackgroundSupervisor::default();
    let error = supervisor
        .decide(&inputs(&candidates), &policy, &[], &[])
        .expect_err("READY 但没有授权任务必须 fail-closed");
    assert!(
        matches!(
            error,
            SupervisorError::UnauthorizedAdvance { ref intent } if intent.as_str() == "intent-1"
        ),
        "实际 {error:?}"
    );
}

#[test]
fn decide_returns_no_intent_and_preserves_basis() {
    let candidate = named_intent("intent-1", IntentKind::Task, IntentStatus::Ready);
    let candidates = [candidate];
    let policy = FixedPolicy {
        decision: IntentDecision::NoIntent {
            basis: DecisionBasis::new("none"),
        },
    };
    let sessions = [IntentId::from_str("intent-9")];
    let supervisor = BackgroundSupervisor::default();
    let decision = supervisor
        .decide(&inputs(&candidates), &policy, &[], &sessions)
        .expect("NoIntent 不是错误");
    assert!(matches!(decision, SupervisorDecision::NoIntent { .. }));
    assert!(decision.selected_intent().is_none());
    assert_eq!(decision.basis().as_str(), "none");
}

#[test]
fn decide_fails_closed_on_policy_error() {
    let candidate = named_intent("intent-1", IntentKind::Task, IntentStatus::Ready);
    let candidates = [candidate];
    let supervisor = BackgroundSupervisor::default();
    let error = supervisor
        .decide(&inputs(&candidates), &FailingPolicy, &[], &[])
        .expect_err("策略失败必须是错误");
    assert!(
        matches!(error, SupervisorError::Decision(_)),
        "策略失败不能折成 NoIntent，实际 {error:?}"
    );
}

#[test]
fn decide_does_not_reorder_the_policy_choice() {
    let first = named_intent("intent-1", IntentKind::Task, IntentStatus::Ready);
    let second = named_intent("intent-2", IntentKind::Task, IntentStatus::Ready);
    let candidates = [first, second];
    let tasks = [
        task(named_intent("intent-1", IntentKind::Task, IntentStatus::Ready)),
        task(named_intent("intent-2", IntentKind::Task, IntentStatus::Ready)),
    ];
    let policy = FixedPolicy {
        decision: advance("intent-2"),
    };
    let supervisor = BackgroundSupervisor::default();
    let decision = supervisor
        .decide(&inputs(&candidates), &policy, &tasks, &[])
        .expect("第二枚候选也是合法选择");
    assert_eq!(
        decision
            .selected_intent()
            .expect("必须给出选择")
            .as_str(),
        "intent-2"
    );
}

#[test]
fn there_is_no_second_ranking_or_implicit_sort() {
    let source = code_only(include_str!("../src/supervisor.rs"));
    for banned in [
        "impl IntentSelectionPolicy",
        "fn choose",
        "enum IntentDecision",
        "struct IntentScheduler",
        "struct SchedulerInputs",
        ".sort",
        ".sort_by",
        ".min(",
        ".max(",
        "cmp(",
        "partial_cmp",
        "BinaryHeap",
        "BTreeSet",
    ] {
        assert!(!source.contains(banned), "supervisor.rs 不应出现 {banned}");
    }
    let body = fn_body(&source, "decide");
    assert_eq!(
        body.matches("IntentScheduler::decide").count(),
        1,
        "decide 只应转调 P7B 一次：{body}"
    );
}
