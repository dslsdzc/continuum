use continuum_supervisor::{AuthorizedTask, TaskSource};

fn main() {
    let _ = AuthorizedTask::new(TaskSource::Policy, todo!());
}
