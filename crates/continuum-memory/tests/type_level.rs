//! 「七字段之外没有第八个可读字段 / claim 构造点 crate 私有 / 没有 Session scope /
//! 提案表达不了终态与时间 / Source 字段私有 / Confidence 没有序」这七条**不可表达性**
//! 命题的编译期照片（设计 §10.2 的 C1–C7；C4 拆成 C4a／C4b 两份）。
//!
//! 本目录现有**七份**样例，下面点名（这段点名的项数必须等于目录里 `.rs` 的份数）：
//!
//! 1. **`claim_has_no_status_or_anchor_accessor`**（C1）——`claim.status()`／`claim.anchor()`
//!    编不过，钉 §5.1 的七字段之外没有第八个可读字段。
//! 2. **`claim_constructor_is_crate_private`**（C2）——crate 外调 `MemoryClaim::new` 编不过，
//!    钉 §5.5 的唯一产生点；`test-support` 下的 `for_test` 是另一个入口，不改变本口径。
//! 3. **`memory_scope_has_no_session`**（C3）——`MemoryScope::Session` 编不过，钉 §5.2。
//! 4. **`proposed_claim_cannot_express_validity`**（C4a）——提案不能写 `validity`。
//! 5. **`proposed_claim_cannot_express_last_confirmed`**（C4b）——提案不能写时间。
//! 6. **`memory_source_fields_are_private`**（C5）——全字段字面量编不过，钉 §4.2。
//! 7. **`confidence_has_no_order`**（C6）——两枚 `Confidence` 不可比较，钉 §5.3.2。
//!
//! 第 8 条是**正控制** [`c7_the_five_scopes_and_nine_kinds_compile`]：五 scope、九 kind、
//! 三 validity、一个空检索 query 都能编过——防止 C1–C6 是「整块编不过」的假照片。
//!
//! `.stderr` 一旦接受，就不再改对应 `.rs` 的注释（trybuild 不规范化行号）。

/// 清单里点到的样例名（`tests/compile_fail/` 下 `.rs` 的文件名主干）——**现有七份**。
const LISTED_SAMPLES: [&str; 7] = [
    "claim_constructor_is_crate_private",
    "claim_has_no_status_or_anchor_accessor",
    "confidence_has_no_order",
    "memory_scope_has_no_session",
    "memory_source_fields_are_private",
    "proposed_claim_cannot_express_last_confirmed",
    "proposed_claim_cannot_express_validity",
];

#[test]
fn type_level_guarantees_hold() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
}

/// 清单 ↔ 目录 ↔ 文件头点名三处对钉。
#[test]
fn the_sample_list_matches_the_directory() {
    let mut in_dir: Vec<String> = std::fs::read_dir("tests/compile_fail")
        .expect("tests/compile_fail/ 应可读（用例工作目录是 crate 根）")
        .map(|entry| {
            entry
                .expect("目录项应可读")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter(|name| name.ends_with(".rs"))
        .map(|name| name.trim_end_matches(".rs").to_string())
        .collect();
    let mut listed: Vec<String> = LISTED_SAMPLES
        .iter()
        .map(|name| (*name).to_string())
        .collect();
    in_dir.sort();
    listed.sort();
    assert_eq!(
        listed, in_dir,
        "清单与 tests/compile_fail/*.rs 的名字集合必须相等：加了样例，就往 LISTED_SAMPLES 与文件头点名各加一行"
    );

    let source = include_str!("type_level.rs");
    let header = source
        .split("const LISTED_SAMPLES")
        .next()
        .expect("本文件里应有 LISTED_SAMPLES 的定义");
    for name in LISTED_SAMPLES {
        assert!(
            header.contains(name),
            "文件头那段点名里没有 `{name}`：加样例时那两处都要同步"
        );
    }
}

/// C7 正控制：这些名字与构造都能编过；否则 C1–C6 的失败可能只是整块编不过。
#[test]
fn c7_the_five_scopes_and_nine_kinds_compile() {
    use continuum_memory::{
        Confidence, MemoryScope, MemorySource, MemorySourceId, RetrievalQuery, SourceKind,
        Validity,
    };

    for scope in MemoryScope::ALL {
        let _ = scope.as_str();
    }
    for kind in SourceKind::ALL {
        let source = MemorySource::new(MemorySourceId::new("s"), kind, "r", 0).unwrap();
        assert_eq!(source.kind(), kind);
    }
    for validity in Validity::ALL {
        let _ = validity.as_str();
    }
    let query = RetrievalQuery::new(vec![MemoryScope::Personal], None, None, 0, false).unwrap();
    assert_eq!(query.limit(), 0);
    let _ = Confidence::from_value(serde_json::json!(null));
}
