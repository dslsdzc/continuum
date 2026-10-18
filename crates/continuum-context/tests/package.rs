//! J2 的源码面（六字段名／序／私有、恰六个访问器）、J1-d 的「禁止构造入口」一半，
//! 以及 Review Focus 5 的一半：本 crate 不提供把包压平成 `Message` 的投影 API。

use continuum_artifact::ArtifactId;
use continuum_context::{ContextPackage, Reference};
use continuum_core::model::{Message, Role};
use continuum_core::tool::ToolResult;
use continuum_memory::MemoryClaim;
use continuum_trust::{ContextProvenance, ContextTrust};

/// 源码面语料。下游断言一律遍历这个 helper，不在本地另写文件清单。
/// Task 2 扩到五个文件，Task 3 扩到六个。
fn src_texts() -> Vec<(&'static str, &'static str)> {
    vec![
        ("src/lib.rs", include_str!("../src/lib.rs")),
        ("src/package.rs", include_str!("../src/package.rs")),
        ("src/reference.rs", include_str!("../src/reference.rs")),
        ("src/request.rs", include_str!("../src/request.rs")),
        ("src/candidate.rs", include_str!("../src/candidate.rs")),
    ]
}

/// 从含开括号的片段起点取到配对的闭括号之前，返回块体。
/// 访问器的函数体自带花括号，不能用「第一个 `}`」截断。
fn balanced_body(text: &str) -> String {
    let mut depth = 0usize;
    let mut body = String::new();
    for ch in text.chars() {
        match ch {
            '{' => {
                depth += 1;
                if depth == 1 {
                    continue;
                }
            }
            '}' => {
                depth = depth.checked_sub(1).expect("闭括号多于开括号");
                if depth == 0 {
                    break;
                }
            }
            _ => {}
        }
        if depth >= 1 {
            body.push(ch);
        }
    }
    body
}

#[test]
fn the_six_fields_are_verbatim_in_declaration_order() {
    let src = include_str!("../src/lib.rs");
    let start = src
        .find("pub struct ContextPackage {")
        .expect("src/lib.rs 里找不到 ContextPackage 的定义");
    let rest = &src[start..];
    let body_start = rest.find('{').expect("定义缺左花括号") + 1;
    let body_end = rest[body_start..]
        .find('}')
        .expect("定义缺右花括号")
        + body_start;
    let fields: Vec<&str> = rest[body_start..body_end]
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| {
            line.split(':')
                .next()
                .expect("字段行缺冒号")
                .trim()
        })
        .collect();

    // 期望值手写，不从源码解析结果反向生成。
    assert_eq!(
        fields,
        [
            "instructions",
            "references",
            "artifacts",
            "memories",
            "provenance",
            "trust_labels",
        ],
        "六个字段必须逐字、按 §280 的声明序；实际取到 {fields:?}"
    );
}

fn assert_accessors(package: &ContextPackage) {
    let _: &[String] = package.instructions();
    let _: &[Reference] = package.references();
    let _: &[ArtifactId] = package.artifacts();
    let _: &[MemoryClaim] = package.memories();
    let _: &[ContextProvenance] = package.provenance();
    let _: &[ContextTrust] = package.trust_labels();
}

#[test]
fn the_package_has_only_read_only_accessors() {
    // 编译期证明六个访问器存在且只读；显式引用而不调用，避免 dead_code 警告。
    let _: fn(&ContextPackage) = assert_accessors;

    for (name, text) in src_texts() {
        for forbidden in [
            "&mut self",
            "pub fn instructions_mut",
            "pub fn into_parts",
            "pub fn into_iter",
        ] {
            assert!(
                !text.contains(forbidden),
                "{name} 不应出现可写／拆包入口：{forbidden:?}"
            );
        }
    }

    // `ContextPackage` 的六个字段必须私有。`ContextRequest` / `ContextCandidate`
    // 的 `pub` 字段是设计 §5.2 的输入形状，不在这条断言的射程内——故只扫定义与
    // 访问器所在的两个文件。
    for (name, text) in src_texts() {
        if name != "src/lib.rs" && name != "src/package.rs" {
            continue;
        }
        for forbidden in [
            "pub instructions:",
            "pub references:",
            "pub artifacts:",
            "pub memories:",
            "pub provenance:",
            "pub trust_labels:",
        ] {
            assert!(
                !text.contains(forbidden),
                "{name} 不应出现 ContextPackage 的公开字段：{forbidden:?}"
            );
        }
    }

    // `impl ContextPackage {` 只允许出现在 src/package.rs 一处。
    for (name, text) in src_texts() {
        let expected = if name == "src/package.rs" { 1 } else { 0 };
        assert_eq!(
            text.matches("impl ContextPackage {").count(),
            expected,
            "{name} 的 impl ContextPackage 块数不对"
        );
    }

    // 那一处 impl 块内恰六条 `pub fn`，名字与 §280 的字段序一致。
    let package_src = include_str!("../src/package.rs");
    let start = package_src
        .find("impl ContextPackage {")
        .expect("src/package.rs 缺 impl ContextPackage 块");
    let rest = &package_src[start..];
    let body = balanced_body(rest);
    let names: Vec<&str> = body
        .lines()
        .map(str::trim)
        .filter_map(|line| line.strip_prefix("pub fn "))
        .map(|line| {
            line.split('(')
                .next()
                .expect("访问器行缺参数表")
                .trim()
        })
        .collect();
    assert_eq!(
        names,
        [
            "instructions",
            "references",
            "artifacts",
            "memories",
            "provenance",
            "trust_labels",
        ],
        "访问器必须恰六条、按字段序；实际取到 {names:?}"
    );

    // 三个禁止构造入口：源码面零命中（注释里也不写这三个名字）。
    for (name, text) in src_texts() {
        for forbidden in ["pub fn new", "from_parts", "from_fields"] {
            assert!(
                !text.contains(forbidden),
                "{name} 不应出现禁止构造入口：{forbidden:?}"
            );
        }
    }
}

#[test]
fn there_is_no_projection_to_messages_or_invoke_request() {
    for (name, text) in src_texts() {
        for forbidden in [
            "to_messages",
            "into_invoke_request",
            "InvokeRequest",
            "impl Serialize",
            "impl Deserialize",
            "Serialize for ContextPackage",
            "Deserialize for ContextPackage",
        ] {
            assert!(
                !text.contains(forbidden),
                "{name} 不应出现投影／序列化面：{forbidden:?}"
            );
        }
    }
}

#[test]
fn reference_keeps_message_and_tool_result_verbatim() {
    let message = Message {
        role: Role::User,
        content: "历史用户轮".into(),
    };
    let conversation = Reference::Conversation(message.clone());
    assert_eq!(
        conversation.clone(),
        Reference::Conversation(message),
        "Reference::Conversation 必须原样保留 role 与 content"
    );

    let tool_result = ToolResult {
        output: serde_json::json!({"k": "v"}),
        is_error: true,
    };
    let tool_output = Reference::ToolOutput(tool_result.clone());
    assert_eq!(
        tool_output.clone(),
        Reference::ToolOutput(tool_result),
        "Reference::ToolOutput 必须原样保留 output 与 is_error"
    );

    assert_ne!(
        conversation,
        tool_output,
        "PartialEq 必须区分不同变体"
    );
    assert_ne!(
        Reference::Text("x".into()),
        Reference::Conversation(Message {
            role: Role::User,
            content: "x".into(),
        }),
        "Text 与 Conversation 不得相等"
    );
}
