//! 版本号与摘要（设计 §3.6 第 1、2 行、§4.2）。

use sha2::{Digest, Sha256};

/// 一组规则的版本号。0 不是合法版本。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RuleVersion(u64);

impl RuleVersion {
    /// 构造版本号；0 返回 `None`。
    pub fn new(value: u64) -> Option<Self> {
        if value == 0 { None } else { Some(Self(value)) }
    }

    /// 版本号的数值。
    pub fn get(&self) -> u64 {
        self.0
    }
}

/// 一组规则字节的 SHA-256。字段私有；crate 外唯一的产生路径是
/// `RuleCommitment::committed`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RuleDigest([u8; 32]);

impl RuleDigest {
    /// 64 个十六进制字符，供照片与日志使用。
    pub fn to_hex(&self) -> String {
        hex(&self.0)
    }

    /// crate 内构造：`committed` 造摘要时使用。crate 外不可见。
    pub(crate) fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// crate 内读 32 字节：规范编码取原始摘要，不取 64 个 ASCII 字节。
    pub(crate) fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

/// `RuleDigest` / `CoreFingerprint` 共用的十六进制编码。
pub(crate) fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;

    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// 规则字节与规范编码共用的 SHA-256。
pub(crate) fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}
