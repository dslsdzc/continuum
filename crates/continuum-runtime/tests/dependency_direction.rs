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
    // Task 15 起 artifact 直接依赖 continuum-events：`save_artifact` 与元数据
    // 同事务写入 ArtifactCreated（§16）。
    (
        "continuum-artifact",
        &["continuum-core", "continuum-events", "continuum-persist"],
    ),
    ("continuum-port", &["continuum-artifact"]),
    ("continuum-operator", &["continuum-artifact"]),
    // Task 15 起 graph 直接依赖 continuum-events（`apply_transition` 在迁移的
    // 同一事务内写 NodeStarted / NodeCompleted），dev-dependencies 同一条。
    (
        "continuum-graph",
        &[
            "continuum-artifact",
            "continuum-events",
            "continuum-port",
            "continuum-operator",
            "continuum-persist",
        ],
    ),
    // P2 边界层：Workspace 抽象与只读强制。events 与 persist 由设计第 4 节要求——
    // Gate 的审计记录与 Task Workspace 的元数据都要落库。
    // 不含 continuum-core：本 crate 至今没有用到它，而本表与 Cargo.toml 必须精确一致。
    (
        "continuum-workspace",
        &["continuum-events", "continuum-persist"],
    ),
    // P2 边界层的进程与内核层：子进程隔离。
    // 只依赖 workspace 抽象（spawn 的参数是 `&TaskWorkspace`，见设计第 4.2 节），
    // 不依赖 P2 其余 crate——沙箱不知道 Effect Journal 与 Policy 的存在。
    // 不含 continuum-core：本 crate 至今没有用到它，而本表与 Cargo.toml 必须精确一致。
    ("continuum-sandbox", &["continuum-workspace"]),
    // P2 边界层的效应记录层。persist 用于 Journal 落库、events 用于状态变更的审计
    // 记录（设计上篇第 7.3 节：状态变更与对应审计记录在同一事务内提交），
    // core 为 id 与时间的既有类型来源。本 task 只建记录体与状态机，
    // 三个依赖的使用点分别在 Task 2 与驱动。
    (
        "continuum-effect",
        &["continuum-core", "continuum-events", "continuum-persist"],
    ),
    // Task 12 起 runtime 直接依赖这几个：artifact 与 graph 用于在启动流程里
    // 注册 P1 迁移，workspace 用于注册 P2 的 workspace 表迁移（Task 4）。
    //
    // 本表是设计第 3 节「依赖方向」的允许集合按**已存在的 crate** 转录，不是按使用点派生，
    // 故它容忍「允许但尚无使用点」的边。这一条对 sandbox 与 core / events / provider 都成立：
    // sandbox 的使用点是驱动（设计第 10 节）装配 Sandbox，而驱动不在 P2 上篇；
    // core / events / provider 在 runtime 内至今无任何引用（`grep -rn 'continuum_core\|…'` 无命中）。
    // 反向的边（真依赖却没登记）由本文件的断言抓住，不会静默。
    (
        "continuum-runtime",
        &[
            "continuum-artifact",
            "continuum-core",
            "continuum-events",
            "continuum-graph",
            "continuum-persist",
            "continuum-provider",
            "continuum-sandbox",
            "continuum-workspace",
        ],
    ),
];

/// workspace 成员名单，派生自 cargo 自身解析出的 workspace 定义。
///
/// 这份名单以前是手工维护的常量。手工名单的全部问题是它会随仓库长大而变坏：
/// 新增 crate 若漏列，它既不会被当作反向边的目标检查（不在内层循环的迭代域里），
/// 也不会作为主体被检查（外层循环走的是 `ALLOWED`）——两个方向同时静默漏检，
/// 而本文件的两个用例仍然全绿。
///
/// 故改为派生：名单的来源只有一个（workspace 定义本身），不可能与仓库不同步。
/// `ALLOWED` 表不随之派生——它列的是设计第 5 节的允许集合，是规范的直接转录。
fn workspace_crates() -> Vec<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    // `--workspace` 以全部成员为根，`--depth 0` 只打印各根自身，每行形如
    // "continuum-port v0.1.0 (/path/to/crate)"；`--prefix none` 去掉树形前缀。
    // 用 `cargo tree` 而非 `cargo metadata`：本文件的解析口径已经是前者的输出格式
    // （见 `cargo_tree_direct`），且无需为解析 JSON 新增依赖。
    let out = Command::new(env!("CARGO"))
        .args(["tree", "--workspace", "--depth", "0", "--prefix", "none"])
        .current_dir(root)
        .output()
        .expect("cargo tree 无法执行");
    assert!(
        out.status.success(),
        "cargo tree --workspace 失败: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).expect("cargo tree 输出不是 UTF-8");
    let mut names: Vec<String> = stdout
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        .map(str::to_string)
        .collect();
    names.sort();
    assert!(
        !names.is_empty(),
        "cargo tree --workspace 未给出任何成员：输出格式已变，或 workspace 为空"
    );
    names
}

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
    let all_crates = workspace_crates();

    // 推导出的名单与手工维护的 `ALLOWED` 必须互为覆盖。少了这一步，派生只解决一半：
    // 新 crate 会被当作反向边的目标检查，却不作为主体被检查——它的依赖方向仍然无人过问。
    let subjects: Vec<&str> = ALLOWED.iter().map(|(pkg, _)| *pkg).collect();
    for name in &all_crates {
        assert!(
            subjects.contains(&name.as_str()),
            "workspace 成员 {name} 未列入 ALLOWED，它的依赖方向不会被检查"
        );
    }
    for pkg in &subjects {
        assert!(
            all_crates.iter().any(|n| n == pkg),
            "ALLOWED 列出的 {pkg} 不是 workspace 成员：表已过期"
        );
    }

    for (pkg, allowed) in ALLOWED {
        let tree = cargo_tree_direct(pkg);
        for other in &all_crates {
            let other = other.as_str();
            if other == *pkg {
                continue;
            }
            // cargo tree 的 --prefix none 输出每行形如 "continuum-port v0.1.0 (path...)"
            let appeared = tree
                .lines()
                .any(|l| l.split_whitespace().next() == Some(other));
            assert_eq!(
                appeared,
                allowed.contains(&other),
                "{pkg} 对 {other} 的依赖与允许集合不符（出现={appeared}，允许={}）:\n{tree}",
                allowed.contains(&other)
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
