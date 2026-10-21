use continuum_supervisor::{AuthorizedTask, PolicyGrantRef};

fn main() {
    let grant = PolicyGrantRef::new("issuer", "grant").unwrap();
    let _ = AuthorizedTask::maintenance(grant);
}
