use continuum_events::audit::{
    record_hash, verify_chain, AuditKind, AuditRecord, AuditError, GENESIS_HASH,
};
use serde_json::json;

fn chain(n: usize) -> Vec<AuditRecord> {
    let mut out: Vec<AuditRecord> = Vec::new();
    for i in 1..=n {
        let prev = out.last().map(|r| r.record_hash.clone()).unwrap_or_else(|| GENESIS_HASH.to_owned());
        let kind = AuditKind::ALL[(i - 1) % AuditKind::ALL.len()];
        let occurred_at = 1_700_000_000_000 + i as i64;
        let payload = json!({"i": i});
        let h = continuum_events::audit::record_hash(&prev, i as i64, kind, occurred_at, &payload);
        out.push(AuditRecord {
            seq: i as i64,
            kind,
            occurred_at,
            payload,
            prev_hash: prev,
            record_hash: h,
        });
    }
    out
}

#[test]
fn eight_audit_kinds_match_spec_313() {
    let names: Vec<&str> = AuditKind::ALL.iter().map(|k| k.as_str()).collect();
    assert_eq!(
        names,
        vec![
            "authority changes",
            "device join",
            "device revoke",
            "capability grants",
            "external effects",
            "contract changes",
            "user approvals",
            "runtime self-update",
        ]
    );
}

#[test]
fn untampered_chain_verifies() {
    verify_chain(&chain(8)).expect("未改动的链应通过校验");
}

#[test]
fn tampered_payload_is_detected() {
    let mut c = chain(3);
    c[1].payload = json!({"i": 999});
    match verify_chain(&c) {
        Err(AuditError::HashMismatch { seq }) => assert_eq!(seq, 2),
        other => panic!("应检出 seq=2 的哈希不匹配，实际 {other:?}"),
    }
}

#[test]
fn tampered_kind_is_detected() {
    let mut c = chain(2);
    c[0].kind = AuditKind::DeviceJoin;
    match verify_chain(&c) {
        Err(AuditError::HashMismatch { seq }) => assert_eq!(seq, 1),
        other => panic!("应检出 seq=1 的哈希不匹配，实际 {other:?}"),
    }
}

#[test]
fn broken_link_is_detected() {
    let mut c = chain(3);
    c[2].prev_hash = GENESIS_HASH.to_owned();
    match verify_chain(&c) {
        Err(AuditError::BrokenLink { seq }) => assert_eq!(seq, 3),
        other => panic!("应检出 seq=3 的断链，实际 {other:?}"),
    }
}

#[test]
fn empty_chain_verifies() {
    verify_chain(&[]).expect("空链应通过校验");
}

#[test]
fn serde_names_match_as_str_for_all_eight_kinds() {
    // Task 7 的 audit_records() 要把库里的 §313 文本反序列化回 AuditKind，
    // 写入用 as_str、读回用 serde rename，两者必须一致，
    // 否则库里的记录读不回来。往返测试是对称的，发现不了这类偏差。
    for k in AuditKind::ALL {
        let wire = serde_json::to_string(&k).expect("可序列化");
        assert_eq!(
            wire,
            format!("\"{}\"", k.as_str()),
            "serde rename 与 as_str 不一致: {k:?}"
        );
    }
}

#[test]
fn record_hash_matches_the_frozen_vector() {
    // 黄金向量锁定 record_hash 的输入顺序与大端编码。
    // 期望值由独立的 Python hashlib 实现按同一输入顺序算出，非本实现自产；
    // 推导命令写在报告里，可复现。
    let payload = json!({"effect": "push_branch", "target": "origin/main"});
    let got = record_hash(
        GENESIS_HASH,
        1,
        AuditKind::ExternalEffects,
        1_700_000_000_000,
        &payload,
    );
    assert_eq!(
        got, "ec9b6a9a3fc381a83dd6daca1a2d9fee76d0cf3ea4bbfd26c46688eba8b39f87",
        "record_hash 的输入顺序或编码被改动；既有审计记录将全部失效"
    );
}

#[test]
fn record_hash_is_independent_of_payload_key_insertion_order() {
    // serde_json 默认用有序 map；若启用 preserve_order 功能，
    // 键序会变成插入序依赖，既有审计记录的哈希会全部失效。
    // 该断言把「键序无关」这一假设锁死。
    let a = json!({"a": 1, "b": 2});
    let b = json!({"b": 2, "a": 1});
    assert_eq!(
        record_hash(GENESIS_HASH, 1, AuditKind::DeviceJoin, 1, &a),
        record_hash(GENESIS_HASH, 1, AuditKind::DeviceJoin, 1, &b),
        "payload 的键序不应影响哈希；此处失败说明 serde_json 启用了 preserve_order"
    );
}
