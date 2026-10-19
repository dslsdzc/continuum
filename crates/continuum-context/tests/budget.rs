//! Task 5：预算、截断与去重。
//!
//! 预算的顺序照设计 §5.3：先丢空串，再按输入序截断；artifacts 先去重后截断；
//! 三个上限互相独立，`max_* == 0` 不是错误。

mod support;

use continuum_artifact::ArtifactId;
use continuum_context::{
    ContextBudget, ContextCandidate, ContextRequest, Reference, compile,
};
use continuum_memory::MemoryScope;
use continuum_trust::{ArtifactProducer, ContextProvenance};
use support::{CountingMemory, test_claim};

fn instruction(text: &str) -> ContextCandidate<String> {
    ContextCandidate {
        content: text.to_string(),
        provenance: ContextProvenance::user_instruction(),
    }
}

fn reference(text: &str) -> ContextCandidate<Reference> {
    ContextCandidate {
        content: Reference::Text(text.to_string()),
        provenance: ContextProvenance::project_data(),
    }
}

fn artifact(id: &str) -> ContextCandidate<ArtifactId> {
    ContextCandidate {
        content: ArtifactId::new(id),
        provenance: ContextProvenance::artifact(ArtifactProducer::Project),
    }
}

fn budget(
    max_references: usize,
    max_artifacts: usize,
    max_memories: usize,
) -> ContextBudget {
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
    scopes: Vec<MemoryScope>,
    max_memories: usize,
) -> ContextRequest {
    ContextRequest {
        instructions,
        references,
        artifacts,
        scopes,
        anchor: None,
        memory_query: if max_memories > 0 {
            Some("x".into())
        } else {
            None
        },
    }
}

#[test]
fn references_are_truncated_in_input_order() {
    let memory = CountingMemory::new(Vec::new());
    let request = request(
        Vec::new(),
        vec![reference("first"), reference("second")],
        Vec::new(),
        Vec::new(),
        0,
    );

    let package = compile(request, budget(1, 0, 0), &memory).expect("截断不是错误");

    assert_eq!(package.references(), [Reference::Text("first".into())]);
    assert!(
        !package
            .references()
            .contains(&Reference::Text("second".into())),
        "第 2 条引用应被截断"
    );
}

#[test]
fn artifact_ids_are_deduped_then_truncated() {
    let memory = CountingMemory::new(Vec::new());
    let package = compile(
        request(
            Vec::new(),
            Vec::new(),
            vec![artifact("A"), artifact("A"), artifact("B")],
            Vec::new(),
            0,
        ),
        budget(0, 2, 0),
        &memory,
    )
    .expect("重复 id 静默丢弃，不是错误");
    assert_eq!(
        package.artifacts(),
        [ArtifactId::new("A"), ArtifactId::new("B")]
    );

    let memory = CountingMemory::new(Vec::new());
    let package = compile(
        request(
            Vec::new(),
            Vec::new(),
            vec![artifact("A"), artifact("B")],
            Vec::new(),
            0,
        ),
        budget(0, 1, 0),
        &memory,
    )
    .expect("截断不是错误");
    assert_eq!(package.artifacts(), [ArtifactId::new("A")]);
}

#[test]
fn the_three_budgets_are_independent() {
    let memory = CountingMemory::new(vec![
        test_claim("c1", "一"),
        test_claim("c2", "二"),
    ]);
    let request = request(
        Vec::new(),
        vec![reference("r1"), reference("r2")],
        vec![artifact("A"), artifact("B")],
        vec![MemoryScope::Personal],
        2,
    );

    let package = compile(request, budget(1, 0, 2), &memory).expect("三个上限互相独立");

    assert_eq!(package.references().len(), 1, "max_references = 1");
    assert_eq!(package.artifacts().len(), 0, "max_artifacts = 0");
    assert_eq!(package.memories().len(), 2, "max_memories = 2");
}

#[test]
fn zero_budgets_do_not_drop_instructions() {
    let memory = CountingMemory::new(Vec::new());
    let request = request(
        vec![instruction("一"), instruction("二")],
        Vec::new(),
        Vec::new(),
        vec![MemoryScope::Personal],
        0,
    );

    let package = compile(request, budget(0, 0, 0), &memory).expect("预算 0 不是错误");

    assert_eq!(package.instructions().len(), 2, "指令不受预算约束");
    assert!(package.references().is_empty());
    assert!(package.artifacts().is_empty());
    assert!(package.memories().is_empty());
    assert_eq!(package.provenance().len(), 2);
    assert_eq!(package.trust_labels().len(), 2);
    assert_eq!(memory.calls(), 0, "max_memories = 0 不得调检索面");
}

#[test]
fn truncated_candidates_do_not_appear_anywhere() {
    let memory = CountingMemory::new(vec![
        test_claim("c1", "一"),
        test_claim("c2", "二"),
        test_claim("c3", "三"),
    ]);
    let request = request(
        Vec::new(),
        vec![reference("r1"), reference("r2")],
        vec![artifact("A"), artifact("B")],
        vec![MemoryScope::Personal],
        2,
    );

    let package = compile(request, budget(1, 1, 2), &memory).expect("截断不是错误");

    // 被截断的引用、artifact 与 claim 都不出现在任何内容数组里。
    assert_eq!(package.references(), [Reference::Text("r1".into())]);
    assert!(!package.references().contains(&Reference::Text("r2".into())));

    assert_eq!(package.artifacts(), [ArtifactId::new("A")]);
    assert!(!package.artifacts().contains(&ArtifactId::new("B")));

    assert_eq!(package.memories().len(), 2);
    assert!(
        package
            .memories()
            .iter()
            .all(|claim| claim.id().as_str() != "c3"),
        "被截断的 claim 不得出现"
    );
    assert!(
        !package
            .provenance()
            .contains(&ContextProvenance::memory("c3")),
        "被截断 claim 的 provenance 也不得出现"
    );
    assert!(
        package
            .instructions()
            .iter()
            .all(|text| text != "r2" && text != "B" && text != "c3"),
        "被截断的候选不得渗入指令位"
    );
}

#[test]
fn duplicate_artifact_ids_are_not_an_error() {
    let memory = CountingMemory::new(Vec::new());
    let result = compile(
        request(
            Vec::new(),
            Vec::new(),
            vec![artifact("A"), artifact("A")],
            Vec::new(),
            0,
        ),
        budget(0, 2, 0),
        &memory,
    );

    let package = result.expect("重复 artifact id 不得返回 Err");
    assert_eq!(package.artifacts(), [ArtifactId::new("A")]);
}
