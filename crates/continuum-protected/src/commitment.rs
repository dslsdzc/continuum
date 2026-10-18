//! 一组规则的承诺：`Unavailable` 或 `Committed { version, digest }`（设计 §4.2）。

use crate::error::ProtectedError;
use crate::version::{RuleDigest, RuleVersion, sha256};

/// 一组规则在某一版本上的承诺。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleCommitment {
    state: CommitmentState,
}

/// 私有状态：`Unavailable` 显式表示「本轮没有这一组的规则集上游」。
#[derive(Debug, Clone, PartialEq, Eq)]
enum CommitmentState {
    Unavailable,
    Committed { version: RuleVersion, digest: RuleDigest },
}

impl RuleCommitment {
    /// 该组规则的上游归属方今天不存在／未冻结。显式记「无」，不是「空规则」。
    pub fn unavailable() -> Self {
        Self {
            state: CommitmentState::Unavailable,
        }
    }

    /// 唯一的提交入口：摘要由本函数从字节算出，调用方给不出一个伪造的摘要。
    /// 空字节是 `EmptyRules`：一份「空规则」不是一份规则。
    pub fn committed(version: RuleVersion, rules: &[u8]) -> Result<Self, ProtectedError> {
        if rules.is_empty() {
            return Err(ProtectedError::EmptyRules);
        }
        Ok(Self {
            state: CommitmentState::Committed {
                version,
                digest: RuleDigest::from_bytes(sha256(rules)),
            },
        })
    }

    /// 已提交的版本；`Unavailable` 为 `None`。
    pub fn version(&self) -> Option<RuleVersion> {
        match &self.state {
            CommitmentState::Unavailable => None,
            CommitmentState::Committed { version, .. } => Some(*version),
        }
    }

    /// 已提交的摘要；`Unavailable` 为 `None`。
    pub fn digest(&self) -> Option<&RuleDigest> {
        match &self.state {
            CommitmentState::Unavailable => None,
            CommitmentState::Committed { digest, .. } => Some(digest),
        }
    }

    /// 以同一份字节重算摘要并比对；`Unavailable` 恒为 `false`。
    pub fn matches(&self, rules: &[u8]) -> bool {
        match &self.state {
            CommitmentState::Unavailable => false,
            CommitmentState::Committed { digest, .. } => sha256(rules) == *digest.as_bytes(),
        }
    }
}
