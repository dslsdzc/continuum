use continuum_supervisor::SessionPlan;

fn inspect(plan: &SessionPlan) -> (bool, usize, bool, usize, usize, bool) {
    (
        plan.mode() == continuum_supervisor::SessionMode::Auto,
        plan.candidates().len(),
        plan.decision().is_some(),
        plan.selection().map_or(0, |_| 1),
        plan.background().findings().len(),
        plan.basis().is_some(),
    )
}

fn main() {}
