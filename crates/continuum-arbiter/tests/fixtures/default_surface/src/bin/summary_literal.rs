use continuum_arbiter::{ResourcePressure, ResourceSummary};

fn main() {
    let _ = ResourceSummary {
        resource_pressure: ResourcePressure::Low,
        workload_class: None,
    };
}
