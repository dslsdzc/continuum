use continuum_arbiter::{
    ArbiterPolicy, ResourceAction, ResourceArbiter, ResourceObservation, SuspensionPhase,
    SuspensionState, WorkloadDeclaration,
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
fn high_pressure_limits_background_and_maintenance_without_any_model_path() {
    let arbiter = critical();
    let mut background = SuspensionState::new();
    let decision = arbiter
        .direct(
            WorkloadDeclaration {
                user_process: false,
                foreground: false,
                maintenance: false,
            },
            &mut background,
            0,
        )
        .unwrap();
    assert_eq!(decision.action(), Some(ResourceAction::Suspend));
    assert_eq!(decision.phase(), SuspensionPhase::Checkpointing);

    let mut maintenance = SuspensionState::new();
    let decision = arbiter
        .direct(
            WorkloadDeclaration {
                user_process: false,
                foreground: false,
                maintenance: true,
            },
            &mut maintenance,
            0,
        )
        .unwrap();
    assert_eq!(decision.action(), Some(ResourceAction::Suspend));
    assert_eq!(decision.phase(), SuspensionPhase::Checkpointing);

    let mut user = SuspensionState::new();
    let decision = arbiter
        .direct(
            WorkloadDeclaration {
                user_process: true,
                foreground: false,
                maintenance: false,
            },
            &mut user,
            0,
        )
        .unwrap();
    assert_eq!(decision.action(), None);
    assert_eq!(decision.phase(), SuspensionPhase::Running);
}

#[test]
fn per_task_state_is_required_for_independent_decisions() {
    let arbiter = critical();
    let mut reused = SuspensionState::new();
    let background = WorkloadDeclaration {
        user_process: false,
        foreground: false,
        maintenance: false,
    };
    assert_eq!(
        arbiter
            .direct(background, &mut reused, 0)
            .unwrap()
            .phase(),
        SuspensionPhase::Checkpointing
    );
    let maintenance = WorkloadDeclaration {
        user_process: false,
        foreground: false,
        maintenance: true,
    };
    assert_eq!(
        arbiter
            .direct(maintenance, &mut reused, 0)
            .unwrap()
            .action(),
        None
    );
    assert_eq!(reused.phase(), SuspensionPhase::Checkpointing);

    let mut fresh = SuspensionState::new();
    assert_eq!(
        arbiter
            .direct(maintenance, &mut fresh, 0)
            .unwrap()
            .action(),
        Some(ResourceAction::Suspend)
    );
    assert_eq!(fresh.phase(), SuspensionPhase::Checkpointing);
}
