use continuum_arbiter::{ArbiterPolicy, ResourceArbiter, ResourceObservation, ResourcePressure};

fn observe(arbiter: &mut ResourceArbiter, cpu: u8, now: u64) -> ResourcePressure {
    arbiter
        .observe(&ResourceObservation { cpu: Some(cpu), ..Default::default() }, now)
        .resource_pressure()
}

#[test]
fn pressure_rises_immediately_and_falls_only_below_resume_at_after_cooldown() {
    let mut arbiter = ResourceArbiter::new(ArbiterPolicy::default()).unwrap();
    assert_eq!(observe(&mut arbiter, 95, 0), ResourcePressure::Critical);
    assert_eq!(observe(&mut arbiter, 10, 10_000), ResourcePressure::Critical);
    assert_eq!(observe(&mut arbiter, 10, 30_000), ResourcePressure::Low);
    assert_eq!(observe(&mut arbiter, 80, 30_001), ResourcePressure::High);
    assert_eq!(observe(&mut arbiter, 80, 31_000), ResourcePressure::High);
}
