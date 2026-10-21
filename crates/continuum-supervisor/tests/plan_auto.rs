//! Task 9：`plan_auto` 复用 P7B 的选择，并保留 `NoIntent` 的会话与后台摘要。

mod support;

use continuum_intent_scheduler::{DecisionBasis, IntentDecision};
use continuum_semantics::{IntentId, IntentKind, IntentStatus};
use continuum_supervisor::{
    BackgroundDigest, BackgroundNote, SessionCandidate, SessionMode, SessionOrchestrator,
    SupervisorDecision, SupervisorError,
};

use support::{inputs, named_intent, task, FailingPolicy, FixedPolicy};

fn advance(intent: &str) -> IntentDecision {
    IntentDecision::Advance {
        intent: IntentId::from_str(intent),
        basis: DecisionBasis::new("fixed"),
    }
}

fn note(label: &str, at_millis: i64) -> BackgroundNote {
    BackgroundNote::new(label, at_millis).expect("夹具标签非空")
}

fn candidate_labels(plan: &continuum_supervisor::SessionPlan) -> Vec<String> {
    plan.candidates()
        .iter()
        .map(|candidate| match candidate {
            SessionCandidate::ContinueSession { intent } => {
                format!("session:{}", intent.as_str())
            }
            SessionCandidate::Task(task) => format!(
                "task:{}",
                task.intent().expect("Intent 任务").id.as_str()
            ),
            SessionCandidate::BackgroundFinding(note) => {
                format!("finding:{}", note.label())
            }
            SessionCandidate::NewConversation => "new".to_owned(),
        })
        .collect()
}

#[test]
fn auto_plan_uses_the_same_segments_and_keeps_the_decision() {
    let candidates = [named_intent(
        "intent-1",
        IntentKind::Task,
        IntentStatus::Ready,
    )];
    let tasks = [task(named_intent(
        "intent-1",
        IntentKind::Task,
        IntentStatus::Ready,
    ))];
    let sessions = [IntentId::from_str("session-a")];
    let background =
        BackgroundDigest::new(vec![note("已完成", 1)], vec![note("发现", 2)]);
    let policy = FixedPolicy {
        decision: advance("intent-1"),
    };
    let orchestrator = SessionOrchestrator::default();
    let plan = orchestrator
        .plan_auto(
            &inputs(&candidates),
            &policy,
            &tasks,
            &sessions,
            background,
        )
        .expect("Auto 计划必须成功");

    assert_eq!(plan.mode(), SessionMode::Auto);
    assert!(matches!(
        plan.decision(),
        Some(SupervisorDecision::Task { .. })
    ));
    assert!(matches!(
        plan.selection(),
        Some(SupervisorDecision::Task { .. })
    ));
    assert_eq!(
        candidate_labels(&plan),
        [
            "session:session-a",
            "task:intent-1",
            "finding:发现",
            "new",
        ]
    );
}

#[test]
fn no_intent_keeps_existing_sessions_and_background() {
    let candidates = [named_intent(
        "intent-1",
        IntentKind::Task,
        IntentStatus::Ready,
    )];
    let sessions = [IntentId::from_str("session-a")];
    let background =
        BackgroundDigest::new(vec![note("已完成", 1)], vec![note("发现", 2)]);
    let policy = FixedPolicy {
        decision: IntentDecision::NoIntent {
            basis: DecisionBasis::new("none"),
        },
    };
    let orchestrator = SessionOrchestrator::default();
    let plan = orchestrator
        .plan_auto(
            &inputs(&candidates),
            &policy,
            &[],
            &sessions,
            background.clone(),
        )
        .expect("NoIntent 不是错误");

    assert!(matches!(
        plan.decision(),
        Some(SupervisorDecision::NoIntent { .. })
    ));
    assert!(plan.selection().is_none());
    assert_eq!(plan.basis().expect("NoIntent 仍保留 basis").as_str(), "none");
    let labels = candidate_labels(&plan);
    assert!(labels.contains(&"session:session-a".to_owned()));
    assert!(labels.contains(&"finding:发现".to_owned()));
    assert_eq!(plan.background(), &background);
}

