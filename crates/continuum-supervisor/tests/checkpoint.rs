//! Task 12：checkpoint 三出口、定时器与相位回退。

mod support;

use std::cell::Cell;

use continuum_arbiter::{
    ArbiterPolicy, ResourceArbiter, ResourceObservation, SuspensionError, SuspensionPhase,
};
use continuum_supervisor::{
    AuthorizedTask, BackgroundSupervisor, CheckpointError, PolicyGrantRef, SupervisorError,
};

use support::CountingPort;

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

fn policy_task(grant_id: &str) -> AuthorizedTask {
    AuthorizedTask::policy(PolicyGrantRef::new("issuer", grant_id).unwrap()).unwrap()
}

fn ok_port() -> CountingPort {
    CountingPort {
        result: Ok(()),
        calls: Cell::new(0),
    }
}

fn failing_port(message: &str) -> CountingPort {
    CountingPort {
        result: Err(CheckpointError::new(message)),
        calls: Cell::new(0),
    }
}

fn admitted(grant_id: &str) -> (BackgroundSupervisor, AuthorizedTask) {
    let mut supervisor = BackgroundSupervisor::default();
    let task = policy_task(grant_id);
    let decision = supervisor
        .admit(&task, &arbiter(95), 1_000)
        .expect("临界压力下必须进入 Checkpointing");
    assert_eq!(decision.phase(), SuspensionPhase::Checkpointing);
    (supervisor, task)
}

#[test]
fn checkpoint_success_before_deadline_enters_suspended() {
    let (mut supervisor, task) = admitted("grant");
    assert_eq!(supervisor.checkpoint_deadline(&task), Some(61_000));
    let port = ok_port();
    let phase = supervisor
        .checkpoint(&task, &port, 2_000)
        .expect("截止前成功必须得到 Suspended");
    assert_eq!(phase, SuspensionPhase::Suspended);
    assert_eq!(
        supervisor.suspension_phase(&task),
        Some(SuspensionPhase::Suspended)
    );
    assert_eq!(supervisor.checkpoint_deadline(&task), None);
    assert_eq!(port.calls.get(), 1);
}

#[test]
fn late_success_returns_timed_out_and_returns_to_throttled() {
    let (mut supervisor, task) = admitted("grant");
    let port = ok_port();
    let error = supervisor
        .checkpoint(&task, &port, 61_000)
        .expect_err("晚到成功不得报 Suspended");
    assert!(
        matches!(
            error,
            SupervisorError::Resource(SuspensionError::CheckpointTimedOut)
        ),
        "实际 {error:?}"
    );
    assert_eq!(
        supervisor.suspension_phase(&task),
        Some(SuspensionPhase::Throttled)
    );
    assert_eq!(supervisor.checkpoint_deadline(&task), None);
}

#[test]
fn port_failure_returns_checkpoint_error_and_throttles() {
    let (mut supervisor, task) = admitted("grant");
    let port = failing_port("磁盘已满");
    let error = supervisor
        .checkpoint(&task, &port, 2_000)
        .expect_err("端口失败必须返回错误");
    match error {
        SupervisorError::Checkpoint { error, phase } => {
            assert_eq!(error.message(), "磁盘已满");
            assert_eq!(phase, SuspensionPhase::Throttled);
        }
        other => panic!("必须落 Checkpoint 臂，实际 {other:?}"),
    }
    assert_eq!(
        supervisor.suspension_phase(&task),
        Some(SuspensionPhase::Throttled)
    );
    assert_eq!(supervisor.checkpoint_deadline(&task), None);
}

#[test]
fn timer_hook_returns_timed_out_and_throttles() {
    let (mut supervisor, task) = admitted("grant");
    let port = ok_port();
    let error = supervisor
        .checkpoint_timed_out(&task, 61_000)
        .expect_err("定时器出口没有 Ok 路径");
    assert!(
        matches!(
            error,
            SupervisorError::Resource(SuspensionError::CheckpointTimedOut)
        ),
        "实际 {error:?}"
    );
    assert_eq!(
        supervisor.suspension_phase(&task),
        Some(SuspensionPhase::Throttled)
    );
    assert_eq!(supervisor.checkpoint_deadline(&task), None);
    assert_eq!(port.calls.get(), 0, "定时器出口不调端口");
}

