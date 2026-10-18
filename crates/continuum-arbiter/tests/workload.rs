use continuum_arbiter::WorkloadClass;

#[test]
fn the_class_set_is_exactly_four_and_round_trips() {
    assert_eq!(WorkloadClass::ALL.len(), 4);
    for class in WorkloadClass::ALL {
        assert_eq!(WorkloadClass::parse(class.as_str()), Some(class));
    }
}

#[test]
fn interactive_is_not_a_fifth_class() {
    assert_eq!(WorkloadClass::parse("INTERACTIVE"), None);
    assert_eq!(WorkloadClass::parse("interactive_game"), None);
}

#[test]
fn the_exhaustive_match_has_no_fifth_arm() {
    fn name(class: WorkloadClass) -> &'static str {
        match class {
            WorkloadClass::InteractiveGame => "INTERACTIVE_GAME",
            WorkloadClass::ComputeWorkload => "COMPUTE_WORKLOAD",
            WorkloadClass::Compiler => "COMPILER",
            WorkloadClass::VideoRender => "VIDEO_RENDER",
        }
    }
    assert_eq!(name(WorkloadClass::Compiler), "COMPILER");
}
