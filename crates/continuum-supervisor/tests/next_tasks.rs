//! Task 6：`next_tasks` 的过滤与每任务状态表。

mod support;

use continuum_semantics::{IntentKind, IntentStatus};
use continuum_supervisor::BackgroundSupervisor;

use support::{code_only, fn_body, named_intent, task};

#[test]
fn next_tasks_keeps_input_order_and_current_candidate_status() {
    let t1 = task(named_intent("intent-1", IntentKind::Task, IntentStatus::Ready));
    let t2 = task(named_intent("intent-2", IntentKind::Task, IntentStatus::Ready));
    let t3 = task(named_intent("intent-3", IntentKind::Task, IntentStatus::Ready));
    let t4 = task(named_intent("intent-4", IntentKind::Task, IntentStatus::Ready));
    let tasks = [t1, t2, t3, t4];
    let candidates = [
        named_intent("intent-1", IntentKind::Task, IntentStatus::Ready),
        named_intent("intent-3", IntentKind::Task, IntentStatus::Ready),
        named_intent("intent-4", IntentKind::Task, IntentStatus::Planning),
    ];

    let supervisor = BackgroundSupervisor::default();
    let visible = supervisor.next_tasks(&candidates, &tasks);
    let ids: Vec<&str> = visible
        .iter()
        .map(|task| task.intent().expect("Intent 任务").id.as_str())
        .collect();
    assert_eq!(ids, ["intent-1", "intent-3"]);
}

#[test]
fn next_tasks_keeps_policy_and_maintenance_tasks_unconditionally() {
    use continuum_supervisor::{MaintenanceOccurrenceRef, PolicyGrantRef};

    let t1 = task(named_intent("intent-1", IntentKind::Task, IntentStatus::Ready));
    let policy = continuum_supervisor::AuthorizedTask::policy(
        PolicyGrantRef::new("issuer", "grant").unwrap(),
    )
    .unwrap();
    let t2 = task(named_intent("intent-2", IntentKind::Task, IntentStatus::Ready));
    let maintenance = continuum_supervisor::AuthorizedTask::maintenance(
        MaintenanceOccurrenceRef::new("schedule", "occurrence").unwrap(),
    )
    .unwrap();
    let t3 = task(named_intent("intent-3", IntentKind::Task, IntentStatus::Ready));
    let tasks = [t1, policy, t2, maintenance, t3];
    let candidates = [
        named_intent("intent-1", IntentKind::Task, IntentStatus::Ready),
        named_intent("intent-3", IntentKind::Task, IntentStatus::Ready),
    ];

    let supervisor = BackgroundSupervisor::default();
    let visible = supervisor.next_tasks(&candidates, &tasks);
    let labels: Vec<&str> = visible
        .iter()
        .map(|task| {
            task.intent()
                .map_or_else(|| task.source().as_str(), |intent| intent.id.as_str())
        })
        .collect();
    assert_eq!(labels, ["intent-1", "policy", "maintenance_schedule", "intent-3"]);
}

#[test]
fn next_tasks_does_not_read_scheduler_inputs_or_sort() {
    let body = fn_body(
        &code_only(include_str!("../src/supervisor.rs")),
        "next_tasks",
    );
    for banned in [
        "SchedulerInputs",
        "decide",
        "sort",
        "sort_by",
        "min",
        "max",
        "cmp",
    ] {
        assert!(!body.contains(banned), "next_tasks 不应出现 {banned}：{body}");
    }
    let compact: String = body.split_whitespace().collect();
    assert_eq!(
        compact.matches("tasks.iter()").count(),
        1,
        "next_tasks 只应遍历 tasks 一次：{body}"
    );
    assert!(compact.contains("candidates"), "next_tasks 必须查候选集");
}

#[test]
fn suspension_phase_is_none_before_admit() {
    let supervisor = BackgroundSupervisor::default();
    let policy = continuum_supervisor::AuthorizedTask::policy(
        continuum_supervisor::PolicyGrantRef::new("issuer", "grant").unwrap(),
    )
    .unwrap();
    assert_eq!(supervisor.suspension_phase(&policy), None);
}
