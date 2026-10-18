//! Task 12：装配面照片（S1／S2／S10）。
//!
//! 读的是仓库文件文本；路径以 `CARGO_MANIFEST_DIR` 为锚（crate 在 `crates/` 下）。

use std::path::Path;

fn repo_file(relative: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("读不到 {}: {e}", path.display()))
}

#[test]
fn workspace_manifest_lists_the_crate_once() {
    let manifest = repo_file("Cargo.toml");
    assert_eq!(
        manifest.matches("\"crates/continuum-memory\"").count(),
        1,
        "workspace members 应恰有一行 `crates/continuum-memory`（S1）"
    );
}

#[test]
fn runtime_manifest_depends_on_memory_once() {
    let manifest = repo_file("crates/continuum-runtime/Cargo.toml");
    assert_eq!(
        manifest
            .matches("continuum-memory = { path = \"../continuum-memory\" }")
            .count(),
        1,
        "runtime manifest 应恰有一次 continuum-memory 的 path 依赖（S1／S10）"
    );
}

#[test]
fn allowed_table_has_the_memory_row_and_the_runtime_entry() {
    let text = repo_file("crates/continuum-runtime/tests/dependency_direction.rs");
    assert_eq!(
        text.matches("(\"continuum-memory\", &[\"continuum-persist\"])")
            .count(),
        1,
        "ALLOWED 应恰有一行 (\"continuum-memory\", &[\"continuum-persist\"])（S2）"
    );
    // P6A 的 continuum-context 条目把 continuum-memory 列为四条允许边之一，
    // 故 3 = memory 行 + runtime 条目 + P6A 条目。
    assert_eq!(
        text.matches("\"continuum-memory\",").count(),
        3,
        "`\"continuum-memory\",` 应恰出现三次：memory 行、runtime 条目与 P6A 条目各一次（S2）"
    );

    let start = text
        .find("(\"continuum-runtime\",")
        .expect("ALLOWED 里应有 continuum-runtime 行");
    let block = &text[start..];
    let block = &block[..block.find("],").expect("runtime 数组应有结尾")];
    assert_eq!(
        block.matches("\"continuum-memory\",").count(),
        1,
        "continuum-runtime 的 ALLOWED 数组应恰有一项 continuum-memory（S2）"
    );
}

#[test]
fn runtime_registers_memory_migrations_once() {
    let main = repo_file("crates/continuum-runtime/src/main.rs");
    assert_eq!(
        main.matches("p6c_memory_migrations()").count(),
        1,
        "runtime_migrations() 应恰注册 p6c_memory_migrations() 一次（S10）"
    );
}

#[test]
fn memory_crate_has_only_the_persist_internal_edge() {
    let manifest = repo_file("crates/continuum-memory/Cargo.toml");
    let start = manifest.find("[dependencies]").expect("应有 [dependencies] 段");
    let rest = &manifest[start + "[dependencies]".len()..];
    let end = rest
        .find("\n[")
        .map(|index| start + "[dependencies]".len() + index)
        .unwrap_or(manifest.len());
    let dependencies = &manifest[start..end];

    let internal: Vec<&str> = dependencies
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("continuum-"))
        .map(|line| line.split('=').next().unwrap_or("").trim())
        .collect();
    assert_eq!(
        internal,
        ["continuum-persist"],
        "`[dependencies]` 段内 workspace 内部 crate 只能有 continuum-persist（S2）"
    );
}
