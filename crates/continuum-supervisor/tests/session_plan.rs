//! Task 8：`SessionPlan` 四字段、后台摘要与 `plan_manual`。

mod support;

use continuum_semantics::{IntentId, IntentKind, IntentStatus};
use continuum_supervisor::{
    BackgroundDigest, BackgroundNote, SessionCandidate, SessionMode, SessionOrchestrator,
    SessionPlan, SupervisorError,
};

use support::{code_only, fn_body, named_intent, task};

fn note(label: &str, at_millis: i64) -> BackgroundNote {
    BackgroundNote::new(label, at_millis).expect("夹具标签非空")
}

fn candidate_labels(plan: &SessionPlan) -> Vec<String> {
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
fn manual_plan_uses_four_segments_in_fixed_order() {
    let t1 = task(named_intent("intent-1", IntentKind::Task, IntentStatus::Ready));
    let t2 = task(named_intent("intent-2", IntentKind::Task, IntentStatus::Ready));
    let candidates = [
        named_intent("intent-1", IntentKind::Task, IntentStatus::Ready),
        named_intent("intent-2", IntentKind::Task, IntentStatus::Planning),
    ];
    let sessions = [
        IntentId::from_str("session-a"),
        IntentId::from_str("session-b"),
    ];
    let background = BackgroundDigest::new(
        vec![note("已完成", 10)],
        vec![note("发现一", 20), note("发现二", 30)],
    );
    let orchestrator = SessionOrchestrator::default();
    let plan = orchestrator.plan_manual(&candidates, &[t1, t2], &sessions, background);

    assert_eq!(
        candidate_labels(&plan),
        [
            "session:session-a",
            "session:session-b",
            "task:intent-1",
            "finding:发现一",
            "finding:发现二",
            "new",
        ]
    );
    assert_eq!(plan.mode(), SessionMode::Manual);
}

#[test]
fn manual_plan_does_not_select() {
    let background = BackgroundDigest::new(vec![note("已完成", 10)], vec![note("发现", 20)]);
    let orchestrator = SessionOrchestrator::default();
    let plan = orchestrator.plan_manual(
        &[named_intent("intent-1", IntentKind::Task, IntentStatus::Ready)],
        &[task(named_intent(
            "intent-1",
            IntentKind::Task,
            IntentStatus::Ready,
        ))],
        &[IntentId::from_str("session-a")],
        background.clone(),
    );
    assert!(plan.decision().is_none());
    assert!(plan.selection().is_none());
    assert!(plan.basis().is_none());
    assert_eq!(plan.mode(), SessionMode::Manual);
    assert_eq!(plan.background(), &background);
}

#[test]
fn session_plan_has_four_private_fields_and_six_accessors() {
    let source = code_only(include_str!("../src/session.rs"));
    let marker = "pub struct SessionPlan {";
    let start = source.find(marker).expect("必须有 SessionPlan 定义") + marker.len();
    let end = start + source[start..].find('}').expect("必须有结构体结尾");
    let fields: Vec<&str> = source[start..end]
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    assert_eq!(
        fields,
        [
            "mode: SessionMode,",
            "candidates: Vec<SessionCandidate>,",
            "decision: Option<SupervisorDecision>,",
            "background: BackgroundDigest,",
        ],
        "SessionPlan 必须恰四个私有字段且顺序固定"
    );
    assert_eq!(
        source.matches("impl SessionPlan {").count(),
        1,
        "SessionPlan 只应有一个 impl 块"
    );
    let impl_start = source.find("impl SessionPlan {").unwrap();
    let impl_end = impl_start + source[impl_start..].find("\n}").expect("impl 必须有结尾");
    assert_eq!(
        source[impl_start..impl_end].matches("pub fn").count(),
        6,
        "SessionPlan 必须恰六个公开访问器"
    );
    for banned in [
        "impl Default for SessionPlan",
        "Deserialize",
        "pub mode:",
    ] {
        assert!(!source.contains(banned), "session.rs 不应出现 {banned}");
    }
}

#[test]
fn background_note_has_exactly_two_private_fields() {
    let source = code_only(include_str!("../src/session.rs"));
    let marker = "pub struct BackgroundNote {";
    let start = source.find(marker).expect("必须有 BackgroundNote 定义") + marker.len();
    let end = start + source[start..].find('}').expect("必须有结构体结尾");
    let fields: Vec<&str> = source[start..end]
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    assert_eq!(fields, ["label: String,", "at_millis: i64,"]);
    for field in fields {
        assert!(!field.starts_with("pub "), "BackgroundNote 的字段必须私有");
    }
}

#[test]
fn background_note_rejects_empty_label() {
    assert!(matches!(
        BackgroundNote::new("", 0),
        Err(SupervisorError::InvalidBackgroundNote)
    ));
    let valid = BackgroundNote::new("发现", -1).expect("负时间戳合法");
    assert_eq!(valid.at_millis(), -1);
}

#[test]
fn background_digest_keeps_input_order_and_empty_is_default() {
    let empty = BackgroundDigest::empty();
    assert!(empty.completed().is_empty());
    assert!(empty.findings().is_empty());
    assert_eq!(empty, BackgroundDigest::default());

    let completed = vec![note("完成一", 1), note("完成二", 2)];
    let findings = vec![note("发现一", 3), note("发现二", 4)];
    let digest = BackgroundDigest::new(completed.clone(), findings.clone());
    assert_eq!(digest.completed(), completed.as_slice());
    assert_eq!(digest.findings(), findings.as_slice());
}

#[test]
fn background_does_not_enter_scheduler_inputs() {
    let supervisor = code_only(include_str!("../src/supervisor.rs"));
    for banned in ["BackgroundDigest", "BackgroundNote"] {
        assert!(!supervisor.contains(banned), "supervisor.rs 不应出现 {banned}");
    }

    let corpus = [
        include_str!("../src/lib.rs"),
        include_str!("../src/task.rs"),
        include_str!("../src/supervisor.rs"),
        include_str!("../src/session.rs"),
        include_str!("../src/resource.rs"),
        include_str!("../src/trigger.rs"),
        include_str!("../src/error.rs"),
    ]
    .map(code_only)
    .join("\n");
    for banned in ["CompletionVerdict", "continuum_verify"] {
        assert!(!corpus.contains(banned), "src 不应出现 {banned}");
    }
    let has_standalone_evidence = corpus
        .split(|ch: char| !(ch.is_alphanumeric() || ch == '_'))
        .any(|token| token == "Evidence");
    assert!(!has_standalone_evidence, "src 不应出现独立的 Evidence 标识符");

    let body = fn_body(&code_only(include_str!("../src/session.rs")), "plan_manual");
    for banned in ["SchedulerInputs", "decide"] {
        assert!(!body.contains(banned), "plan_manual 不应出现 {banned}");
    }
    assert_eq!(
        body.matches("next_tasks").count(),
        1,
        "plan_manual 应恰调一次 next_tasks：{body}"
    );
}
