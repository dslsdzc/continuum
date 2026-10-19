//! Task 2：请求、候选与预算的构造面。

use continuum_artifact::ArtifactId;
use continuum_context::{
    ContextBudget, ContextCandidate, ContextRequest, Reference,
};
use continuum_memory::MemoryScope;
use continuum_trust::{ArtifactProducer, ContextProvenance};

#[test]
fn a_request_carries_the_three_memory_controls() {
    let request = ContextRequest {
        instructions: vec![ContextCandidate {
            content: "当前指令".to_string(),
            provenance: ContextProvenance::user_instruction(),
        }],
        references: vec![ContextCandidate {
            content: Reference::Text("项目片段".into()),
            provenance: ContextProvenance::project_data(),
        }],
        artifacts: vec![ContextCandidate {
            content: ArtifactId::new("artifact-1"),
            provenance: ContextProvenance::artifact(ArtifactProducer::Project),
        }],
        scopes: vec![MemoryScope::Personal],
        anchor: None,
        memory_query: Some("x".into()),
    };

    assert_eq!(request.scopes, vec![MemoryScope::Personal], "scopes 必须原样保留");
    assert_eq!(request.anchor, None, "anchor 必须原样保留");
    assert_eq!(
        request.memory_query.as_deref(),
        Some("x"),
        "memory_query 必须原样保留"
    );

    // 三个内容容器各自能装下对应类型的候选。
    assert_eq!(request.instructions.len(), 1);
    assert_eq!(request.instructions[0].content, "当前指令");
    assert_eq!(request.references.len(), 1);
    assert_eq!(
        request.references[0].content,
        Reference::Text("项目片段".into())
    );
    assert_eq!(request.artifacts.len(), 1);
    assert_eq!(request.artifacts[0].content, ArtifactId::new("artifact-1"));
}

#[test]
fn a_candidate_carries_content_and_provenance() {
    let text_candidate = ContextCandidate {
        content: "文本".to_string(),
        provenance: ContextProvenance::user_instruction(),
    };
    assert_eq!(text_candidate.content, "文本");
    assert_eq!(
        text_candidate.provenance,
        ContextProvenance::user_instruction()
    );

    let reference_candidate = ContextCandidate {
        content: Reference::Text("项目".into()),
        provenance: ContextProvenance::project_data(),
    };
    assert_eq!(
        reference_candidate.content,
        Reference::Text("项目".into())
    );
    assert_eq!(
        reference_candidate.provenance,
        ContextProvenance::project_data()
    );

    let artifact_candidate = ContextCandidate {
        content: ArtifactId::new("artifact-2"),
        provenance: ContextProvenance::artifact(ArtifactProducer::Project),
    };
    assert_eq!(artifact_candidate.content, ArtifactId::new("artifact-2"));
    assert_eq!(
        artifact_candidate.provenance,
        ContextProvenance::artifact(ArtifactProducer::Project)
    );
}

#[test]
fn the_budget_is_three_independent_counts() {
    let budget = ContextBudget {
        max_references: 0,
        max_artifacts: 2,
        max_memories: 1,
    };

    assert_eq!(budget.max_references, 0, "三个上限互不派生");
    assert_eq!(budget.max_artifacts, 2, "三个上限互不覆盖");
    assert_eq!(budget.max_memories, 1, "三个上限各自保留");
}
