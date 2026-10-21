//! Task 11：`admit`／`release` 与每任务挂起状态复用。

mod support;

use continuum_arbiter::{
    ArbiterPolicy, ResourceAction, ResourceArbiter, ResourceObservation, SuspensionPhase,
};
use continuum_semantics::{IntentKind, IntentStatus};
use continuum_supervisor::{AuthorizedTask, BackgroundSupervisor, PolicyGrantRef};

use support::{named_intent, product_task, task};

fn arbiter(cpu: u8) -> ResourceArbiter {
    let mut arbiter = ResourceArbiter::new(ArbiterPolicy::default()).expect("默认策略合法");
    arbiter.observe(
        &ResourceObservation {
            cpu: Some(cpu),
            ..Default::default()
        },
        0,
    );
    arbiter
}

fn ready_task(id: &str) -> AuthorizedTask {
    task(named_intent(id, IntentKind::Task, IntentStatus::Ready))
}

fn policy_task(grant_id: &str) -> AuthorizedTask {
    AuthorizedTask::policy(PolicyGrantRef::new("issuer", grant_id).unwrap()).unwrap()
}

#[test]
fn admit_maps_all_four_task_sources_to_declarations() {
    let mut supervisor = BackgroundSupervisor::default();
    let high = arbiter(80);
    let low = arbiter(10);
    let critical = arbiter(95);

    let maintenance = AuthorizedTask::maintenance(
        continuum_supervisor::MaintenanceOccurrenceRef::new("schedule", "occurrence").unwrap(),
    )
    .unwrap();
    let decision = supervisor
        .admit(&maintenance, &high, 1_000)
        .expect("维护任务必须可 admit");
    assert_eq!(decision.action(), Some(ResourceAction::Suspend));
    assert_eq!(decision.phase(), SuspensionPhase::Checkpointing);

    for (index, task) in [
        policy_task("grant"),
        ready_task("intent-1"),
        product_task(IntentStatus::Ready),
    ]
    .iter()
    .enumerate()
    {
        let mut supervisor = BackgroundSupervisor::default();
        let decision = supervisor
            .admit(task, &high, 1_000 + index as u64)
            .expect("背景任务必须可 admit");
        assert_eq!(decision.action(), Some(ResourceAction::Throttle));
        assert_eq!(decision.phase(), SuspensionPhase::Throttled);
    }

    let mut supervisor = BackgroundSupervisor::default();
    let decision = supervisor
        .admit(&policy_task("low"), &low, 1_000)
        .expect("低压力必须可 admit");
    assert_eq!(decision.action(), None);
    assert_eq!(decision.phase(), SuspensionPhase::Running);

    let mut supervisor = BackgroundSupervisor::default();
    let decision = supervisor
        .admit(&policy_task("critical"), &critical, 1_000)
        .expect("临界压力必须可 admit");
    assert_eq!(decision.action(), Some(ResourceAction::Suspend));
    assert_eq!(decision.phase(), SuspensionPhase::Checkpointing);
}

#[test]
fn admit_returns_none_as_none_and_keeps_phase() {
    let mut supervisor = BackgroundSupervisor::default();
    let task = policy_task("grant");
    let low = arbiter(10);
    let first = supervisor.admit(&task, &low, 1_000).expect("第一次 admit");
    assert_eq!(first.action(), None);
    assert_eq!(first.phase(), SuspensionPhase::Running);
    let second = supervisor
        .admit(&task, &low, 1_001)
        .expect("第二次 admit");
    assert_eq!(second.action(), None, "None 不是 Run，不得改写成动作");
    assert_eq!(second.phase(), SuspensionPhase::Running);
}

#[test]
fn admit_reuses_the_same_state_for_the_same_task() {
    let mut supervisor = BackgroundSupervisor::default();
    let task = policy_task("grant");
    let critical = arbiter(95);
    let first = supervisor
        .admit(&task, &critical, 1_000)
        .expect("第一次 admit");
    assert_eq!(first.action(), Some(ResourceAction::Suspend));
    assert_eq!(first.phase(), SuspensionPhase::Checkpointing);

    let second = supervisor
        .admit(&task, &critical, 1_000)
        .expect("同一任务第二次 admit");
    assert_eq!(second.action(), None);
    assert_eq!(second.phase(), SuspensionPhase::Checkpointing);
}

#[test]
fn release_drops_state_and_a_new_admit_starts_fresh() {
    let mut supervisor = BackgroundSupervisor::default();
    let task = policy_task("grant");
    let critical = arbiter(95);
    supervisor
        .admit(&task, &critical, 1_000)
        .expect("第一次 admit");
    assert_eq!(
        supervisor.suspension_phase(&task),
        Some(SuspensionPhase::Checkpointing)
    );
    assert!(supervisor.release(&task));
    assert_eq!(supervisor.suspension_phase(&task), None);
    assert!(!supervisor.release(&task));

    let decision = supervisor
        .admit(&task, &critical, 2_000)
        .expect("释放后重新 admit");
    assert_eq!(decision.action(), Some(ResourceAction::Suspend));
    assert_eq!(decision.phase(), SuspensionPhase::Checkpointing);
}

#[test]
fn same_policy_reference_shares_one_state_and_is_a_contract_violation() {
    let mut supervisor = BackgroundSupervisor::default();
    let grant = PolicyGrantRef::new("issuer", "same-grant").unwrap();
    let first = AuthorizedTask::policy(grant.clone()).unwrap();
    let second = AuthorizedTask::policy(grant).unwrap();
    let critical = arbiter(95);

    let first_decision = supervisor
        .admit(&first, &critical, 1_000)
        .expect("第一枚任务 admit");
    assert_eq!(first_decision.action(), Some(ResourceAction::Suspend));
    assert_eq!(first_decision.phase(), SuspensionPhase::Checkpointing);

    let second_decision = supervisor
        .admit(&second, &critical, 1_000)
        .expect("第二枚任务共享同一状态");
    assert_eq!(second_decision.action(), None);
    assert_eq!(
        second_decision.phase(),
        SuspensionPhase::Checkpointing,
        "同一 ref 的两枚任务共享状态，这是调用方契约违反，不是合并行为"
    );
}

#[test]
fn distinct_policy_references_get_independent_states() {
    let mut supervisor = BackgroundSupervisor::default();
    let first = policy_task("grant-a");
    let second = policy_task("grant-b");
    let critical = arbiter(95);

    let first_decision = supervisor
        .admit(&first, &critical, 1_000)
        .expect("第一枚 admit");
    let second_decision = supervisor
        .admit(&second, &critical, 1_000)
        .expect("第二枚 admit");
    assert_eq!(first_decision.action(), Some(ResourceAction::Suspend));
    assert_eq!(second_decision.action(), Some(ResourceAction::Suspend));
    assert_eq!(
        supervisor.suspension_phase(&first),
        Some(SuspensionPhase::Checkpointing)
    );
    assert_eq!(
        supervisor.suspension_phase(&second),
        Some(SuspensionPhase::Checkpointing)
    );
}
