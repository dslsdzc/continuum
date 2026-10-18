//! R1 / R1b / R3b / R5 的照片（设计 §8.1）。
//!
//! 五组指纹敏感性是**逐组表驱动**的：基准核心的指纹为 F0；对五组各做一次**只改
//! 该组**的变异，逐行断言 `Fi != F0`，并断言六个指纹两两不等。
//!
//! **编码漏掉任何一组时，该组那一行会得到 `Fi == F0`，本测试红。**
//! 版本与摘要两种变异都要覆盖：第 1/3/5 行改版本（摘要不变），第 2/4 行改字节
//! （版本不变、摘要变）。只改版本测不出摘要是否进编码，只改字节测不出版本是否进编码。

use std::collections::BTreeSet;

use continuum_protected::{
    ProtectedCoreParts, ProtectedError, RuleCommitment, RuleVersion, seal,
};

fn version(value: u64) -> RuleVersion {
    RuleVersion::new(value).expect("1..=u64::MAX 是合法版本")
}

fn base_parts() -> ProtectedCoreParts {
    ProtectedCoreParts {
        authority_rules: RuleCommitment::committed(version(1), b"authority-rules").unwrap(),
        capability_rules: RuleCommitment::committed(version(2), b"capability-rules").unwrap(),
        contract_rules: RuleCommitment::committed(version(3), b"contract-rules").unwrap(),
        audit_rules: RuleCommitment::committed(version(4), b"audit-rules").unwrap(),
        effect_rules: RuleCommitment::committed(version(5), b"effect-rules").unwrap(),
    }
}

#[test]
fn r1_committed_matches_only_the_same_bytes_and_has_a_64_hex_digest() {
    let commitment = RuleCommitment::committed(version(7), b"abc").expect("非空字节应能提交");
    assert!(commitment.matches(b"abc"), "同一份字节必须匹配");
    assert!(!commitment.matches(b"abd"), "不同字节必须不匹配");

    let hex = commitment
        .digest()
        .expect("已提交的承诺必有摘要")
        .to_hex();
    assert_eq!(hex.len(), 64, "SHA-256 的十六进制是 64 个字符");
    assert!(
        hex.chars().all(|c| c.is_ascii_hexdigit()),
        "摘要只能是十六进制字符：{hex}"
    );
}

#[test]
fn r1b_empty_rules_are_rejected_but_one_zero_byte_is_a_rule() {
    assert!(
        matches!(
            RuleCommitment::committed(version(1), b""),
            Err(ProtectedError::EmptyRules)
        ),
        "空字节必须被拒绝为 EmptyRules"
    );

    let zero_byte = RuleCommitment::committed(version(1), b"\0").expect("一个零字节是合法规则");
    assert!(zero_byte.matches(b"\0"));
    assert!(!zero_byte.matches(b""));
}

#[test]
fn r3b_rule_version_rejects_zero_and_accepts_the_full_domain() {
    assert_eq!(RuleVersion::new(0), None, "0 不是合法版本");
    assert_eq!(RuleVersion::new(1).map(|v| v.get()), Some(1));
    let max = RuleVersion::new(u64::MAX).expect("u64::MAX 是合法版本");
    assert_eq!(max.get(), u64::MAX);
}

#[test]
fn r5_fingerprint_is_stable_and_sensitive_to_each_of_the_five_groups() {
    let base = base_parts();

    let first = seal(base.clone()).expect("五组都有承诺");
    let second = seal(base.clone()).expect("同一份 parts 的第二次封存");
    assert_eq!(
        first.fingerprint(),
        second.fingerprint(),
        "同一份输入两次封存必须得到同一个指纹"
    );
    let f0 = first.fingerprint();

    let mut authority_version = base.clone();
    authority_version.authority_rules =
        RuleCommitment::committed(version(9), b"authority-rules").unwrap();

    let mut capability_digest = base.clone();
    capability_digest.capability_rules =
        RuleCommitment::committed(version(2), b"capability-rules-v2").unwrap();

    let mut contract_version = base.clone();
    contract_version.contract_rules =
        RuleCommitment::committed(version(9), b"contract-rules").unwrap();

    let mut audit_digest = base.clone();
    audit_digest.audit_rules = RuleCommitment::committed(version(4), b"audit-rules-v2").unwrap();

    let mut effect_version = base.clone();
    effect_version.effect_rules =
        RuleCommitment::committed(version(9), b"effect-rules").unwrap();

    let mutants = [
        ("authority_rules", authority_version),
        ("capability_rules", capability_digest),
        ("contract_rules", contract_version),
        ("audit_rules", audit_digest),
        ("effect_rules", effect_version),
    ];

    let mut fingerprints = BTreeSet::new();
    fingerprints.insert(f0.to_hex());

    for (group, parts) in mutants {
        let fingerprint = seal(parts).expect("变异后的核心仍有一组以上承诺").fingerprint();
        assert_ne!(
            fingerprint, f0,
            "只改 {group} 后指纹必须变化：该组没有进入规范编码"
        );
        assert!(
            fingerprints.insert(fingerprint.to_hex()),
            "只改 {group} 后的指纹与已有指纹重复：规范编码漏掉了该组"
        );
    }

    assert_eq!(fingerprints.len(), 6, "F0 与五个变异指纹必须两两不等");
}
