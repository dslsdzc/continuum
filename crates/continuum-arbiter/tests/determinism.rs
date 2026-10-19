use continuum_arbiter::{
    ArbiterPolicy, ResourceAction, ResourceArbiter, ResourceDecision, ResourceObservation,
    ResourcePressure, SuspensionPhase, SuspensionState, WorkloadDeclaration,
};

const BACKGROUND: WorkloadDeclaration = WorkloadDeclaration {
    user_process: false,
    foreground: false,
    maintenance: false,
};

fn run_once() -> (
    Vec<ResourcePressure>,
    Vec<ResourceDecision>,
    Vec<SuspensionPhase>,
) {
    let mut arbiter = ResourceArbiter::new(ArbiterPolicy::default()).unwrap();
    let mut state = SuspensionState::new();
    let mut pressures = Vec::new();
    let mut decisions = Vec::new();
    let mut phases = Vec::new();
    for (cpu, now) in [(95, 0), (10, 10_000), (10, 30_000), (95, 30_001)] {
        pressures.push(
            arbiter
                .observe(
                    &ResourceObservation {
                        cpu: Some(cpu),
                        ..Default::default()
                    },
                    now,
                )
                .resource_pressure(),
        );
        decisions.push(arbiter.direct(BACKGROUND, &mut state, now).unwrap());
        phases.push(state.phase());
    }
    (pressures, decisions, phases)
}

#[test]
fn the_same_sequence_yields_equal_values() {
    assert_eq!(run_once(), run_once());
}

#[test]
fn action_and_phase_text_round_trip() {
    for action in [
        ResourceAction::Run,
        ResourceAction::Throttle,
        ResourceAction::Suspend,
        ResourceAction::Resume,
    ] {
        assert_eq!(ResourceAction::parse(action.as_str()), Some(action));
    }
    for phase in [
        SuspensionPhase::Running,
        SuspensionPhase::Throttled,
        SuspensionPhase::Checkpointing,
        SuspensionPhase::Suspended,
    ] {
        assert_eq!(SuspensionPhase::parse(phase.as_str()), Some(phase));
    }
}
