//! J19：依赖面、源码面与 P6／190 未改动的照片。
//!
//! 语料是 `src/`（随 `cargo test` 一起跑）；注释行与 `use` 行不计。
//! 这是「不建第二份／不越层」的否定面，不是风格检查。

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn src_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn source_files() -> Vec<(PathBuf, String)> {
    let mut files: Vec<(PathBuf, String)> = std::fs::read_dir(src_dir())
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("rs"))
        .map(|path| {
            let source = std::fs::read_to_string(&path).unwrap();
            (path, source)
        })
        .collect();
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files
}

fn code_lines(source: &str) -> impl Iterator<Item = (usize, &str)> {
    source.lines().enumerate().filter_map(|(index, line)| {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") || trimmed.starts_with("use ") {
            None
        } else {
            Some((index + 1, line))
        }
    })
}

/// 生产部分（第一个 `#[cfg(test)]` 之前）与测试部分。
fn split_test(source: &str) -> (&str, &str) {
    match source.find("#[cfg(test)]") {
        Some(index) => (&source[..index], &source[index..]),
        None => (source, ""),
    }
}

fn count(haystack: &str, needle: &str) -> usize {
    haystack.match_indices(needle).count()
}

#[test]
fn there_is_no_second_product_id_intent_status_evidence_completion_or_decision() {
    for (path, source) in source_files() {
        for needle in [
            "struct ProductId",
            "enum IntentStatus",
            "struct Evidence",
            "enum CompletionVerdict",
            "struct Decision",
        ] {
            assert!(
                !source.contains(needle),
                "{} 出现第二份类型定义：{needle}",
                path.display()
            );
        }
    }
}

#[test]
fn the_product_has_a_single_private_construction_point() {
    let mut allowed_product_braces = 0;
    for (path, source) in source_files() {
        for (line_number, line) in source.lines().enumerate() {
            let mut rest = line;
            while let Some(index) = rest.find("Product {") {
                let before = rest[..index].chars().next_back();
                let is_identifier_tail = before
                    .map(|c| c.is_ascii_alphanumeric() || c == '_')
                    .unwrap_or(false);
                let is_variant_path = rest[..index].ends_with("::");
                let is_return_type = line.contains("-> Product {");
                if !is_identifier_tail && !is_variant_path && !is_return_type {
                    let allowed = line.contains("struct Product {")
                        || line.contains("impl Product {");
                    assert!(
                        allowed,
                        "{}:{} 出现未登记的 Product 构造面：{line}",
                        path.display(),
                        line_number + 1
                    );
                    allowed_product_braces += 1;
                }
                rest = &rest[index + "Product {".len()..];
            }
        }
    }
    assert_eq!(allowed_product_braces, 3, "Product {{ 的允许清单是三处");

    let mut production_calls = 0;
    let mut test_calls = 0;
    for (path, source) in source_files() {
        let (production, test) = split_test(&source);
        production_calls += count(production, "Product::from_verified_parts(");
        test_calls += count(test, "Product::from_verified_parts(");
        if test.contains("Product::from_verified_parts(") {
            let name = path.file_name().unwrap().to_str().unwrap();
            assert!(
                ["persist.rs", "lower.rs", "shippable.rs"].contains(&name),
                "{name} 的 #[cfg(test)] 块里出现未登记的 from_verified_parts 调用"
            );
        }
    }
    assert_eq!(production_calls, 2, "生产 exactly 2 处");
    // persist／lower 各一处；shippable 的 Task 5 测试夹具两处。全部在 #[cfg(test)] 块内。
    assert_eq!(test_calls, 4, "测试 exactly 4 处");

    let product = std::fs::read_to_string(src_dir().join("product.rs")).unwrap();
    let (production, test) = split_test(&product);
    let constructions = |source: &str| {
        source
            .lines()
            .filter(|line| line.contains("Self {") && !line.contains("-> Self {"))
            .count()
    };
    assert_eq!(constructions(production), 1, "生产 Self {{ 构造点唯一");
    assert_eq!(constructions(test), 1, "测试 Self {{ 构造点唯一");
}

#[test]
fn there_is_no_direct_sql_against_intent_decision_or_evidence() {
    for (path, source) in source_files() {
        for needle in ["FROM intent", "FROM decision", "FROM evidence"] {
            assert!(
                !source.contains(needle),
                "{} 出现直接 SQL：{needle}",
                path.display()
            );
        }
    }
}

#[test]
fn there_are_no_planner_replan_contract_or_plan_writes() {
    for (path, source) in source_files() {
        for (line_number, line) in code_lines(&source) {
            for needle in ["Planner", "replan(", "save_contract", "save_plan"] {
                assert!(
                    !line.contains(needle),
                    "{}:{} 出现 {needle}：{line}",
                    path.display(),
                    line_number
                );
            }
        }
    }
}

#[test]
fn there_is_no_full_future_milestone_api() {
    for (path, source) in source_files() {
        for needle in ["all_milestones", "full_plan"] {
            assert!(
                !source.contains(needle),
                "{} 出现完整未来 API：{needle}",
                path.display()
            );
        }
    }
}

