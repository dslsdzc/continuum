use continuum_arbiter::{ArbiterPolicy, ResourceArbiter, ResourceObservation};

fn main() {
    let mut arbiter = ResourceArbiter::new(ArbiterPolicy::default()).unwrap();
    let summary = arbiter.observe(&ResourceObservation::default(), 0);
    let _ = summary.priority();
}
