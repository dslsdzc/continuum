//! 迁移编号与名称的唯一性，以及「注册的集合」与「实际应用的集合」一致。
//!
//! 本文件与 `startup.rs` 的分工：这里的断言**直接列出期望的迁移集合**，不依赖
//! 任何计数；`startup.rs` 那几处「迁移应用 N 项」抓的是另一件事——启动输出确实
//! 报了那么多项。两处都在，不是重复：计数断言的判别力依赖「作者按实际输出改
//! 断言」，数从 6 变 7 时把它改成 6 同样能过，而后果是新表根本没建。本文件的
//! 集合比对不受这一点影响。

use continuum_persist::{Db, Migration, Value, builtin_migrations};
use std::process::Command;

/// 期望的迁移集合：P0 内建 + P1（artifact、graph）+ P2（workspace、policy、effect）
/// + P3（capability 的 `tool` 表；子项目 D 的三张模型表）。
///
/// `policy` 由 Task 10 加上（第 7 步的集成路径要读它），`effect` 由 Task 11 加上
/// （第 4 步要写效应记录），`tool` 由 P3 的 Task 4 加上（Task 5 的 `authorize` 要读它），
/// D 的三张表（`model_registry` / `model_profile` / `model_skill_score`）由 P3 子项目 D
/// 的 Task 5 加上——每张表由「用它的那个 task」注册，见 `main.rs` 装配处的说明。
///
/// **这是 `main.rs` 装配处的第二份转录**。转录的风险由下面的比对抵掉——
/// `the_runtime_applies_…` 拿**实际启动后**落在 `schema_migrations` 里的集合与
/// 本函数对照，故 `main.rs` 少注册或多注册一条都会变红，而不是自查自洽。
///
/// **下面是二进制而不是 import 那份清单**：本用例要观察的是**实际落库**的集合，
/// 不是装配处写了什么——「注册了但没跑」（编号撞车被跳过、SQL 失败被吞）这一路
/// 只有落库侧看得见。故即便那份清单被移进 lib、import 得到，本用例也照旧驱动
/// 二进制。（`continuum-runtime` 现在确有 lib target，但只导出命令行解析，
/// 见 `src/lib.rs`；装配清单不在其中。）
fn expected_migrations() -> Vec<Migration> {
    let mut m = builtin_migrations();
    m.extend(continuum_artifact::p1_artifact_migrations());
    m.extend(continuum_graph::p1_graph_migrations());
    m.extend(continuum_workspace::p2_workspace_migrations());
    m.extend(continuum_policy::p2_policy_migrations());
    m.extend(continuum_effect::p2_effect_migrations());
    m.extend(continuum_capability::p3_capability_migrations());
    m.extend(continuum_model_registry::p3d_model_migrations());
    m
}

/// 编号两两不重复。
///
/// 重复的后果是静默的：`schema_migrations` 按 version 去重，后一条会被当作
/// 已应用而跳过——它的表永远不会建出来，且任何计数断言都可能被顺手改成碰巧
/// 对上的值。故这里直接对编号本身下断言。
#[test]
fn migration_versions_are_unique() {
    let mut versions: Vec<i64> = expected_migrations().iter().map(|m| m.version).collect();
    versions.sort_unstable();
    for pair in versions.windows(2) {
        assert_ne!(
            pair[0], pair[1],
            "迁移编号重复：{pair:?}——后一条会被当作已应用而跳过，它的表永远不会建出来"
        );
    }
}

#[test]
fn migration_names_are_unique() {
    let mut names: Vec<&str> = expected_migrations().iter().map(|m| m.name).collect();
    names.sort_unstable();
    for pair in names.windows(2) {
        assert_ne!(pair[0], pair[1], "迁移名称重复：{pair:?}");
    }
}

/// 启动一次真实的 runtime，断言库里记下的迁移集合恰是期望集合。
///
/// 同时抓两个方向：「注册了但没跑」（编号撞车被跳过、SQL 失败被吞）与
/// 「跑了但没注册」（`main.rs` 与 `startup.rs` 之外还有别的装配路径）。
#[test]
fn the_runtime_applies_exactly_the_expected_migration_set() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let out = Command::new(env!("CARGO_BIN_EXE_continuum-runtime"))
        .args(["recover", "--db"])
        .arg(&path)
        .output()
        .expect("continuum-runtime 无法执行");
    assert!(
        out.status.success(),
        "启动失败: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // 用只带 P0 内建迁移的连接读回**实际应用**的集合：`Db::open` 不跑迁移，
    // 故这里看到的就是上面那次启动写下的内容。
    let db = Db::open(&path).unwrap();
    let tx = db.begin().unwrap();
    let rows = tx
        .query(
            "SELECT version, name FROM schema_migrations ORDER BY version",
            &[],
        )
        .unwrap();
    let applied: Vec<(i64, String)> = rows
        .iter()
        .map(|r| {
            let version = match &r[0] {
                Value::Int(i) => *i,
                other => panic!("version 应为整数，实际 {other:?}"),
            };
            let name = match &r[1] {
                Value::Text(s) => s.clone(),
                other => panic!("name 应为文本，实际 {other:?}"),
            };
            (version, name)
        })
        .collect();

    let mut expected: Vec<(i64, String)> = expected_migrations()
        .iter()
        .map(|m| (m.version, m.name.to_owned()))
        .collect();
    expected.sort();

    assert_eq!(
        applied, expected,
        "库里应用的迁移集合与注册的不一致（注册了没跑，或跑了没注册）"
    );
}
