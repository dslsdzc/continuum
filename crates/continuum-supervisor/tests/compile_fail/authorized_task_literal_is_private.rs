use continuum_supervisor::{AuthorizedTask, TaskSource};

fn main() {
    let _ = AuthorizedTask {
        source: TaskSource::Policy,
        evidence: todo!(),
    };
}