#[test]
fn there_is_no_readiness_aggregate_method() {
    for (path, source) in source_files() {
        for needle in ["fn score", "fn average", "fn sum", "fn aggregate"] {
            assert!(
                !source.contains(needle),
                "{} 出现聚合方法：{needle}",
                path.display()
            );
        }
    }
}

#[test]
fn the_only_intent_transition_call_is_retire() {
    for (path, source) in source_files() {
        for (line_number, line) in code_lines(&source) {
            if !line.contains("apply_intent_transition") {
                continue;
            }
            let name = path.file_name().unwrap().to_str().unwrap();
            assert_eq!(name, "handoff.rs", "状态迁移出现在 {name}:{line_number}");
        }
    }
    let handoff = std::fs::read_to_string(src_dir().join("handoff.rs")).unwrap();
    let calls = code_lines(&handoff)
        .filter(|(_, line)| line.contains("apply_intent_transition("))
        .count();
    assert_eq!(calls, 1, "apply_intent_transition 只允许 retire 一处调用");
}

#[test]
fn there_is_no_direct_event_append() {
    for (path, source) in source_files() {
        for (line_number, line) in code_lines(&source) {
            assert!(
                !line.contains("append_event"),
                "{}:{} 直接写事件：{line}",
                path.display(),
                line_number
            );
        }
    }
}

fn cargo_dependencies() -> BTreeSet<String> {
    let manifest = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
    )
    .unwrap();
    // 只看 [dependencies] 段；[dev-dependencies] 不计入运行期边。
    let start = manifest.find("[dependencies]").unwrap() + "[dependencies]".len();
    let rest = &manifest[start..];
    let end = rest.find("\n[").unwrap_or(rest.len());
    rest[..end]
        .lines()
        .filter_map(|line| {
            let name = line.split('=').next()?.trim();
            if name.starts_with("continuum-") {
                Some(name.to_owned())
            } else {
                None
            }
        })
        .collect()
}

#[test]
fn the_crate_has_exactly_the_four_allowed_workspace_dependencies() {
    let expected: BTreeSet<String> = [
        "continuum-canonical",
        "continuum-persist",
        "continuum-semantics",
        "continuum-verify",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    assert_eq!(cargo_dependencies(), expected);
}

#[test]
fn the_product_crate_does_not_depend_on_runtime_provider_registry_node_or_context() {
    let dependencies = cargo_dependencies();
    for forbidden in [
        "continuum-runtime",
        "continuum-provider",
        "continuum-model-registry",
        "continuum-node",
        "continuum-context",
    ] {
        assert!(!dependencies.contains(forbidden), "出现越层依赖 {forbidden}");
    }
}

#[test]
fn the_allowed_row_is_exactly_four_entries() {
    let source = include_str!("../../continuum-runtime/tests/dependency_direction.rs");
    let start = source
        .find("(\"continuum-product\", &[")
        .expect("ALLOWED 缺少 continuum-product 行");
    let rest = &source[start..];
    let end = rest.find("])").expect("continuum-product 行未闭合");
    let row = &rest[..end];
    let entries: Vec<&str> = row
        .split('"')
        .skip(1)
        .step_by(2)
        .filter(|item| item.starts_with("continuum-") && *item != "continuum-product")
        .collect();
    assert_eq!(
        entries,
        [
            "continuum-canonical",
            "continuum-persist",
            "continuum-semantics",
            "continuum-verify"
        ]
    );
}

fn git_diff_is_empty(path: &str) {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let status = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["diff", "--exit-code", "c89d839", "--", path])
        .status()
        .expect("git 可执行");
    assert!(status.success(), "{path} 相对 c89d839 有改动");
}

#[test]
fn the_p6_fixture_is_untouched() {
    git_diff_is_empty("crates/continuum-runtime/tests/fixtures/p6_pre_allowed.txt");
    let source = include_str!("../../continuum-runtime/tests/dependency_direction.rs");
    let start = source.find("const P6_EXPECTED_ROWS").unwrap();
    let rest = &source[start..];
    let end = rest.find("\n];").unwrap();
    let block = &rest[..end];
    let entries = block
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            trimmed == "(" || trimmed.starts_with("(\"continuum-")
        })
        .count();
    assert_eq!(entries, 5, "P6_EXPECTED_ROWS 必须是五行");
    for subject in [
        "continuum-context",
        "continuum-trust",
        "continuum-memory",
        "continuum-protected",
        "continuum-learning",
    ] {
        assert!(
            block.contains(&format!("\"{subject}\"")),
            "P6_EXPECTED_ROWS 缺少 {subject}"
        );
    }
}

#[test]
fn the_190_sql_is_unchanged() {
    git_diff_is_empty("crates/continuum-memory/src/persist.rs");
}

#[test]
fn the_phase_has_a_single_sql_write_point() {
    let mut hits = Vec::new();
    for (path, source) in source_files() {
        for (line_number, line) in source.lines().enumerate() {
            if line.contains("phase = ?") {
                hits.push((path.display().to_string(), line_number + 1, line.trim().to_owned()));
            }
        }
    }
    assert_eq!(hits.len(), 1, "phase 写点必须唯一: {hits:?}");
    assert!(
        hits[0].2.contains("product_drive"),
        "唯一的 phase 写点不在 product_drive: {:?}",
        hits[0]
    );
}