#[test]
fn resume_selection_need_not_appear_in_candidates() {
    let candidates = [named_intent(
        "intent-1",
        IntentKind::Task,
        IntentStatus::Waiting,
    )];
    let policy = FixedPolicy {
        decision: advance("intent-1"),
    };
    let orchestrator = SessionOrchestrator::default();
    let plan = orchestrator
        .plan_auto(
            &inputs(&candidates),
            &policy,
            &[],
            &[],
            BackgroundDigest::empty(),
        )
        .expect("恢复分支不要求候选");

    match plan.decision() {
        Some(SupervisorDecision::ContinueSession {
            disposition,
            intent,
            ..
        }) => {
            assert_eq!(*disposition, continuum_supervisor::ContinueDisposition::Resume);
            assert_eq!(intent.as_str(), "intent-1");
        }
        other => panic!("必须得到 ContinueSession，实际 {other:?}"),
    }
    assert!(plan.selection().is_some());
    assert_eq!(
        candidate_labels(&plan),
        ["new"],
        "Resume 选中项不补进 candidates"
    );
}

#[test]
fn auto_plan_includes_new_conversation_in_both_modes() {
    let candidates = [named_intent(
        "intent-1",
        IntentKind::Task,
        IntentStatus::Ready,
    )];
    let tasks = [task(named_intent(
        "intent-1",
        IntentKind::Task,
        IntentStatus::Ready,
    ))];
    let sessions = [IntentId::from_str("session-a")];
    let background = BackgroundDigest::new(vec![], vec![note("发现", 2)]);
    let policy = FixedPolicy {
        decision: advance("intent-1"),
    };
    let orchestrator = SessionOrchestrator::default();
    let manual = orchestrator.plan_manual(
        &candidates,
        &tasks,
        &sessions,
        background.clone(),
    );
    let auto = orchestrator
        .plan_auto(
            &inputs(&candidates),
            &policy,
            &tasks,
            &sessions,
            background,
        )
        .expect("Auto 计划必须成功");

    assert!(matches!(
        manual.candidates().last(),
        Some(SessionCandidate::NewConversation)
    ));
    assert!(matches!(
        auto.candidates().last(),
        Some(SessionCandidate::NewConversation)
    ));
    assert_eq!(candidate_labels(&manual), candidate_labels(&auto));
}

#[test]
fn background_does_not_change_the_policy_choice() {
    let candidates = [named_intent(
        "intent-1",
        IntentKind::Task,
        IntentStatus::Ready,
    )];
    let tasks = [task(named_intent(
        "intent-1",
        IntentKind::Task,
        IntentStatus::Ready,
    ))];
    let policy = FixedPolicy {
        decision: advance("intent-1"),
    };
    let orchestrator = SessionOrchestrator::default();
    let with_finding = orchestrator
        .plan_auto(
            &inputs(&candidates),
            &policy,
            &tasks,
            &[],
            BackgroundDigest::new(vec![], vec![note("发现", 2)]),
        )
        .expect("Auto 计划必须成功");
    let without_finding = orchestrator
        .plan_auto(
            &inputs(&candidates),
            &policy,
            &tasks,
            &[],
            BackgroundDigest::empty(),
        )
        .expect("Auto 计划必须成功");

    assert_eq!(
        with_finding
            .selection()
            .and_then(SupervisorDecision::selected_intent)
            .map(IntentId::as_str),
        without_finding
            .selection()
            .and_then(SupervisorDecision::selected_intent)
            .map(IntentId::as_str)
    );
    assert_eq!(
        with_finding.basis().map(DecisionBasis::as_str),
        without_finding.basis().map(DecisionBasis::as_str)
    );
    assert_ne!(with_finding.background(), without_finding.background());
}

#[test]
fn auto_plan_fails_closed_without_a_plan() {
    let candidates = [named_intent(
        "intent-1",
        IntentKind::Task,
        IntentStatus::Ready,
    )];
    let orchestrator = SessionOrchestrator::default();
    let error = orchestrator
        .plan_auto(
            &inputs(&candidates),
            &FailingPolicy,
            &[],
            &[],
            BackgroundDigest::empty(),
        )
        .expect_err("策略失败必须返回错误");
    assert!(
        matches!(error, SupervisorError::Decision(_)),
        "实际 {error:?}"
    );
}
