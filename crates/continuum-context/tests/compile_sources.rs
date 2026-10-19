//! Task 3：`compile` 的骨架与五来源映射（Memory 之外）。
//!
//! 对齐数组（`provenance[]` / `trust_labels[]`）在每个片段装入时同步 push；
//! 本测试只按读取偏移断言标签，长度／逐位映射的独立复核留给 Task 6 的 `validate`。

use continuum_artifact::ArtifactId;
use continuum_context::{ContextBudget, ContextCandidate, ContextRequest, Reference, compile};
use continuum_core::model::{Message, Role};
use continuum_core::tool::ToolResult;
use continuum_memory::{
    MemoryClaim, MemoryError, MemoryRetrieval, RetrievalQuery,
};
use continuum_trust::{ArtifactProducer, ContextProvenance, ContextTrust};

/// 本 task 不接记忆分支；空检索面即可。
struct NoMemory;

impl MemoryRetrieval for NoMemory {
    fn retrieve(&self, _query: &RetrievalQuery) -> Result<Vec<MemoryClaim>, MemoryError> {
        Ok(Vec::new())
    }
}

fn instruction(text: &str, provenance: ContextProvenance) -> ContextCandidate<String> {
    ContextCandidate {
        content: text.to_string(),
        provenance,
    }
}

fn reference(
    content: Reference,
    provenance: ContextProvenance,
) -> ContextCandidate<Reference> {
    ContextCandidate {
        content,
        provenance,
    }
}

fn artifact(id: &str, provenance: ContextProvenance) -> ContextCandidate<ArtifactId> {
    ContextCandidate {
        content: ArtifactId::new(id),
        provenance,
    }
}

fn budget(max_references: usize, max_artifacts: usize, max_memories: usize) -> ContextBudget {
    ContextBudget {
        max_references,
        max_artifacts,
        max_memories,
    }
}

fn request(
    instructions: Vec<ContextCandidate<String>>,
    references: Vec<ContextCandidate<Reference>>,
    artifacts: Vec<ContextCandidate<ArtifactId>>,
) -> ContextRequest {
    ContextRequest {
        instructions,
        references,
        artifacts,
        scopes: Vec::new(),
        anchor: None,
        memory_query: None,
    }
}

/// 某字段第 `local` 条片段在全局 `provenance[]` / `trust_labels[]` 里的下标。
/// 只读 `references()` 的消费方用 `instructions.len() + j`（设计 §4.2 第 6 条）。
fn global_index(
    package: &continuum_context::ContextPackage,
    field: Field,
    local: usize,
) -> usize {
    match field {
        Field::Instruction => local,
        Field::Reference => package.instructions().len() + local,
        Field::Artifact => {
            package.instructions().len() + package.references().len() + local
        }
    }
}

#[derive(Clone, Copy)]
enum Field {
    Instruction,
    Reference,
    Artifact,
}

fn label_at(package: &continuum_context::ContextPackage, field: Field, local: usize) -> ContextTrust {
    package.trust_labels()[global_index(package, field, local)]
}

#[test]
fn instructions_are_taken_whole_and_empty_strings_are_dropped() {
    let request = request(
        vec![
            instruction("a", ContextProvenance::user_instruction()),
            instruction("", ContextProvenance::user_instruction()),
            instruction("b", ContextProvenance::user_instruction()),
        ],
        Vec::new(),
        Vec::new(),
    );

    let package = compile(request, budget(0, 0, 0), &NoMemory).expect("空指令应被丢弃，不是错误");

    assert_eq!(
        package.instructions(),
        ["a".to_string(), "b".to_string()],
        "非空指令必须全收，空串丢弃"
    );
    assert!(package.references().is_empty());
    assert!(package.artifacts().is_empty());
    assert!(package.memories().is_empty());
    // 两个保留片段各有一条 provenance 与标签（设计 §4.2）。
    assert_eq!(package.provenance().len(), 2);
    assert_eq!(package.trust_labels().len(), 2);
}

#[test]
fn whitespace_only_text_is_not_dropped() {
    let request = request(
        vec![instruction(" ", ContextProvenance::user_instruction())],
        vec![reference(
            Reference::Text(" ".into()),
            ContextProvenance::project_data(),
        )],
        Vec::new(),
    );

    let package = compile(request, budget(1, 0, 0), &NoMemory).expect("纯空白不是错误");

    assert_eq!(package.instructions().len(), 1, "纯空白指令不是空串");
    assert_eq!(package.references().len(), 1, "纯空白引用不是空串");
}

#[test]
fn conversation_history_lands_in_references_with_user_instruction_label() {
    let message = Message {
        role: Role::User,
        content: "历史用户轮".into(),
    };
    let request = request(
        Vec::new(),
        vec![reference(
            Reference::Conversation(message.clone()),
            ContextProvenance::conversation_history(),
        )],
        Vec::new(),
    );

    let package = compile(request, budget(1, 0, 0), &NoMemory).expect("历史轮是合法引用");

    assert!(package.instructions().is_empty(), "历史轮不得进指令位");
    assert_eq!(package.references(), [Reference::Conversation(message)]);
    assert_eq!(
        label_at(&package, Field::Reference, 0),
        ContextTrust::UserInstruction
    );
}

