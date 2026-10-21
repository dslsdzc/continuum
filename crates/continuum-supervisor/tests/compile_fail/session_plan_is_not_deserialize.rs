fn main() {
    let _: continuum_supervisor::SessionPlan = serde_json::from_str("{}").expect("空对象");
}
