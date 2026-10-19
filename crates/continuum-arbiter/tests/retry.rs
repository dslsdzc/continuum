use continuum_arbiter::{
    ArbiterPolicy, ResourceAction, ResourceArbiter, ResourceObservation, SuspensionError,
    SuspensionPhase, SuspensionState, WorkloadDeclaration,
};

const BACKGROUND: WorkloadDeclaration = WorkloadDeclaration {
    user_process: false,
    foreground: false,
    maintenance: false,
};

fn observe(arbiter: &mut ResourceArbiter, cpu: u8, now: u64) {
    arbiter.observe(
        &ResourceObservation {
            cpu: Some(cpu),
            ..Default::default()
        },
        now,
    );
}

#[test]
fn attempts_are_capped_and_run_resets_the_budget() {
    let mut arbiter = ResourceArbiter::new(ArbiterPolicy {
        max_checkpoint_attempts: 2,
        cooldown_millis: 0,
        ..ArbiterPolicy::default()
    })
    .unwrap();
    let mut state = SuspensionState::new();
    observe(&mut arbiter, 95, 0);
    assert_eq!(
        arbiter.direct(BACKGROUND, &mut state, 0).unwrap().action(),
        Some(ResourceAction::Suspend)
    );
    assert_eq!(
        state.checkpoint_failed(0),
        SuspensionError::CheckpointFailed
    );
    assert_eq!(
        arbiter.direct(BACKGROUND, &mut state, 0).unwrap().action(),
        Some(ResourceAction::Suspend)
    );
    assert_eq!(
        state.checkpoint_failed(0),
        SuspensionError::CheckpointFailed
    );
    let decision = arbiter.direct(BACKGROUND, &mut state, 0).unwrap();
    assert_eq!(decision.action(), None);
    assert_eq!(decision.phase(), SuspensionPhase::Throttled);

    observe(&mut arbiter, 10, 1);
    let decision = arbiter.direct(BACKGROUND, &mut state, 1).unwrap();
    assert_eq!(decision.action(), Some(ResourceAction::Run));
    assert_eq!(decision.phase(), SuspensionPhase::Running);

    observe(&mut arbiter, 95, 2);
    assert_eq!(
        arbiter.direct(BACKGROUND, &mut state, 2).unwrap().action(),
        Some(ResourceAction::Suspend)
    );
}

#[test]
fn resume_resets_the_checkpoint_attempt_budget() {
    let mut arbiter = ResourceArbiter::new(ArbiterPolicy {
        max_checkpoint_attempts: 1,
        cooldown_millis: 0,
        ..ArbiterPolicy::default()
    })
    .unwrap();
    let mut state = SuspensionState::new();
    observe(&mut arbiter, 95, 0);
    assert_eq!(
        arbiter.direct(BACKGROUND, &mut state, 0).unwrap().action(),
        Some(ResourceAction::Suspend)
    );
    assert_eq!(
        state.checkpoint_succeeded(0),
        Ok(SuspensionPhase::Suspended)
    );
    observe(&mut arbiter, 10, 30_000);
    let decision = arbiter.direct(BACKGROUND, &mut state, 30_000).unwrap();
    assert_eq!(decision.action(), Some(ResourceAction::Resume));
    assert_eq!(decision.phase(), SuspensionPhase::Running);
    observe(&mut arbiter, 95, 30_001);
    let decision = arbiter.direct(BACKGROUND, &mut state, 30_001).unwrap();
    assert_eq!(decision.action(), Some(ResourceAction::Suspend));
    assert_eq!(decision.phase(), SuspensionPhase::Checkpointing);
}

#[test]
fn retry_waits_for_the_state_clock_cooldown() {
    let mut arbiter = ResourceArbiter::new(ArbiterPolicy {
        max_checkpoint_attempts: 3,
        cooldown_millis: 1_000,
        ..ArbiterPolicy::default()
    })
    .unwrap();
    let mut state = SuspensionState::new();
    observe(&mut arbiter, 95, 0);
    assert_eq!(
        arbiter.direct(BACKGROUND, &mut state, 0).unwrap().action(),
        Some(ResourceAction::Suspend)
    );
    assert_eq!(
        state.checkpoint_failed(1),
        SuspensionError::CheckpointFailed
    );
    assert_eq!(
        arbiter.direct(BACKGROUND, &mut state, 500).unwrap().action(),
        None
    );
    assert_eq!(
        arbiter.direct(BACKGROUND, &mut state, 500).unwrap().phase(),
        SuspensionPhase::Throttled
    );
    assert_eq!(
        arbiter
            .direct(BACKGROUND, &mut state, 1_001)
            .unwrap()
            .action(),
        Some(ResourceAction::Suspend)
    );
}

