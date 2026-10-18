const MANIFEST: &str = include_str!("../Cargo.toml");

fn keys_in(manifest: &str, section: &str) -> Vec<String> {
    let mut in_section = false;
    let mut keys = Vec::new();
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_section = line == section;
            continue;
        }
        if !in_section || line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, _)) = line.split_once('=') {
            keys.push(key.trim().to_owned());
        }
    }
    keys.sort();
    keys
}

#[test]
fn the_manifest_declares_exactly_three_allowed_edges() {
    assert_eq!(
        keys_in(MANIFEST, "[dependencies]"),
        vec!["continuum-graph", "continuum-semantics"]
    );
    assert_eq!(
        keys_in(MANIFEST, "[dev-dependencies]"),
        vec!["continuum-canonical"]
    );
}

#[test]
fn the_manifest_declares_no_persistence_resource_or_runtime_edge() {
    let all = [keys_in(MANIFEST, "[dependencies]"), keys_in(MANIFEST, "[dev-dependencies]")].concat();
    for forbidden in [
        "continuum-arbiter", "continuum-events", "continuum-model-registry", "continuum-node",
        "continuum-persist", "continuum-product", "continuum-provider", "continuum-runtime",
        "continuum-supervisor", "serde", "serde_json", "thiserror", "trybuild",
    ] {
        assert!(!all.iter().any(|key| key == forbidden), "Cargo.toml 不得登记 {forbidden}");
    }
}
