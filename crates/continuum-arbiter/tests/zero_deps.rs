use std::process::Command;

#[test]
fn the_crate_has_no_dependencies() {
    let output = Command::new(env!("CARGO"))
        .args([
            "tree", "-p", "continuum-arbiter", "--depth", "1",
            "--edges", "all", "--prefix", "none",
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("cargo tree 运行失败");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let tree = String::from_utf8(output.stdout).expect("cargo tree 输出不是 UTF-8");
    let lines: Vec<&str> = tree.lines().filter(|line| !line.trim().is_empty()).collect();
    assert_eq!(lines.len(), 1, "依赖树必须只有本 crate 一行：\n{tree}");
    assert!(lines[0].starts_with("continuum-arbiter v"), "第一行不是本 crate：{tree}");
}

#[test]
fn the_manifest_has_empty_dependency_sections_and_the_allowed_row_is_empty() {
    let manifest = include_str!("../Cargo.toml");
    for section in ["dependencies", "dev-dependencies"] {
        let body = manifest
            .split(&format!("[{section}]"))
            .nth(1)
            .unwrap_or_else(|| panic!("Cargo.toml 缺少 [{section}]"))
            .split("\n[")
            .next()
            .expect("section 体为空");
        let entries: Vec<&str> = body
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .collect();
        assert!(entries.is_empty(), "[{section}] 必须为空：{entries:?}");
    }
    let allowed = include_str!("../../continuum-runtime/tests/dependency_direction.rs");
    let row = allowed
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with("(\"continuum-arbiter\","))
        .expect("ALLOWED 缺少 continuum-arbiter 行");
    assert_eq!(row, "(\"continuum-arbiter\", &[]),");
}
