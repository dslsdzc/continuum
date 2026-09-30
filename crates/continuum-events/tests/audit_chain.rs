use continuum_events::audit::{
    verify_chain, AuditKind, AuditRecord, AuditError, GENESIS_HASH,
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
