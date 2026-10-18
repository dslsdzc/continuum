//! R6 / R7 / R8 / R10 / R11 的照片（设计 §8.1）。
//!
//! 期望字符串一律**手写**，不从 `ALL` 派生：`ALL` 漏一枚时，遍历测不到新臂，
//! 但手写清单会把「少了一枚」暴露成长度或逐枚断言失败。

use std::collections::BTreeSet;

use continuum_learning::{
    LearningProposal, LearningTarget, ObservationKind,
};
use continuum_protected::{
    ProtectedCore, ProtectedCoreParts, ProtectedSemantic, RuleCommitment, RuleVersion, seal,
};

fn version(value: u64) -> RuleVersion {
    RuleVersion::new(value).expect("1..=u64::MAX 是合法版本")
}

fn sample_core() -> ProtectedCore {
    seal(ProtectedCoreParts {
        authority_rules: RuleCommitment::committed(version(1), b"authority-rules").unwrap(),
        capability_rules: RuleCommitment::committed(version(2), b"capability-rules").unwrap(),
        contract_rules: RuleCommitment::unavailable(),
        audit_rules: RuleCommitment::unavailable(),
        effect_rules: RuleCommitment::unavailable(),
    })
    .expect("两组已提交是合法的部分核心")
}

/// R11 的穷尽 `match`：加第七类时本函数编译不过。
fn target_str(target: LearningTarget) -> &'static str {
    match target {
        LearningTarget::RoutingPolicy => target.as_str(),
        LearningTarget::PlanningHeuristics => target.as_str(),
        LearningTarget::ExecutionMethod => target.as_str(),
        LearningTarget::VerificationStrategy => target.as_str(),
        LearningTarget::ContextSelection => target.as_str(),
        LearningTarget::SchedulingHeuristic => target.as_str(),
    }
}

/// R11 的穷尽 `match`：加第六类时本函数编译不过。
fn observation_str(kind: ObservationKind) -> &'static str {
    match kind {
        ObservationKind::PlanRework => kind.as_str(),
        ObservationKind::ModelSkillFailure => kind.as_str(),
        ObservationKind::VerificationMiss => kind.as_str(),
        ObservationKind::ExecutionMethodReliability => kind.as_str(),
        ObservationKind::ContextSelectionNoise => kind.as_str(),
    }
}

/// R11 的穷尽 `match`：加第六类时本函数编译不过。
fn semantic_str(semantic: ProtectedSemantic) -> &'static str {
    match semantic {
        ProtectedSemantic::Authority => semantic.as_str(),
        ProtectedSemantic::Contract => semantic.as_str(),
        ProtectedSemantic::Permission => semantic.as_str(),
        ProtectedSemantic::Approval => semantic.as_str(),
        ProtectedSemantic::SecurityPolicy => semantic.as_str(),
    }
}

#[test]
fn r6_learning_targets_are_exactly_six_handwritten_strings() {
    assert_eq!(LearningTarget::ALL.len(), 6);

    let expected = [
        (LearningTarget::RoutingPolicy, "routing policy"),
        (LearningTarget::PlanningHeuristics, "planning heuristics"),
        (LearningTarget::ExecutionMethod, "execution method"),
        (LearningTarget::VerificationStrategy, "verification strategy"),
        (LearningTarget::ContextSelection, "context selection"),
        (LearningTarget::SchedulingHeuristic, "scheduling heuristic"),
    ];
    for (target, text) in expected {
        assert_eq!(target.as_str(), text);
    }
}

#[test]
fn r7_protected_semantics_are_disjoint_from_learning_targets() {
    let protected = [
        (ProtectedSemantic::Authority, "Authority semantics"),
        (ProtectedSemantic::Contract, "Contract semantics"),
        (ProtectedSemantic::Permission, "Permission invariants"),
        (ProtectedSemantic::Approval, "User approval requirements"),
        (ProtectedSemantic::SecurityPolicy, "Core security policy"),
    ];
    assert_eq!(ProtectedSemantic::ALL.len(), protected.len());
    for (semantic, text) in protected {
        assert_eq!(semantic.as_str(), text, "§321 原文逐字");
    }

    let protected_strings: BTreeSet<&str> = protected.iter().map(|(_, text)| *text).collect();
    let target_strings: BTreeSet<&str> = LearningTarget::ALL
        .iter()
        .map(|target| target.as_str())
        .collect();
    assert!(
        protected_strings.intersection(&target_strings).next().is_none(),
        "§321 的五个串与六类学习目标必须无交集"
    );
}

#[test]
fn r8_proposals_are_bound_to_the_core_fingerprint() {
    let core = sample_core();
    let proposals: Vec<LearningProposal> = LearningTarget::ALL
        .into_iter()
        .map(|target| LearningProposal::new(&core, target, "subject", "rationale"))
        .collect();

    for proposal in &proposals {
        assert!(proposal.is_current_for(&core), "同一枚核心上的提案必须有效");
    }

    let changed = seal(ProtectedCoreParts {
        authority_rules: RuleCommitment::committed(version(2), b"authority-rules").unwrap(),
        capability_rules: RuleCommitment::committed(version(2), b"capability-rules").unwrap(),
        contract_rules: RuleCommitment::unavailable(),
        audit_rules: RuleCommitment::unavailable(),
        effect_rules: RuleCommitment::unavailable(),
    })
    .expect("改动一组承诺后的核心仍合法");

    assert_ne!(changed.fingerprint(), core.fingerprint());
    for proposal in &proposals {
        assert!(
            !proposal.is_current_for(&changed),
            "核心一变，全部旧提案必须过期"
        );
    }
}

#[test]
fn r10_constructing_proposals_does_not_change_the_core() {
    let core = sample_core();
    let before = core.clone();
    let fingerprint_before = core.fingerprint();

    for target in LearningTarget::ALL {
        let _ = LearningProposal::new(&core, target, "subject", "rationale");
    }

    assert_eq!(core, before, "核心在提案构造前后逐字段相等");
    assert_eq!(core.fingerprint(), fingerprint_before, "核心指纹不变");
}

#[test]
fn r11_all_variants_round_trip_through_handwritten_strings() {
    for target in LearningTarget::ALL {
        assert_eq!(target_str(target), target.as_str());
        assert_eq!(LearningTarget::parse(target.as_str()), Some(target));
    }
    assert_eq!(LearningTarget::parse("skill library"), None);
    assert_eq!(LearningTarget::parse("RoutingPolicy"), None);

    let observation_strings = [
        (ObservationKind::PlanRework, "哪些计划经常返工"),
        (ObservationKind::ModelSkillFailure, "哪些模型在哪些 Skill 上失败"),
        (ObservationKind::VerificationMiss, "哪些验证方式经常漏错"),
        (ObservationKind::ExecutionMethodReliability, "哪些 Execution Method 更可靠"),
        (ObservationKind::ContextSelectionNoise, "哪些 Context 选择导致噪声"),
    ];
    assert_eq!(ObservationKind::ALL.len(), observation_strings.len());
    for (kind, text) in observation_strings {
        assert_eq!(observation_str(kind), text, "§216 中文原句逐字");
        assert_eq!(ObservationKind::parse(text), Some(kind));
    }
    assert_eq!(ObservationKind::parse("哪些计划需要返工"), None);
    assert_eq!(ObservationKind::parse(""), None);

    for semantic in ProtectedSemantic::ALL {
        assert_eq!(semantic_str(semantic), semantic.as_str());
        assert_eq!(ProtectedSemantic::parse(semantic.as_str()), Some(semantic));
    }
    assert_eq!(ProtectedSemantic::parse("Authority"), None);
}
