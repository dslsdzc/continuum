use continuum_arbiter::{
    ArbiterPolicy, ResourceAction, ResourceArbiter, ResourceObservation, ResourcePressure,
    SuspensionError, SuspensionPhase, SuspensionState, WorkloadDeclaration,
};

const BACKGROUND: WorkloadDeclaration = WorkloadDeclaration {
    user_process: false,
    foreground: false,
    maintenance: false,
};
const FOREGROUND: WorkloadDeclaration = WorkloadDeclaration {
    user_process: false,
    foreground: true,
    maintenance: false,
};

fn critical() -> ResourceArbiter {
    let mut arbiter = ResourceArbiter::new(ArbiterPolicy::default()).unwrap();
    arbiter.observe(
        &ResourceObservation {
            cpu: Some(95),
            ..Default::default()
        },
        0,
    );
    arbiter
}

#[test]
fn suspend_only_reaches_checkpointing_and_success_reaches_suspended() {
    let arbiter = critical();
    let mut state = SuspensionState::new();
    let decision = arbiter.direct(BACKGROUND, &mut state, 0).unwrap();
    assert_eq!(decision.action(), Some(ResourceAction::Suspend));
    assert_eq!(decision.phase(), SuspensionPhase::Checkpointing);
    assert!(state.checkpoint_deadline_millis().is_some());
    assert_eq!(
        state.checkpoint_succeeded(0),
        Ok(SuspensionPhase::Suspended)
    );
    assert_eq!(state.phase(), SuspensionPhase::Suspended);
    assert_eq!(state.checkpoint_deadline_millis(), None);
}

#[test]
fn resume_uses_the_suspension_state_clock_not_the_pressure_clock() {
    let mut arbiter = critical();
    let mut state = SuspensionState::new();
    assert_eq!(
        arbiter
            .direct(BACKGROUND, &mut state, 0)
            .unwrap()
            .action(),
        Some(ResourceAction::Suspend)
    );
    assert_eq!(
        state.checkpoint_succeeded(0),
        Ok(SuspensionPhase::Suspended)
    );
    arbiter.observe(
        &ResourceObservation {
            cpu: Some(10),
            ..Default::default()
        },
        30_000,
    );
    assert_eq!(
        arbiter.summary().resource_pressure(),
        ResourcePressure::Low
    );
    let decision = arbiter.direct(BACKGROUND, &mut state, 30_000).unwrap();
    assert_eq!(decision.action(), Some(ResourceAction::Resume));
    assert_eq!(decision.phase(), SuspensionPhase::Running);
}

#[test]
fn resume_waits_for_the_state_cooldown_even_when_pressure_is_low() {
    let mut arbiter = critical();
    let mut state = SuspensionState::new();
    assert_eq!(
        arbiter
            .direct(BACKGROUND, &mut state, 0)
            .unwrap()
            .action(),
        Some(ResourceAction::Suspend)
    );
    assert_eq!(
        state.checkpoint_succeeded(20_000),
        Ok(SuspensionPhase::Suspended)
    );
    arbiter.observe(
        &ResourceObservation {
            cpu: Some(10),
            ..Default::default()
        },
        30_000,
    );
    assert_eq!(
        arbiter.summary().resource_pressure(),
        ResourcePressure::Low
    );
    let decision = arbiter.direct(BACKGROUND, &mut state, 30_000).unwrap();
    assert_eq!(decision.action(), None);
    assert_eq!(decision.phase(), SuspensionPhase::Suspended);
    let decision = arbiter.direct(BACKGROUND, &mut state, 50_000).unwrap();
    assert_eq!(decision.action(), Some(ResourceAction::Resume));
    assert_eq!(decision.phase(), SuspensionPhase::Running);
}

#[test]
fn checkpoint_failure_returns_to_throttled_and_never_suspends() {
    let arbiter = critical();
    let mut state = SuspensionState::new();
    assert_eq!(
        arbiter
            .direct(BACKGROUND, &mut state, 0)
            .unwrap()
            .action(),
        Some(ResourceAction::Suspend)
    );
    assert_eq!(
        state.checkpoint_failed(1),
        SuspensionError::CheckpointFailed
    );
    assert_eq!(state.phase(), SuspensionPhase::Throttled);
    assert_eq!(
        state.checkpoint_succeeded(2),
        Err(SuspensionError::NotCheckpointing {
            phase: SuspensionPhase::Throttled
        })
    );
    assert_ne!(state.phase(), SuspensionPhase::Suspended);
}

#[test]
fn checkpoint_calls_outside_checkpointing_report_the_current_phase() {
    let mut state = SuspensionState::new();
    assert_eq!(
        state.checkpoint_failed(3),
        SuspensionError::NotCheckpointing {
            phase: SuspensionPhase::Running
        }
    );
    assert_eq!(state.phase(), SuspensionPhase::Running);
    assert_eq!(
        state.checkpoint_timed_out(4),
        SuspensionError::NotCheckpointing {
            phase: SuspensionPhase::Running
        }
    );
}

#[test]
fn throttle_while_already_throttled_is_a_no_op() {
    let arbiter = critical();
    let mut state = SuspensionState::new();
    let first = arbiter.direct(FOREGROUND, &mut state, 0).unwrap();
    assert_eq!(first.action(), Some(ResourceAction::Throttle));
    assert_eq!(first.phase(), SuspensionPhase::Throttled);
    let second = arbiter.direct(FOREGROUND, &mut state, 1).unwrap();
    assert_eq!(second.action(), None, "Throttled + Throttle 必须无转移");
    assert_eq!(second.phase(), SuspensionPhase::Throttled);
    assert_eq!(state.phase(), SuspensionPhase::Throttled);
}