#[test]
fn project_text_lands_in_references_with_project_data_label() {
    let request = request(
        Vec::new(),
        vec![reference(
            Reference::Text("project".into()),
            ContextProvenance::project_data(),
        )],
        Vec::new(),
    );

    let package = compile(request, budget(1, 0, 0), &NoMemory).expect("项目文本是合法引用");

    assert_eq!(package.references(), [Reference::Text("project".into())]);
    assert!(package.instructions().is_empty(), "项目文本不得进指令位");
    assert!(package.artifacts().is_empty(), "项目文本不得进 artifact 位");
    assert!(package.memories().is_empty(), "项目文本不得进记忆位");
    assert_eq!(
        label_at(&package, Field::Reference, 0),
        ContextTrust::ProjectData
    );
}

#[test]
fn tool_output_lands_in_references_with_tool_output_label() {
    let result = ToolResult {
        output: serde_json::json!({"ok": true}),
        is_error: false,
    };
    let request = request(
        Vec::new(),
        vec![reference(
            Reference::ToolOutput(result.clone()),
            ContextProvenance::tool_output(false),
        )],
        Vec::new(),
    );

    let package = compile(request, budget(1, 0, 0), &NoMemory).expect("工具结果是合法引用");

    assert_eq!(package.references(), [Reference::ToolOutput(result)]);
    assert!(package.instructions().is_empty(), "工具结果不得进指令位");
    assert!(package.artifacts().is_empty(), "工具结果不得进 artifact 位");
    assert!(package.memories().is_empty(), "工具结果不得进记忆位");
    assert_eq!(
        label_at(&package, Field::Reference, 0),
        ContextTrust::ToolOutput
    );
}

#[test]
fn external_provider_text_lands_in_references_with_web_untrusted_label() {
    let request = request(
        Vec::new(),
        vec![reference(
            Reference::Text("provider".into()),
            ContextProvenance::provider_supplied(),
        )],
        Vec::new(),
    );

    let package = compile(request, budget(1, 0, 0), &NoMemory).expect("provider 文本是合法引用");

    let label = label_at(&package, Field::Reference, 0);
    assert_eq!(label, ContextTrust::WebUntrusted);
    assert_ne!(label, ContextTrust::TrustedRuntime);
    assert!(package.instructions().is_empty(), "provider 文本不得进指令位");
    assert!(package.artifacts().is_empty(), "provider 文本不得进 artifact 位");
    assert!(package.memories().is_empty(), "provider 文本不得进记忆位");
}

#[test]
fn artifact_ids_land_in_artifacts_with_the_artifact_label() {
    let request = request(
        Vec::new(),
        Vec::new(),
        vec![artifact(
            "artifact-1",
            ContextProvenance::artifact(ArtifactProducer::Project),
        )],
    );

    let package = compile(request, budget(0, 1, 0), &NoMemory).expect("artifact 是合法来源");

    assert_eq!(package.artifacts(), [ArtifactId::new("artifact-1")]);
    assert!(package.instructions().is_empty(), "artifact 不得进指令位");
    assert!(package.references().is_empty(), "artifact 不得进引用位");
    assert!(package.memories().is_empty(), "artifact 不得进记忆位");
    assert_eq!(
        label_at(&package, Field::Artifact, 0),
        ContextTrust::ProjectData
    );
}

#[test]
fn references_never_leak_into_instructions() {
    // 前半（J10）：有引用无指令 ⇒ instructions[] 空、references[] 非空。
    let request_with_reference_only = request(
        Vec::new(),
        vec![reference(
            Reference::Text("外部内容".into()),
            ContextProvenance::web_untrusted(),
        )],
        Vec::new(),
    );
    let package = compile(request_with_reference_only, budget(1, 0, 0), &NoMemory)
        .expect("只有引用是合法的");
    assert!(package.instructions().is_empty(), "引用不得渗入指令位");
    assert_eq!(package.references().len(), 1);
    assert_eq!(package.references()[0], Reference::Text("外部内容".into()));

    // 后半：同一段文本由调用方自己放进 instructions，compile 不拦（设计 §11 第 20 条）。
    let request_with_instruction = request(
        vec![instruction("外部内容", ContextProvenance::web_untrusted())],
        Vec::new(),
        Vec::new(),
    );
    let package = compile(request_with_instruction, budget(0, 0, 0), &NoMemory)
        .expect("本块不判定调用方把外部内容放进指令位");
    assert_eq!(package.instructions(), ["外部内容".to_string()]);
    assert_eq!(
        label_at(&package, Field::Instruction, 0),
        ContextTrust::WebUntrusted
    );
}

#[test]
fn an_empty_request_yields_six_empty_arrays() {
    let package = compile(request(Vec::new(), Vec::new(), Vec::new()), budget(0, 0, 0), &NoMemory)
        .expect("空包合法");

    assert!(package.instructions().is_empty());
    assert!(package.references().is_empty());
    assert!(package.artifacts().is_empty());
    assert!(package.memories().is_empty());
    assert!(package.provenance().is_empty());
    assert!(package.trust_labels().is_empty());
}

#[test]
fn the_alignment_arrays_are_non_empty_and_equal_length() {
    let request = request(
        vec![instruction("指令", ContextProvenance::user_instruction())],
        vec![reference(
            Reference::Text("引用".into()),
            ContextProvenance::project_data(),
        )],
        vec![artifact(
            "artifact-1",
            ContextProvenance::artifact(ArtifactProducer::Tool),
        )],
    );

    let package = compile(request, budget(1, 1, 0), &NoMemory).expect("三类来源都应装配");

    assert!(!package.provenance().is_empty());
    assert!(!package.trust_labels().is_empty());
    assert_eq!(
        package.provenance().len(),
        package.trust_labels().len(),
        "对齐数组必须等长"
    );
}
