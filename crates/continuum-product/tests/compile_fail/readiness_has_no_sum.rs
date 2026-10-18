fn main() {
    let readiness = continuum_product::ProductReadiness::unassessed();
    let _ = readiness.sum();
}
