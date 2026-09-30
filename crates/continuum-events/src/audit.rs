//! §313 Audit Log 与 §114 的防篡改。
//!
//! v0.1 取法：append-only、每条含 prev_hash 与 record_hash 构成哈希链、
//! 保留期不设上限、不做外部时间戳与签名。见 P0 设计第 6 节。

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// 链首的 prev_hash。
pub const GENESIS_HASH: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

/// 哈希域分隔前缀，防止与其它哈希用途串用。
const DOMAIN: &[u8] = b"continuum.audit.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AuditKind {
    #[serde(rename = "authority changes")]
    AuthorityChanges,
    #[serde(rename = "device join")]
    DeviceJoin,
    #[serde(rename = "device revoke")]
    DeviceRevoke,
    #[serde(rename = "capability grants")]
    CapabilityGrants,
    #[serde(rename = "external effects")]
    ExternalEffects,
    #[serde(rename = "contract changes")]
    ContractChanges,
    #[serde(rename = "user approvals")]
    UserApprovals,
    #[serde(rename = "runtime self-update")]
    RuntimeSelfUpdate,
}

impl AuditKind {
    pub const ALL: [AuditKind; 8] = [
        AuditKind::AuthorityChanges,
        AuditKind::DeviceJoin,
        AuditKind::DeviceRevoke,
        AuditKind::CapabilityGrants,
        AuditKind::ExternalEffects,
        AuditKind::ContractChanges,
        AuditKind::UserApprovals,
        AuditKind::RuntimeSelfUpdate,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            AuditKind::AuthorityChanges => "authority changes",
            AuditKind::DeviceJoin => "device join",
            AuditKind::DeviceRevoke => "device revoke",
            AuditKind::CapabilityGrants => "capability grants",
            AuditKind::ExternalEffects => "external effects",
            AuditKind::ContractChanges => "contract changes",
            AuditKind::UserApprovals => "user approvals",
            AuditKind::RuntimeSelfUpdate => "runtime self-update",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuditRecord {
    pub seq: i64,
    pub kind: AuditKind,
    /// Unix 毫秒。
    pub occurred_at: i64,
    pub payload: Value,
    pub prev_hash: String,
    pub record_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AuditError {
    #[error("seq={seq} 的 record_hash 与重算结果不符")]
    HashMismatch { seq: i64 },
    #[error("seq={seq} 的 prev_hash 与前一条的 record_hash 不符")]
    BrokenLink { seq: i64 },
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(DIGITS[(b >> 4) as usize] as char);
        s.push(DIGITS[(b & 0x0f) as usize] as char);
    }
    s
}

/// 输入顺序固定为 域前缀 → prev_hash → seq → kind → occurred_at → payload。
/// 改变顺序会改变所有既有记录的重算结果，视为破坏兼容。
pub fn record_hash(
    prev_hash: &str,
    seq: i64,
    kind: AuditKind,
    occurred_at: i64,
    payload: &Value,
) -> String {
    let mut h = Sha256::new();
    h.update(DOMAIN);
    h.update(prev_hash.as_bytes());
    h.update(seq.to_be_bytes());
    h.update(kind.as_str().as_bytes());
    h.update(occurred_at.to_be_bytes());
    h.update(serde_json::to_vec(payload).expect("payload 可序列化"));
    hex(&h.finalize())
}

/// 校验顺序、断链与逐条哈希。空链通过。
pub fn verify_chain(records: &[AuditRecord]) -> Result<(), AuditError> {
    let mut expected_prev = GENESIS_HASH.to_owned();
    for r in records {
        if r.prev_hash != expected_prev {
            return Err(AuditError::BrokenLink { seq: r.seq });
        }
        let recomputed =
            record_hash(&r.prev_hash, r.seq, r.kind, r.occurred_at, &r.payload);
        if recomputed != r.record_hash {
            return Err(AuditError::HashMismatch { seq: r.seq });
        }
        expected_prev = r.record_hash.clone();
    }
    Ok(())
}
