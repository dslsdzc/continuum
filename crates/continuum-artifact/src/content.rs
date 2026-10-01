//! 内容寻址键（ENG-003）。

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// 内容身份。相同内容必须产生相同值；不同内容不得产生相同值。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentHash(String);

impl ContentHash {
    pub fn of(bytes: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        Self(hex(&hasher.finalize()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// 小写十六进制编码。与 continuum-events 的同类辅助函数重复，
// 见本计划「遗留」一节。
fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(DIGITS[(b >> 4) as usize] as char);
        out.push(DIGITS[(b & 0x0f) as usize] as char);
    }
    out
}
