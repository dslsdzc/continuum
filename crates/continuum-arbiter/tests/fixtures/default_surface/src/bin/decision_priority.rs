use continuum_arbiter::{
    ArbiterPolicy, ResourceArbiter, SuspensionState, WorkloadDeclaration,
};

fn main() {
    let arbiter = ResourceArbiter::new(ArbiterPolicy::default()).unwrap();
    let mut state = SuspensionState::new();
    let decision = arbiter
        .direct(
            WorkloadDeclaration {
                user_process: false,
                foreground: false,
                maintenance: false,
            },
            &mut state,
            0,
        )
        .unwrap();
    let _ = decision.priority();
}
