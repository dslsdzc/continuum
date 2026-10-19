mod support;

const MANIFEST: &str = include_str!("../Cargo.toml");

#[test]
fn the_manifest_is_zero_table_and_gates_the_assembly_target() {
    let manifest_code: String = MANIFEST
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n");
    for forbidden in [
        "continuum-persist",
        "continuum-events",
        "continuum-runtime",
        "continuum-model-registry",
        "continuum-node",
        "continuum-provider",
        "continuum-graph",
        "continuum-budget",
        "continuum-verify",
        "serde",
        "thiserror",
    ] {
        assert!(
            !manifest_code.contains(forbidden),
            "Cargo.toml 出现禁用依赖 {forbidden}"
        );
    }
    assert!(manifest_code.contains("name = \"assembly\""));
    assert!(manifest_code.contains("required-features = [\"assembly\"]"));
    let all: String = support::code_files()
        .iter()
        .map(|(_, text)| text.as_str())
        .collect();
    for token in ["Migration", "Tx", "persist"] {
        assert!(
            !support::contains_token(&all, token),
            "src/ 出现持久化符号 {token}"
        );
    }
}

#[test]
fn runtime_migrations_and_startup_do_not_mention_the_arbiter() {
    for source in [
        include_str!("../../continuum-runtime/src/main.rs"),
        include_str!("../../continuum-runtime/tests/migrations.rs"),
        include_str!("../../continuum-runtime/tests/startup.rs"),
    ] {
        assert!(!source.contains("continuum_arbiter"));
        assert!(!source.contains("continuum-arbiter"));
    }
}
