//! §9.3 (t)(u)(v)(w) 的源码面复核：把 Task 6 Step 2 的四条 grep 落成可重复的测试。
//!
//! 这里拍的不是类型级保证，只是本轮语料的补件（设计 §4.1 第 3 条、§9.3 末）。
//! 只扫 `crates/*/src` 下的 `.rs`；`tests/` 里的手写期望名单不在语料里。
//!
//! 计数口径按计划 Task 6 Step 2 的读法：
//! - (t) 七个引号串共 **十四条**（编码表七臂 + 解码表七臂），全部在 `trust.rs`；
//! - (u) `enum .*Trust` / `struct .*Trust` 共 **四处**（含 `TrustError` 的前缀命中）；
//! - (v) `continuum-trust` 的直接依赖只有四个外部 crate，没有 `continuum-*` / `path`；
//! - (w) 在 `crates/continuum-trust/src` 内，`label(&` 的**可执行**命中只有 `validate.rs`
//!   一处，定义只有 `label.rs` 一处（`validate.rs` 的文档注释里还有一处同形文本，不算调用）。
//!   扫描范围收窄到本 crate：P6A 的 `continuum-context/src/compile.rs` 按设计 §4.2 第 5 条／
//!   §5.3 第 6 条每片段调一次 `label()`，那四个生产调用点不归本 crate 的语料管。

use std::fs;
use std::path::{Path, PathBuf};

