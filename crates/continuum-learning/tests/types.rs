//! 未编号的形状测试（计划 Task 4）：记录、六类目标与提案的形状。
//!
//! **`subject` 是 `String`，本测试不校验它是否指向受保护语义**——那是应用侧义务
//! （设计 §10.2 第 12 条）。这里只钉字段类型、getter 与指纹读取。

use continuum_learning::{LearningProposal, LearningRecord, LearningTarget, ObservationKind};
use continuum_protected::{
    ProtectedCore, ProtectedCoreParts, RuleCommitment, RuleVersion, seal,
};

fn version(value: u64) -> RuleVersion {
    RuleVersion::new(value).expect("1..=u64::MAX 是合法版本")
}

fn sample_core() -> ProtectedCore {
    seal(ProtectedCoreParts {
        authority_rules: RuleCommitment::committed(version(1), b"authority-rules").unwrap(),
        capability_rules: RuleCommitment::unavailable(),
        contract_rules: RuleCommitment::unavailable(),
        audit_rules: RuleCommitment::unavailable(),
        effect_rules: RuleCommitment::unavailable(),
    })
    .expect("一组已提交是合法的部分核心")
}

#[test]
fn learning_record_is_a_plain_literal() {
    let record = LearningRecord {
        kind: ObservationKind::PlanRework,
        subject: "plan-42".to_string(),
        evidence_refs: vec!["evidence-1".to_string(), "evidence-2".to_string()],
    };

    assert_eq!(record.kind, ObservationKind::PlanRework);
    assert_eq!(record.subject, "plan-42");
    assert_eq!(record.evidence_refs, ["evidence-1", "evidence-2"]);
}

#[test]
fn proposals_carry_their_inputs_and_the_core_fingerprint() {
    let core = sample_core();
    let fingerprint = core.fingerprint();

    for (index, target) in LearningTarget::ALL.into_iter().enumerate() {
        let subject = format!("subject-{index}");
        let rationale = format!("rationale-{index}");
        let proposal = LearningProposal::new(&core, target, subject.clone(), rationale.clone());

        assert_eq!(proposal.target(), target);
        assert_eq!(proposal.subject(), subject);
        assert_eq!(proposal.rationale(), rationale);
        assert_eq!(proposal.core_fingerprint(), &fingerprint);
        assert!(proposal.is_current_for(&core));
    }
}
