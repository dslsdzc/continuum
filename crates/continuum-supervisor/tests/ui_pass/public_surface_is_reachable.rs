use continuum_supervisor::AuthorizedTask;

fn inspect(task: &AuthorizedTask) -> (&'static str, bool, bool) {
    (
        task.source().as_str(),
        task.intent().is_some(),
        task.contract().is_some(),
    )
}

fn main() {}