/// 从 `CARGO_MANIFEST_DIR` 回溯到 workspace 根（含 `crates/continuum-trust` 的那一层）。
fn workspace_root() -> PathBuf {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = crate_dir
        .parent()
        .and_then(Path::parent)
        .expect("crate 目录必须位于 workspace/crates/ 下");
    assert!(
        root.join("crates/continuum-trust").is_dir(),
        "回溯出的 workspace 根必须含 crates/continuum-trust：{}",
        root.display()
    );
    root.to_path_buf()
}

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("读目录失败 {}: {e}", dir.display()))
        .map(|entry| entry.expect("目录项可读").path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            collect_rs(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

/// `crates/*/src` 下的全部 Rust 源文件，按路径排序以保证输出确定。
fn rust_sources() -> Vec<PathBuf> {
    let root = workspace_root();
    let crates_dir = root.join("crates");
    let mut crate_dirs: Vec<PathBuf> = fs::read_dir(&crates_dir)
        .unwrap_or_else(|e| panic!("读 crates/ 失败 {}: {e}", crates_dir.display()))
        .map(|entry| entry.expect("目录项可读").path())
        .filter(|path| path.is_dir())
        .collect();
    crate_dirs.sort();

    let mut sources = Vec::new();
    for crate_dir in crate_dirs {
        let src = crate_dir.join("src");
        if src.is_dir() {
            collect_rs(&src, &mut sources);
        }
    }
    assert!(!sources.is_empty(), "crates/*/src 下必须至少有一个 .rs");
    sources
}

/// 本 crate 的 `src` 语料。(w) 只在这里数：P6A 的 `compile.rs` 另有四个生产调用点。
fn trust_sources() -> Vec<PathBuf> {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut sources = Vec::new();
    collect_rs(&src, &mut sources);
    assert!(
        !sources.is_empty(),
        "continuum-trust/src 下必须至少有一个 .rs"
    );
    sources
}

fn relative(path: &Path) -> String {
    path.strip_prefix(workspace_root())
        .expect("源文件必须位于 workspace 内")
        .to_string_lossy()
        .replace('\\', "/")
}

fn source_text(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("读源文件失败 {}: {e}", path.display()))
}

fn is_comment(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("//")
}

/// (t)：七个引号串只在 `trust.rs` 的编解码对里，合计十四条。
#[test]
fn the_seven_encodings_live_only_in_the_encoder_pair() {
    const QUOTED: [&str; 7] = [
        "\"USER_INSTRUCTION\"",
        "\"TRUSTED_RUNTIME\"",
        "\"PROJECT_DATA\"",
        "\"WEB_UNTRUSTED\"",
        "\"TOOL_OUTPUT\"",
        "\"MEMORY\"",
        "\"MODEL_OUTPUT\"",
    ];

    let mut total = 0;
    for path in rust_sources() {
        let text = source_text(&path);
        for needle in QUOTED {
            let count = text.matches(needle).count();
            if count == 0 {
                continue;
            }
            assert_eq!(
                relative(&path),
                "crates/continuum-trust/src/trust.rs",
                "{needle} 只许出现在 `as_str`/`parse` 的同一文件里"
            );
            assert_eq!(
                count, 2,
                "{needle} 必须恰有两条（编码一臂、解码一臂），实际 {count}"
            );
            total += count;
        }
    }
    assert_eq!(
        total, 14,
        "七个引号串合计必须是十四条（编码七 + 解码七），实际 {total}"
    );
}

/// (u)：源码面只有四个「名字里带 Trust」的类型声明，且窄口径的枚举恰为两枚。
#[test]
fn trust_named_type_declarations_are_exactly_the_expected_four() {
    let mut declarations: Vec<(String, String, String)> = Vec::new();
    for path in rust_sources() {
        for (line_no, line) in source_text(&path).lines().enumerate() {
            let Some((kind, rest)) = ["enum ", "struct "]
                .iter()
                .find_map(|keyword| line.find(keyword).map(|at| (*keyword, &line[at + keyword.len()..])))
            else {
                continue;
            };
            if !rest.contains("Trust") {
                continue;
            }
            let name: String = rest
                .chars()
                .take_while(|ch| ch.is_alphanumeric() || *ch == '_')
                .collect();
            declarations.push((
                relative(&path),
                format!("{}:{}", kind.trim(), name),
                format!("{}:{}", relative(&path), line_no + 1),
            ));
        }
    }

    let mut found: Vec<&str> = declarations.iter().map(|(_, id, _)| id.as_str()).collect();
    found.sort_unstable();
    assert_eq!(
        found,
        ["enum:ContextTrust", "enum:NodeTrust", "enum:TrustError", "struct:Trust"],
        "名字里带 Trust 的类型声明必须恰为这四处；逐处读命中行：ContextTrust 是标签类型，\
         TrustError 是错误类型，NodeTrust / Trust 是既有同名异物"
    );

    let mut enums_ending_with_trust: Vec<&str> = declarations
        .iter()
        .filter(|(_, id, _)| id.starts_with("enum:") && id.ends_with("Trust"))
        .map(|(_, id, _)| id.trim_start_matches("enum:"))
        .collect();
    enums_ending_with_trust.sort_unstable();
    assert_eq!(
        enums_ending_with_trust,
        ["ContextTrust", "NodeTrust"],
        "窄口径 `enum [A-Za-z]*Trust\\b` 只应命中 ContextTrust 与 NodeTrust"
    );
}

/// (v)：`Cargo.toml` 的直接依赖只有四个外部 crate，且没有 workspace 内依赖或 path 依赖。
#[test]
fn the_manifest_declares_only_external_dependencies() {
    let manifest_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let manifest = source_text(&manifest_path);

    fn section_lines<'a>(manifest: &'a str, section: &str) -> Vec<&'a str> {
        let header = format!("[{section}]");
        let mut inside = false;
        let mut lines = Vec::new();
        for line in manifest.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('[') {
                inside = trimmed == header;
                continue;
            }
            if inside && !trimmed.is_empty() && !trimmed.starts_with('#') {
                lines.push(trimmed);
            }
        }
        lines
    }

    let dependencies = section_lines(&manifest, "dependencies");
    let dev_dependencies = section_lines(&manifest, "dev-dependencies");

    let names = |lines: &[&str]| -> Vec<String> {
        lines
            .iter()
            .map(|line| {
                line.split_once('=')
                    .unwrap_or_else(|| panic!("依赖行必须形如 key = value：{line}"))
                    .0
                    .trim()
                    .to_string()
            })
            .collect()
    };

    assert_eq!(
        names(&dependencies),
        ["serde", "thiserror"],
        "正式依赖只许 serde / thiserror"
    );
    assert_eq!(
        names(&dev_dependencies),
        ["serde_json", "trybuild"],
        "开发依赖只许 serde_json / trybuild"
    );

    for line in dependencies.iter().chain(dev_dependencies.iter()) {
        assert!(
            !line.contains("continuum-"),
            "本 crate 对 workspace 内零依赖，依赖行不许出现 continuum-*：{line}"
        );
        assert!(
            !line.contains("path ="),
            "依赖行不许是 path 依赖：{line}"
        );
    }
}

/// (w)：本 crate 内 `label(&` 的可执行命中只有 `validate.rs` 一处；定义只有 `label.rs` 一处。
#[test]
fn label_is_defined_once_and_called_only_by_validate_within_trust() {
    let mut calls: Vec<(String, usize, String)> = Vec::new();
    let mut definitions: Vec<(String, usize)> = Vec::new();

    for path in trust_sources() {
        for (line_no, line) in source_text(&path).lines().enumerate() {
            if line.contains("pub fn label") {
                definitions.push((relative(&path), line_no + 1));
            }
            if line.contains("label(&") && !is_comment(line) {
                calls.push((relative(&path), line_no + 1, line.trim().to_string()));
            }
        }
    }

    assert_eq!(
        definitions,
        [("crates/continuum-trust/src/label.rs".to_string(), 12)],
        "`pub fn label` 必须只在 label.rs 定义一次"
    );
    assert_eq!(
        calls.len(),
        1,
        "`label(&` 的可执行调用只许一处（文档注释里的同形文本不计），实际：{calls:?}"
    );
    let (path, _, line) = &calls[0];
    assert_eq!(path, "crates/continuum-trust/src/validate.rs");
    assert!(
        line.contains("label(&provenance[i])"),
        "唯一调用点必须是 validate 的 I3 逐位比对：{line}"
    );
}
