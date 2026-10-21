use std::path::Path;
use std::process::Command;

fn run_bin(name: &str) -> String {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/default_surface");
    // CARGO_MANIFEST_DIR 是绝对路径，因此 ../../target 解析到 workspace 根 target/。
    let target =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/ui/default_surface");
    let output = Command::new(env!("CARGO"))
        .args(["check", "--quiet", "--bin", name])
        .current_dir(fixture)
        .env("CARGO_TARGET_DIR", target)
        .output()
        .expect("夹具 cargo check 运行失败");
    assert!(!output.status.success(), "{name} 不应在默认构建下通过");
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn default_surface_rejects_hidden_priority_and_private_literals() {
    assert!(run_bin("priority").contains("E0432"));
    assert!(run_bin("assembly_view").contains("E0432"));
    assert!(run_bin("summary_priority").contains("E0599"));
    assert!(run_bin("summary_literal").contains("E0451"));
    assert!(run_bin("decision_priority").contains("E0599"));
}
