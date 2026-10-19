use continuum_arbiter::{ArbiterPolicy, PolicyError, ResourceArbiter, ResourcePressure};

#[test]
fn defaults_are_the_documented_values() {
    let policy = ArbiterPolicy::default();
    assert_eq!(policy.moderate_at, 50);
    assert_eq!(policy.high_at, 75);
    assert_eq!(policy.critical_at, 90);
    assert_eq!(policy.resume_at, ResourcePressure::Low);
    assert_eq!(policy.cooldown_millis, 30_000);
    assert_eq!(policy.checkpoint_timeout_millis, 60_000);
    assert_eq!(policy.max_checkpoint_attempts, 3);
}

#[test]
fn non_increasing_thresholds_are_rejected() {
    for (moderate_at, high_at, critical_at) in
        [(0, 75, 90), (80, 70, 90), (80, 90, 90), (80, 90, 101)]
    {
        let policy = ArbiterPolicy {
            moderate_at, high_at, critical_at, ..ArbiterPolicy::default()
        };
        assert_eq!(
            ResourceArbiter::new(policy).unwrap_err(),
            PolicyError::ThresholdsNotIncreasing
        );
    }
}

#[test]
fn resume_at_must_be_below_high() {
    let high = ArbiterPolicy { resume_at: ResourcePressure::High, ..ArbiterPolicy::default() };
    assert_eq!(ResourceArbiter::new(high).unwrap_err(), PolicyError::ResumeAtTooHigh);
    let moderate = ArbiterPolicy { resume_at: ResourcePressure::Moderate, ..ArbiterPolicy::default() };
    assert!(ResourceArbiter::new(moderate).is_ok());
}
