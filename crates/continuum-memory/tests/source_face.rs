//! 源码面照片（设计 §10.3 的 S3／S4／S6／S8／S11 与 §13.3 词汇表）。
//!
//! 这些用例读的是本 crate 的源码文本，不是运行期行为。

use std::path::PathBuf;

/// 本块要求零事件、零审计的九个源文件。
const SRC_FILES: [&str; 9] = [
    "lib.rs",
    "error.rs",
    "persist.rs",
    "source.rs",
    "claim.rs",
    "summary.rs",
    "correction.rs",
    "synthesis.rs",
    "retrieval.rs",
];

fn crate_file(relative: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("读不到 {}: {e}", path.display()))
}

fn source(relative: &str) -> String {
    crate_file(&format!("src/{relative}"))
}

/// 逐行去掉 `//` 起的注释后的源码语料（文档注释以 `//` 开头，一并去掉）。
fn code_corpus() -> String {
    let mut corpus = String::new();
    for file in SRC_FILES {
        let text = source(file);
        for line in text.lines() {
            corpus.push_str(line.split("//").next().unwrap_or(""));
            corpus.push('\n');
        }
    }
    corpus
}

fn tokens(corpus: &str) -> Vec<String> {
    corpus
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .filter(|token| !token.is_empty())
        .map(str::to_string)
        .collect()
}

fn struct_fields(file: &str, header: &str) -> Vec<String> {
    let text = source(file);
    let start = text
        .find(header)
        .unwrap_or_else(|| panic!("{file} 里找不到 `{header}`"));
    let after_header = &text[start + header.len()..];
    let body = after_header
        .split_once('\n')
        .expect("结构体头之后应有换行")
        .1;
    let mut fields = Vec::new();
    for line in body.lines() {
        if line == "}" {
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("#[") || trimmed.starts_with("//") {
            continue;
        }
        let Some((name, _)) = trimmed.split_once(':') else {
            continue;
        };
        let name = name.trim();
        if !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        {
            fields.push(name.to_string());
        }
    }
    fields.sort();
    fields
}

#[test]
fn source_module_does_not_reference_claims() {
    let text = source("source.rs");
    for forbidden in ["crate::claim", "MemoryClaim", "Validity"] {
        assert!(
            !text.contains(forbidden),
            "src/source.rs 不得出现 `{forbidden}`（S3：source 模块不引用 claim 侧类型）"
        );
    }
}

#[test]
fn no_event_or_audit_symbols() {
    for file in SRC_FILES {
        let text = source(file);
        for forbidden in ["EventType", "AuditKind", "append_event", "append_audit"] {
            assert!(
                !text.contains(forbidden),
                "src/{file} 出现 `{forbidden}`：本块零事件、零审计（S4）"
            );
        }
    }
}

#[test]
fn enum_arity_literals_are_pinned() {
    assert_eq!(
        source("claim.rs").matches("ALL: [MemoryScope; 5]").count(),
        1,
        "`ALL: [MemoryScope; 5]` 必须恰一次"
    );
    assert_eq!(
        source("source.rs").matches("ALL: [SourceKind; 9]").count(),
        1,
        "`ALL: [SourceKind; 9]` 必须恰一次"
    );
    assert_eq!(continuum_memory::MemoryScope::ALL.len(), 5);
    assert_eq!(continuum_memory::SourceKind::ALL.len(), 9);
}

#[test]
fn validity_encodes_superseded_verbatim() {
    use continuum_memory::Validity;
    assert_eq!(Validity::Superseded.as_str(), "superseded");
    for validity in Validity::ALL {
        assert_eq!(Validity::parse(validity.as_str()), Some(validity));
    }
}

#[test]
fn cross_cutting_vocabulary_is_not_reused() {
    let corpus = code_corpus();
    let tokens = tokens(&corpus);

    for token in &tokens {
        let lower = token.to_ascii_lowercase();
        assert!(
            !lower.contains("provenance"),
            "源码标识符 `{token}` 使用了 `provenance`：§13.3 不用它指来源"
        );
        let trust_exempt = token == "distrust" || token == "Distrust";
        assert!(
            !lower.contains("trust") || trust_exempt,
            "源码标识符 `{token}` 使用了 `trust`：§13.3 不用它指来源种类（整词 distrust 除外）"
        );
        assert!(
            token != "Scope",
            "源码里出现裸 `Scope`：本块只用 `MemoryScope`（§13.3）"
        );
    }
}

#[test]
fn claim_and_source_field_lists_are_exact() {
    let claim_fields = struct_fields("claim.rs", "struct MemoryClaim {");
    assert_eq!(
        claim_fields.len(),
        7,
        "`MemoryClaim` 必须恰有七个字段（S11／S7）"
    );
    assert_eq!(
        claim_fields,
        [
            "confidence",
            "id",
            "last_confirmed",
            "scope",
            "sources",
            "statement",
            "validity"
        ]
    );

    let source_fields = struct_fields("source.rs", "struct MemorySource {");
    assert_eq!(
        source_fields.len(),
        5,
        "`MemorySource` 必须恰有五个字段（S11）"
    );
    assert_eq!(
        source_fields,
        ["external_ref", "id", "kind", "observed_at", "removed_at"]
    );
}
