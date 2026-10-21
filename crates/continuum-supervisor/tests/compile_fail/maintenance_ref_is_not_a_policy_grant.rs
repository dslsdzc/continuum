use continuum_supervisor::{AuthorizedTask, MaintenanceOccurrenceRef};

fn main() {
    let occurrence = MaintenanceOccurrenceRef::new("schedule", "occurrence").unwrap();
    let _ = AuthorizedTask::policy(occurrence);
}
