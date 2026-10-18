use continuum_arbiter::assembly::{AssemblyView, PriorityTier};
use continuum_arbiter::{
    ArbiterPolicy, ResourceArbiter, ResourcePressure, WorkloadDeclaration,
};

fn arbiter() -> ResourceArbiter {
    ResourceArbiter::new(ArbiterPolicy::default()).unwrap()
}

const USER: WorkloadDeclaration =
    WorkloadDeclaration { user_process: true, foreground: false, maintenance: false };
const FOREGROUND: WorkloadDeclaration =
    WorkloadDeclaration { user_process: false, foreground: true, maintenance: false };
const BACKGROUND: WorkloadDeclaration =
    WorkloadDeclaration { user_process: false, foreground: false, maintenance: false };
const MAINTENANCE: WorkloadDeclaration =
    WorkloadDeclaration { user_process: false, foreground: false, maintenance: true };

#[test]
fn classification_follows_maintenance_user_foreground_default() {
    assert_eq!(arbiter().assembly_view(MAINTENANCE).priority(), PriorityTier::Maintenance);
    assert_eq!(arbiter().assembly_view(USER).priority(), PriorityTier::UserWorkload);
    assert_eq!(arbiter().assembly_view(FOREGROUND).priority(), PriorityTier::ForegroundRuntime);
    assert_eq!(arbiter().assembly_view(BACKGROUND).priority(), PriorityTier::BackgroundRuntime);
    let both = WorkloadDeclaration {
        user_process: true, foreground: true, maintenance: true,
    };
    assert_eq!(arbiter().assembly_view(both).priority(), PriorityTier::Maintenance);
    let user_and_foreground = WorkloadDeclaration {
        user_process: true, foreground: true, maintenance: false,
    };
    assert_eq!(
        arbiter().assembly_view(user_and_foreground).priority(),
        PriorityTier::UserWorkload
    );
}

#[test]
fn priority_tier_text_and_rank_are_verbatim() {
    assert_eq!(PriorityTier::UserWorkload.as_str(), "User workload");
    assert_eq!(PriorityTier::ForegroundRuntime.as_str(), "Foreground Runtime");
    assert_eq!(PriorityTier::BackgroundRuntime.as_str(), "Background Runtime");
    assert_eq!(PriorityTier::Maintenance.as_str(), "Maintenance");
    assert_eq!(PriorityTier::UserWorkload.rank(), 3);
    assert_eq!(PriorityTier::ForegroundRuntime.rank(), 2);
    assert_eq!(PriorityTier::BackgroundRuntime.rank(), 1);
    assert_eq!(PriorityTier::Maintenance.rank(), 0);
}

#[test]
fn assembly_view_exposes_summary_and_priority_only() {
    let arbiter = arbiter();
    let view = arbiter.assembly_view(BACKGROUND);
    fn reads(view: &AssemblyView) -> (continuum_arbiter::ResourceSummary, PriorityTier) {
        (view.summary(), view.priority())
    }
    let _: fn(&AssemblyView) -> (continuum_arbiter::ResourceSummary, PriorityTier) = reads;
    assert_eq!(view.summary().resource_pressure(), ResourcePressure::Low);
}
