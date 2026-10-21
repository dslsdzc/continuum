use continuum_arbiter::{ResourceAction, SuspensionPhase};

#[test]
fn action_text_round_trips_and_has_exactly_four_variants() {
    for action in [
        ResourceAction::Run,
        ResourceAction::Throttle,
        ResourceAction::Suspend,
        ResourceAction::Resume,
    ] {
        assert_eq!(ResourceAction::parse(action.as_str()), Some(action));
    }
    fn name(action: ResourceAction) -> &'static str {
        match action {
            ResourceAction::Run => "run",
            ResourceAction::Throttle => "throttle",
            ResourceAction::Suspend => "suspend",
            ResourceAction::Resume => "resume",
        }
    }
    assert_eq!(name(ResourceAction::Suspend), "suspend");
}

#[test]
fn phase_text_round_trips_and_has_exactly_four_variants() {
    for phase in [
        SuspensionPhase::Running,
        SuspensionPhase::Throttled,
        SuspensionPhase::Checkpointing,
        SuspensionPhase::Suspended,
    ] {
        assert_eq!(SuspensionPhase::parse(phase.as_str()), Some(phase));
    }
    fn name(phase: SuspensionPhase) -> &'static str {
        match phase {
            SuspensionPhase::Running => "Running",
            SuspensionPhase::Throttled => "Throttled",
            SuspensionPhase::Checkpointing => "Checkpointing",
            SuspensionPhase::Suspended => "Suspended",
        }
    }
    assert_eq!(name(SuspensionPhase::Checkpointing), "Checkpointing");
}