#[test]
fn checkpoint_without_state_is_rejected() {
    let mut supervisor = BackgroundSupervisor::default();
    let task = policy_task("grant");
    let port = ok_port();
    let error = supervisor
        .checkpoint(&task, &port, 2_000)
        .expect_err("无状态必须拒绝");
    assert!(
        matches!(error, SupervisorError::NoSuspensionState { .. }),
        "实际 {error:?}"
    );
    assert_eq!(port.calls.get(), 0);

    supervisor.admit(&task, &arbiter(95), 1_000).unwrap();
    assert!(supervisor.release(&task));
    let error = supervisor
        .checkpoint(&task, &port, 2_000)
        .expect_err("release 后必须拒绝");
    assert!(
        matches!(error, SupervisorError::NoSuspensionState { .. }),
        "实际 {error:?}"
    );
    assert_eq!(port.calls.get(), 0);
}

#[test]
fn checkpoint_outside_checkpointing_does_not_call_port() {
    let mut supervisor = BackgroundSupervisor::default();
    let task = policy_task("grant");
    supervisor.admit(&task, &arbiter(10), 1_000).unwrap();
    assert_eq!(
        supervisor.suspension_phase(&task),
        Some(SuspensionPhase::Running)
    );
    let port = ok_port();
    let error = supervisor
        .checkpoint(&task, &port, 2_000)
        .expect_err("Running 相位必须拒绝");
    assert!(
        matches!(
            error,
            SupervisorError::Resource(SuspensionError::NotCheckpointing {
                phase: SuspensionPhase::Running
            })
        ),
        "实际 {error:?}"
    );
    assert_eq!(port.calls.get(), 0);

    let (mut supervisor, task) = admitted("suspended");
    supervisor
        .checkpoint(&task, &port, 2_000)
        .expect("先进入 Suspended");
    let error = supervisor
        .checkpoint(&task, &port, 3_000)
        .expect_err("Suspended 相位必须拒绝");
    assert!(
        matches!(
            error,
            SupervisorError::Resource(SuspensionError::NotCheckpointing {
                phase: SuspensionPhase::Suspended
            })
        ),
        "实际 {error:?}"
    );
    assert_eq!(port.calls.get(), 1, "错相位不得再调端口");
}

#[test]
fn checkpoint_failure_can_be_retried_and_never_stays_checkpointing() {
    let (mut supervisor, task) = admitted("grant");
    let failing = failing_port("磁盘已满");
    supervisor
        .checkpoint(&task, &failing, 2_000)
        .expect_err("第一次 checkpoint 必须失败");
    assert_eq!(
        supervisor.suspension_phase(&task),
        Some(SuspensionPhase::Throttled)
    );

    let decision = supervisor
        .admit(&task, &arbiter(95), 40_000)
        .expect("cooldown 过后必须可重试");
    assert_eq!(
        decision.action(),
        Some(continuum_arbiter::ResourceAction::Suspend)
    );
    assert_eq!(
        supervisor.suspension_phase(&task),
        Some(SuspensionPhase::Checkpointing)
    );

    let ok = ok_port();
    let phase = supervisor
        .checkpoint(&task, &ok, 40_001)
        .expect("重试成功必须进入 Suspended");
    assert_eq!(phase, SuspensionPhase::Suspended);
    assert_eq!(supervisor.checkpoint_deadline(&task), None);
}

#[test]
fn checkpoint_timed_out_without_state_or_wrong_phase_is_rejected() {
    let mut supervisor = BackgroundSupervisor::default();
    let task = policy_task("grant");
    let error = supervisor
        .checkpoint_timed_out(&task, 1_000)
        .expect_err("无状态必须拒绝");
    assert!(
        matches!(error, SupervisorError::NoSuspensionState { .. }),
        "实际 {error:?}"
    );

    let (mut supervisor, task) = admitted("grant");
    let port = ok_port();
    supervisor
        .checkpoint(&task, &port, 2_000)
        .expect("先进入 Suspended");
    let error = supervisor
        .checkpoint_timed_out(&task, 3_000)
        .expect_err("Suspended 相位必须拒绝");
    assert!(
        matches!(
            error,
            SupervisorError::Resource(SuspensionError::NotCheckpointing {
                phase: SuspensionPhase::Suspended
            })
        ),
        "实际 {error:?}"
    );
    assert_eq!(
        supervisor.suspension_phase(&task),
        Some(SuspensionPhase::Suspended)
    );
}
