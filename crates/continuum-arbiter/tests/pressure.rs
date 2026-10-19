use continuum_arbiter::ResourcePressure;

#[test]
fn pressure_text_is_exact_uppercase() {
    assert_eq!(ResourcePressure::parse("HIGH"), Some(ResourcePressure::High));
    assert_eq!(ResourcePressure::parse("HIGHER"), None);
    assert_eq!(ResourcePressure::parse("high"), None);
    for pressure in [
        ResourcePressure::Low, ResourcePressure::Moderate,
        ResourcePressure::High, ResourcePressure::Critical,
    ] {
        assert_eq!(ResourcePressure::parse(pressure.as_str()), Some(pressure));
    }
}

#[test]
fn pressure_has_four_ordered_bands() {
    assert!(ResourcePressure::Low < ResourcePressure::Moderate);
    assert!(ResourcePressure::Moderate < ResourcePressure::High);
    assert!(ResourcePressure::High < ResourcePressure::Critical);
}

#[test]
fn pressure_has_an_exhaustive_match() {
    fn name(pressure: ResourcePressure) -> &'static str {
        match pressure {
            ResourcePressure::Low => "LOW",
            ResourcePressure::Moderate => "MODERATE",
            ResourcePressure::High => "HIGH",
            ResourcePressure::Critical => "CRITICAL",
        }
    }
    assert_eq!(name(ResourcePressure::Critical), "CRITICAL");
}

use continuum_arbiter::{ArbiterPolicy, ResourceArbiter, ResourceObservation, ResourceSummary,
    WorkloadClass};

fn arbiter() -> ResourceArbiter {
    ResourceArbiter::new(ArbiterPolicy::default()).expect("默认策略必须合法")
}

fn pressure_of(cpu: u8) -> ResourcePressure {
    arbiter()
        .observe(&ResourceObservation { cpu: Some(cpu), ..Default::default() }, 0)
        .resource_pressure()
}

#[test]
fn pressure_bands_use_the_max_of_non_none_readings() {
    assert_eq!(pressure_of(0), ResourcePressure::Low);
    assert_eq!(pressure_of(49), ResourcePressure::Low);
    assert_eq!(pressure_of(50), ResourcePressure::Moderate);
    assert_eq!(pressure_of(75), ResourcePressure::High);
    assert_eq!(pressure_of(90), ResourcePressure::Critical);
    assert_eq!(pressure_of(101), ResourcePressure::Critical);
    let observation = ResourceObservation { cpu: Some(10), gpu: Some(100), ..Default::default() };
    let summary = arbiter().observe(&observation, 0);
    assert_eq!(summary.resource_pressure(), ResourcePressure::Critical);
}

#[test]
fn all_none_keeps_pressure_and_clears_the_class() {
    let mut arbiter = arbiter();
    let first = ResourceObservation {
        cpu: Some(95), workload_class: Some(WorkloadClass::Compiler), ..Default::default()
    };
    let summary = arbiter.observe(&first, 0);
    assert_eq!(summary.resource_pressure(), ResourcePressure::Critical);
    assert_eq!(summary.workload_class(), Some(WorkloadClass::Compiler));
    let summary = arbiter.observe(&ResourceObservation::default(), 1);
    assert_eq!(summary.resource_pressure(), ResourcePressure::Critical);
    assert_eq!(summary.workload_class(), None);

    // 读数 Some、class=None 也必须清掉旧类别（不是只在“全 None”分支清）。
    let with_class = ResourceObservation {
        cpu: Some(10), workload_class: Some(WorkloadClass::Compiler), ..Default::default()
    };
    assert_eq!(arbiter.observe(&with_class, 2).workload_class(), Some(WorkloadClass::Compiler));
    let clear_class = ResourceObservation {
        cpu: Some(10), workload_class: None, ..Default::default()
    };
    let summary = arbiter.observe(&clear_class, 3);
    assert_eq!(summary.resource_pressure(), ResourcePressure::Critical, "滞回保持压力");
    assert_eq!(summary.workload_class(), None, "本次 class=None 必须清空旧类别");
}

#[test]
fn summary_has_exactly_two_read_only_accessors() {
    fn reads(summary: &ResourceSummary)
        -> (ResourcePressure, Option<WorkloadClass>) {
        (summary.resource_pressure(), summary.workload_class())
    }
    let _: fn(&ResourceSummary) -> (ResourcePressure, Option<WorkloadClass>) = reads;
}