#[test]
fn the_checkpointing_window_is_not_interrupted_while_retrying() {
    let mut arbiter = ResourceArbiter::new(ArbiterPolicy {
        max_checkpoint_attempts: 3,
        ..ArbiterPolicy::default()
    })
    .unwrap();
    let mut state = SuspensionState::new();
    observe(&mut arbiter, 95, 0);
    arbiter.direct(BACKGROUND, &mut state, 0).unwrap();
    let decision = arbiter.direct(BACKGROUND, &mut state, 49).unwrap();
    assert_eq!(decision.action(), None);
    assert_eq!(decision.phase(), SuspensionPhase::Checkpointing);
    assert_eq!(
        state.checkpoint_succeeded(50),
        Ok(SuspensionPhase::Suspended)
    );
}

#[test]
fn first_attempt_from_throttled_does_not_wait_for_cooldown() {
    let mut arbiter = ResourceArbiter::new(ArbiterPolicy {
        max_checkpoint_attempts: 3,
        cooldown_millis: 10_000,
        ..ArbiterPolicy::default()
    })
    .unwrap();
    let mut state = SuspensionState::new();
    observe(&mut arbiter, 80, 0);
    assert_eq!(
        arbiter.direct(BACKGROUND, &mut state, 0).unwrap().action(),
        Some(ResourceAction::Throttle)
    );
    assert_eq!(state.phase(), SuspensionPhase::Throttled);
    observe(&mut arbiter, 95, 0);
    let decision = arbiter.direct(BACKGROUND, &mut state, 0).unwrap();
    assert_eq!(
        decision.action(),
        Some(ResourceAction::Suspend),
        "首次尝试 attempts==0 不要求 cooldown"
    );
    assert_eq!(decision.phase(), SuspensionPhase::Checkpointing);
}

#[test]
fn checkpoint_succeeded_does_not_reset_the_attempt_budget() {
    let mut arbiter = ResourceArbiter::new(ArbiterPolicy {
        max_checkpoint_attempts: 2,
        cooldown_millis: 0,
        ..ArbiterPolicy::default()
    })
    .unwrap();
    let mut state = SuspensionState::new();
    observe(&mut arbiter, 95, 0);
    assert_eq!(
        arbiter.direct(BACKGROUND, &mut state, 0).unwrap().action(),
        Some(ResourceAction::Suspend)
    );
    assert_eq!(
        state.checkpoint_failed(0),
        SuspensionError::CheckpointFailed
    );
    assert_eq!(
        arbiter.direct(BACKGROUND, &mut state, 0).unwrap().action(),
        Some(ResourceAction::Suspend)
    );
    assert_eq!(
        state.checkpoint_succeeded(0),
        Ok(SuspensionPhase::Suspended)
    );
    // attempts 不公开；设计 §4.8 明确 Debug 打印尝试数，故用 Debug 断言“成功不重置”。
    assert!(
        format!("{state:?}").contains("attempts: 2"),
        "checkpoint_succeeded 不得重置 attempts：{state:?}"
    );
}

#[test]
fn zero_attempts_only_throttles() {
    let mut arbiter = ResourceArbiter::new(ArbiterPolicy {
        max_checkpoint_attempts: 0,
        ..ArbiterPolicy::default()
    })
    .unwrap();
    let mut state = SuspensionState::new();
    observe(&mut arbiter, 95, 0);
    let decision = arbiter.direct(BACKGROUND, &mut state, 0).unwrap();
    assert_eq!(decision.action(), Some(ResourceAction::Throttle));
    assert_eq!(decision.phase(), SuspensionPhase::Throttled);
    let decision = arbiter.direct(BACKGROUND, &mut state, 0).unwrap();
    assert_eq!(decision.action(), None);
    assert_eq!(decision.phase(), SuspensionPhase::Throttled);

    observe(&mut arbiter, 10, 30_000);
    let decision = arbiter.direct(BACKGROUND, &mut state, 30_000).unwrap();
    assert_eq!(
        decision.action(),
        Some(ResourceAction::Run),
        "压力回 Low 后必须能 Run"
    );
    assert_eq!(decision.phase(), SuspensionPhase::Running);
}
