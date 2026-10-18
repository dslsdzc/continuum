//! Task 6：标签对齐与 P6B `validate` 的独立复核（J3 的落点）。
//!
//! 长度、逐位映射与类别全部交给 `continuum_trust::validate`；本文件不自写第二份
//! 长度／逐位断言。`compile` 的生产路径不调用 `validate`。

mod support;

use continuum_artifact::ArtifactId;
use continuum_context::{ContextBudget, ContextCandidate, ContextRequest, Reference, compile};
use continuum_core::model::{Message, Role};
use continuum_memory::MemoryScope;
use continuum_trust::{
    ArtifactProducer, ContextProvenance, ContextTrust, PackageShape, TrustError, validate,
};
use support::{CountingMemory, test_claim};

fn instruction(text: &str, provenance: ContextProvenance) -> ContextCandidate<String> {
    ContextCandidate {
        content: text.to_string(),
        provenance,
    }
}

fn reference(
    text: &str,
    provenance: ContextProvenance,
) -> ContextCandidate<Reference> {
    ContextCandidate {
        content: Reference::Text(text.to_string()),
        provenance,
    }
}

fn artifact(id: &str) -> ContextCandidate<ArtifactId> {
    ContextCandidate {
        content: ArtifactId::new(id),
        provenance: ContextProvenance::artifact(ArtifactProducer::Project),
    }
}

fn budget(max_references: usize, max_artifacts: usize, max_memories: usize) -> ContextBudget {
    ContextBudget {
        max_references,
        max_artifacts,
        max_memories,
    }
}

#[test]
fn the_package_passes_the_independent_validate() {
    let memory = CountingMemory::new(vec![test_claim("c1", "记忆")]);
    let request = ContextRequest {
        instructions: vec![instruction("当前指令", ContextProvenance::user_instruction())],
        references: vec![
            ContextCandidate {
                content: Reference::Conversation(Message {
                    role: Role::User,
                    content: "历史轮".into(),
                }),
                provenance: ContextProvenance::conversation_history(),
            },
            reference("外部文本", ContextProvenance::web_untrusted()),
        ],
        artifacts: vec![artifact("artifact-1")],
        scopes: vec![MemoryScope::Personal],
        anchor: None,
        memory_query: None,
    };

    let package = compile(request, budget(2, 1, 1), &memory).expect("五类来源都应装配");

    validate(
        PackageShape {
            instructions: 1,
            references: 2,
            artifacts: 1,
            memories: 1,
        },
        package.provenance(),
        package.trust_labels(),
    )
    .expect("compile 产出的包必须通过 P6B 的独立复核");
}

#[test]
fn relabeling_a_reference_changes_its_label_in_place() {
    // 同一条 Reference::Text，只换 provenance；指令位保留一条，读取偏移必须是
    // instructions.len() + j（设计 §4.2 第 6 条）。
    let compile_with = |provenance: ContextProvenance| {
        let memory = CountingMemory::new(Vec::new());
        let request = ContextRequest {
            instructions: vec![instruction("当前指令", ContextProvenance::user_instruction())],
            references: vec![reference("同一条引用", provenance)],
            artifacts: Vec::new(),
            scopes: Vec::new(),
            anchor: None,
            memory_query: None,
        };
        compile(request, budget(1, 0, 0), &memory).expect("编译成功")
    };

    let web = compile_with(ContextProvenance::web_untrusted());
    let project = compile_with(ContextProvenance::project_data());

    let offset = web.instructions().len();
    assert_eq!(
        web.trust_labels()[offset],
        ContextTrust::WebUntrusted,
        "引用标签的读取偏移是 instructions.len() + j"
    );
    assert_eq!(
        project.trust_labels()[offset],
        ContextTrust::ProjectData,
        "换 provenance 后标签应在同一位跟着变"
    );
}

#[test]
fn validate_rejects_a_label_that_does_not_match_its_provenance() {
    let result = validate(
        PackageShape {
            instructions: 0,
            references: 1,
            artifacts: 0,
            memories: 0,
        },
        &[ContextProvenance::web_untrusted()],
        &[ContextTrust::ProjectData],
    );

    assert!(
        matches!(
            result,
            Err(TrustError::LabelDoesNotMatchProvenance { .. })
        ),
        "标签与 provenance 不符必须命中 LabelDoesNotMatchProvenance，实际 {result:?}"
    );
}

#[test]
fn validate_rejects_a_current_instruction_in_a_data_slot() {
    let result = validate(
        PackageShape {
            instructions: 0,
            references: 1,
            artifacts: 0,
            memories: 0,
        },
        &[ContextProvenance::UserInstruction { historical: false }],
        &[ContextTrust::UserInstruction],
    );

    assert!(
        matches!(
            result,
            Err(TrustError::DataSlotCarriesUserInstruction { .. })
        ),
        "当前指令进 references 必须被拒，实际 {result:?}"
    );
}

#[test]
fn validate_rejects_a_historical_instruction_in_the_instruction_slot() {
    let result = validate(
        PackageShape {
            instructions: 1,
            references: 0,
            artifacts: 0,
            memories: 0,
        },
        &[ContextProvenance::UserInstruction { historical: true }],
        &[ContextTrust::UserInstruction],
    );

    assert!(
        matches!(
            result,
            Err(TrustError::HistoricalInstructionInInstructions { .. })
        ),
        "历史轮进指令位必须被拒，实际 {result:?}"
    );
}

#[test]
fn compile_does_not_judge_historical_but_validate_does() {
    let memory = CountingMemory::new(Vec::new());
    let request = ContextRequest {
        instructions: vec![instruction(
            "历史用户轮",
            ContextProvenance::conversation_history(),
        )],
        references: Vec::new(),
        artifacts: Vec::new(),
        scopes: Vec::new(),
        anchor: None,
        memory_query: None,
    };

    // 生产路径不判 historical：compile 静默接受调用方放错的位置。
    let package = compile(request, budget(0, 0, 0), &memory)
        .expect("compile 不判 historical");
    assert_eq!(package.instructions(), ["历史用户轮".to_string()]);

    // 复核面判：独立 validate 拒绝。
    let result = validate(
        PackageShape {
            instructions: 1,
            references: 0,
            artifacts: 0,
            memories: 0,
        },
        package.provenance(),
        package.trust_labels(),
    );
    assert!(
        matches!(
            result,
            Err(TrustError::HistoricalInstructionInInstructions { .. })
        ),
        "validate 必须拒绝历史轮进指令位，实际 {result:?}"
    );
}
