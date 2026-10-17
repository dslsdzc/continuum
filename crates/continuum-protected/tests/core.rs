//! R2 / R3 / R4 与 `ProtectedSemantic` 的基本照片（设计 §8.1、§4.4）。
//!
//! - R2：`Unavailable` 不是「空摘要」：没有版本、没有摘要，匹配恒 false。
//! - R3：拒绝五组全空的核心；放行一组已提交的部分核心。
//! - R4：用字段名构造 `ProtectedCoreParts`，封存后逐组断言访问器返回的正是该组。
//!   **红条件**：把 `authority_rules` 与 `contract_rules` 的字段对调，本测试必红。
//! - `ProtectedSemantic`：`ALL` 五枚、§321 原文逐字、`parse` 严格逆。

use continuum_protected::{
    ProtectedCoreParts, ProtectedError, ProtectedSemantic, RuleCommitment, RuleVersion, seal,
};

#[test]
fn r2_unavailable_commitment_has_no_version_and_never_matches() {
    let commitment = RuleCommitment::unavailable();
    assert_eq!(commitment.version(), None);
    assert_eq!(commitment.digest(), None);
    assert!(!commitment.matches(b"anything"), "Unavailable 对任意字节都不匹配");
    assert!(!commitment.matches(b""), "Unavailable 也不等于「匹配空字节」");
}

#[test]
fn r3_seal_rejects_empty_shell_but_accepts_partial_core() {
    let empty_shell = ProtectedCoreParts {
        authority_rules: RuleCommitment::unavailable(),
        capability_rules: RuleCommitment::unavailable(),
        contract_rules: RuleCommitment::unavailable(),
        audit_rules: RuleCommitment::unavailable(),
        effect_rules: RuleCommitment::unavailable(),
    };
    assert!(
        matches!(seal(empty_shell), Err(ProtectedError::NothingCommitted)),
        "五组全 Unavailable 必须被拒绝为 NothingCommitted"
    );

    let authority = RuleCommitment::committed(RuleVersion::new(1).unwrap(), b"authority-v1")
        .expect("非空字节应能提交");
    let partial_core = ProtectedCoreParts {
        authority_rules: authority,
        capability_rules: RuleCommitment::unavailable(),
        contract_rules: RuleCommitment::unavailable(),
        audit_rules: RuleCommitment::unavailable(),
        effect_rules: RuleCommitment::unavailable(),
    };
    let core = seal(partial_core).expect("一组已提交、四组缺失是合法的部分核心");

    assert_eq!(core.authority_rules().version(), RuleVersion::new(1));
    assert!(core.authority_rules().digest().is_some());
    assert!(core.authority_rules().matches(b"authority-v1"));

    assert_eq!(core.capability_rules().version(), None);
    assert_eq!(core.contract_rules().version(), None);
    assert_eq!(core.audit_rules().version(), None);
    assert_eq!(core.effect_rules().version(), None);
}

#[test]
fn r4_fields_are_not_transposed() {
    let parts = ProtectedCoreParts {
        authority_rules: RuleCommitment::committed(
            RuleVersion::new(1).unwrap(),
            b"authority-rules",
        )
        .unwrap(),
        capability_rules: RuleCommitment::committed(
            RuleVersion::new(2).unwrap(),
            b"capability-rules",
        )
        .unwrap(),
        contract_rules: RuleCommitment::committed(
            RuleVersion::new(3).unwrap(),
            b"contract-rules",
        )
        .unwrap(),
        audit_rules: RuleCommitment::committed(RuleVersion::new(4).unwrap(), b"audit-rules")
            .unwrap(),
        effect_rules: RuleCommitment::committed(RuleVersion::new(5).unwrap(), b"effect-rules")
            .unwrap(),
    };
    let core = seal(parts).expect("五组都有承诺的核心应能封存");

    assert_eq!(core.authority_rules().version(), RuleVersion::new(1));
    assert_eq!(core.capability_rules().version(), RuleVersion::new(2));
    assert_eq!(core.contract_rules().version(), RuleVersion::new(3));
    assert_eq!(core.audit_rules().version(), RuleVersion::new(4));
    assert_eq!(core.effect_rules().version(), RuleVersion::new(5));

    assert!(core.authority_rules().matches(b"authority-rules"));
    assert!(core.capability_rules().matches(b"capability-rules"));
    assert!(core.contract_rules().matches(b"contract-rules"));
    assert!(core.audit_rules().matches(b"audit-rules"));
    assert!(core.effect_rules().matches(b"effect-rules"));
}

#[test]
fn protected_semantic_round_trips_section_321_strings() {
    assert_eq!(ProtectedSemantic::ALL.len(), 5);

    let table = [
        (ProtectedSemantic::Authority, "Authority semantics"),
        (ProtectedSemantic::Contract, "Contract semantics"),
        (ProtectedSemantic::Permission, "Permission invariants"),
        (ProtectedSemantic::Approval, "User approval requirements"),
        (ProtectedSemantic::SecurityPolicy, "Core security policy"),
    ];
    for (semantic, text) in table {
        assert_eq!(semantic.as_str(), text, "§321 原文逐字");
        assert_eq!(ProtectedSemantic::parse(text), Some(semantic), "parse 是严格逆");
    }

    assert_eq!(ProtectedSemantic::parse("Authority Semantics"), None);
    assert_eq!(ProtectedSemantic::parse("authority semantics"), None);
    assert_eq!(ProtectedSemantic::parse(""), None);
}
