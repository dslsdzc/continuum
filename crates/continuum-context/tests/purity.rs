//! Task 7：无隐藏来源与确定性（J7a／J7b），以及 Review Focus 4 的第一半。
//!
//! `compile` 只依赖三个参数；请求外的 artifact / claim 不出现；两份逐字段相同的
//! 请求在同一检索面下产物相等。

mod support;

use continuum_artifact::ArtifactId;
use continuum_context::{
    ContextBudget, ContextCandidate, ContextRequest, Reference, compile,
};
use continuum_memory::MemoryScope;
use continuum_trust::ContextProvenance;
use support::{CountingMemory, test_claim};

fn budget(max_references: usize, max_artifacts: usize, max_memories: usize) -> ContextBudget {
    ContextBudget {
        max_references,
        max_artifacts,
        max_memories,
    }
}

#[test]
fn only_requested_artifacts_and_returned_claims_appear() {
    let requested = ArtifactId::new("A");
    let not_requested = ArtifactId::new("B");
    let returned_claim = test_claim("c1", "检索面返回");
    let hidden_claim = test_claim("d1", "只作为测试变量");

    let memory = CountingMemory::new(vec![returned_claim.clone()]);
    let request = ContextRequest {
        instructions: Vec::new(),
        references: Vec::new(),
        artifacts: vec![ContextCandidate {
            content: requested.clone(),
            provenance: ContextProvenance::artifact(continuum_trust::ArtifactProducer::Project),
        }],
        scopes: vec![MemoryScope::Personal],
        anchor: None,
        memory_query: None,
    };

    let package = compile(request, budget(0, 1, 2), &memory).expect("编译成功");

    assert_eq!(package.artifacts(), [requested], "只装请求里的 artifact");
    assert!(
        !package.artifacts().contains(&not_requested),
        "请求外的 artifact B 不得出现"
    );
    assert_eq!(
        package.memories(),
        [returned_claim],
        "只装检索面返回的 claim"
    );
    assert!(
        package
            .memories()
            .iter()
            .all(|claim| claim.id().as_str() != hidden_claim.id().as_str()),
        "未交给检索面的 claim D 不得出现"
    );
}

#[test]
fn two_identical_requests_produce_equal_packages() {
    // 两份请求逐字段手写；ContextRequest 不实现 Clone。
    let memory = CountingMemory::new(vec![test_claim("c1", "记忆")]);

    let first = ContextRequest {
        instructions: vec![ContextCandidate {
            content: "指令".to_string(),
            provenance: ContextProvenance::user_instruction(),
        }],
        references: vec![ContextCandidate {
            content: Reference::Text("引用".into()),
            provenance: ContextProvenance::project_data(),
        }],
        artifacts: vec![ContextCandidate {
            content: ArtifactId::new("artifact-1"),
            provenance: ContextProvenance::artifact(continuum_trust::ArtifactProducer::Tool),
        }],
        scopes: vec![MemoryScope::Personal],
        anchor: None,
        memory_query: None,
    };
    let second = ContextRequest {
        instructions: vec![ContextCandidate {
            content: "指令".to_string(),
            provenance: ContextProvenance::user_instruction(),
        }],
        references: vec![ContextCandidate {
            content: Reference::Text("引用".into()),
            provenance: ContextProvenance::project_data(),
        }],
        artifacts: vec![ContextCandidate {
            content: ArtifactId::new("artifact-1"),
            provenance: ContextProvenance::artifact(continuum_trust::ArtifactProducer::Tool),
        }],
        scopes: vec![MemoryScope::Personal],
        anchor: None,
        memory_query: None,
    };

    let first_package = compile(first, budget(1, 1, 2), &memory).expect("第一次编译成功");
    let second_package = compile(second, budget(1, 1, 2), &memory).expect("第二次编译成功");

    assert_eq!(
        first_package, second_package,
        "同一检索面、逐字段相同的请求必须产出相等的包"
    );
}

#[test]
fn compile_reads_no_clock_environment_or_filesystem() {
    let src = include_str!("../src/compile.rs");
    for forbidden in [
        "std::env",
        "env::var",
        "SystemTime",
        "Instant::now",
        "current_dir",
        "std::fs",
        "File::open",
    ] {
        assert!(
            !src.contains(forbidden),
            "compile.rs 不得读取隐藏来源：{forbidden:?}"
        );
    }
}
