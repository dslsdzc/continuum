use continuum_arbiter::{
    ArbiterPolicy, ResourceArbiter, ResourceObservation, SuspensionError, SuspensionPhase,
    SuspensionState, WorkloadDeclaration,
};

const BACKGROUND: WorkloadDeclaration = WorkloadDeclaration {
    user_process: false,
    foreground: false,
    maintenance: false,
};

fn critical_with_timeout(timeout: u64) -> ResourceArbiter {
    let mut arbiter = ResourceArbiter::new(ArbiterPolicy {
        checkpoint_timeout_millis: timeout,
        ..ArbiterPolicy::default()
    })
    .unwrap();
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
fn direct_at_the_deadline_returns_to_throttled() {
    let arbiter = critical_with_timeout(100);
    let mut state = SuspensionState::new();
    assert_eq!(
        arbiter
            .direct(BACKGROUND, &mut state, 0)
            .unwrap()
            .phase(),
        SuspensionPhase::Checkpointing
    );
    assert_eq!(state.checkpoint_deadline_millis(), Some(100));
    let decision = arbiter.direct(BACKGROUND, &mut state, 99).unwrap();
    assert_eq!(decision.action(), None);
    assert_eq!(decision.phase(), SuspensionPhase::Checkpointing);
    assert_eq!(
        arbiter.direct(BACKGROUND, &mut state, 100),
        Err(SuspensionError::CheckpointTimedOut)
    );
    assert_eq!(state.phase(), SuspensionPhase::Throttled);
    assert_eq!(state.checkpoint_deadline_millis(), None);
}

#[test]
fn late_success_is_rejected_after_the_deadline() {
    let arbiter = critical_with_timeout(100);
    let mut state = SuspensionState::new();
    arbiter.direct(BACKGROUND, &mut state, 0).unwrap();
    assert_eq!(
        state.checkpoint_succeeded(100),
        Err(SuspensionError::CheckpointTimedOut)
    );
    assert_eq!(state.phase(), SuspensionPhase::Throttled);
}

#[test]
fn the_timer_hook_returns_to_throttled() {
    let arbiter = critical_with_timeout(100);
    let mut state = SuspensionState::new();
    arbiter.direct(BACKGROUND, &mut state, 0).unwrap();
    assert_eq!(
        state.checkpoint_timed_out(50),
        SuspensionError::CheckpointTimedOut
    );
    assert_eq!(state.phase(), SuspensionPhase::Throttled);
    assert_eq!(state.checkpoint_deadline_millis(), None);
}

#[test]
fn timer_hook_outside_checkpointing_reports_the_phase() {
    let mut state = SuspensionState::new();
    assert_eq!(
        state.checkpoint_timed_out(50),
        SuspensionError::NotCheckpointing {
            phase: SuspensionPhase::Running
        }
    );
}

#[test]
fn zero_timeout_expires_at_the_next_state_entry() {
    let arbiter = critical_with_timeout(0);
    let mut state = SuspensionState::new();
    assert_eq!(
        arbiter
            .direct(BACKGROUND, &mut state, 0)
            .unwrap()
            .phase(),
        SuspensionPhase::Checkpointing
    );
    assert_eq!(state.checkpoint_deadline_millis(), Some(0));
    assert_eq!(
        arbiter.direct(BACKGROUND, &mut state, 0),
        Err(SuspensionError::CheckpointTimedOut)
    );
    assert_eq!(state.phase(), SuspensionPhase::Throttled);
}
