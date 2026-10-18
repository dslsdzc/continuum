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
