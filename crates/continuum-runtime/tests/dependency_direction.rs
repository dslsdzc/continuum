use std::path::Path;
use std::process::Command;

fn cargo_tree(pkg: &str) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    // --edges all 同时覆盖 normal、build、dev。只查 normal 不够：
    // dev-dependency 同样允许测试代码引用 provider 类型，
    // 而 §346 的中立性约束要拦住所有引用路径。
    let out = Command::new(env!("CARGO"))
        .args(["tree", "-p", pkg, "--edges", "all", "--prefix", "none"])
        .current_dir(root)
        .output()
        .expect("cargo tree 无法执行");
    assert!(
        out.status.success(),
        "cargo tree -p {pkg} 失败: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("cargo tree 输出不是 UTF-8")
}

/// 每个包：自身 + 允许依赖的内部 crate。
/// 只查「不含某字符串」不够——P1 新增四个 crate，
/// 必须逐包断言允许集合，否则反向边不会被发现。
const ALLOWED: &[(&str, &[&str])] = &[
    ("continuum-core", &[]),
    ("continuum-events", &["continuum-core"]),
    ("continuum-persist", &["continuum-events"]),
    ("continuum-provider", &["continuum-core"]),
    (
        "continuum-artifact",
        &["continuum-core", "continuum-persist"],
    ),
    ("continuum-port", &["continuum-artifact"]),
    ("continuum-operator", &["continuum-artifact"]),
    (
        "continuum-graph",
        &[
            "continuum-artifact",
            "continuum-port",
            "continuum-operator",
            "continuum-persist",
        ],
    ),
    // 本 task 结束时 runtime 的直接依赖只有这四个。
    // Task 12 会给它加上 continuum-artifact 与 continuum-graph，届时此表要同步扩充。
    (
        "continuum-runtime",
        &[
            "continuum-core",
            "continuum-events",
            "continuum-persist",
            "continuum-provider",
        ],
    ),
];

const ALL_CRATES: &[&str] = &[
    "continuum-core",
    "continuum-events",
    "continuum-persist",
    "continuum-provider",
    "continuum-artifact",
    "continuum-port",
    "continuum-operator",
    "continuum-graph",
    "continuum-runtime",
];

/// 只取直接依赖（`--depth 1`）。
///
/// 既有的 `cargo_tree()` 不带 `--depth`，输出的是整棵传递树，
/// 与 `ALLOWED` 表的语义不符——`ALLOWED` 列的是直接边，
/// 与设计第 5 节的依赖方向一致。直接边足以拦住反向依赖：
/// 任何反向边本身必然是一条直接边。
fn cargo_tree_direct(pkg: &str) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = Command::new(env!("CARGO"))
        .args([
            "tree", "-p", pkg, "--depth", "1", "--edges", "all", "--prefix", "none",
        ])
        .current_dir(root)
        .output()
        .expect("cargo tree 无法执行");
    assert!(
        out.status.success(),
        "cargo tree -p {pkg} 失败: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("cargo tree 输出不是 UTF-8")
}

#[test]
fn every_crate_depends_only_on_its_allowed_set() {
    for (pkg, allowed) in ALLOWED {
        let tree = cargo_tree_direct(pkg);
        for other in ALL_CRATES {
            if other == pkg {
                continue;
            }
            // cargo tree 的 --prefix none 输出每行形如 "continuum-port v0.1.0 (path...)"
            let appeared = tree
                .lines()
                .any(|l| l.split_whitespace().next() == Some(*other));
            assert_eq!(
                appeared,
                allowed.contains(other),
                "{pkg} 对 {other} 的依赖与允许集合不符（出现={appeared}，允许={}）:\n{tree}",
                allowed.contains(other)
            );
        }
    }
}

#[test]
fn core_and_persist_do_not_depend_on_provider() {
    for pkg in ["continuum-core", "continuum-persist"] {
        let tree = cargo_tree(pkg);
        assert!(
            !tree.contains("continuum-provider"),
            "{pkg} 依赖了 continuum-provider，违反 §346:\n{tree}"
        );
    }
}
