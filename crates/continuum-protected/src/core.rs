//! `ProtectedCore` 的五个字段、封存入口与核心指纹（设计 §4.1、§4.2）。

use crate::commitment::RuleCommitment;
use crate::error::ProtectedError;
use crate::version::{hex, sha256};

/// §322 的 `ProtectedCore`。字段名、次序、五个——逐字照规范；字段私有。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtectedCore {
    authority_rules: RuleCommitment,
    capability_rules: RuleCommitment,
    contract_rules: RuleCommitment,
    audit_rules: RuleCommitment,
    effect_rules: RuleCommitment,
}

/// [`seal`] 的输入。五个字段名与 §322 逐字相同。
///
/// 字段 pub：它是**封存前**的零件，封存后由 [`ProtectedCore`] 私有持有。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtectedCoreParts {
    pub authority_rules: RuleCommitment,
    pub capability_rules: RuleCommitment,
    pub contract_rules: RuleCommitment,
    pub audit_rules: RuleCommitment,
    pub effect_rules: RuleCommitment,
}

/// 五组承诺的规范编码的 SHA-256。不是第六个字段：每次按五个字段现算。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CoreFingerprint([u8; 32]);

impl CoreFingerprint {
    /// 64 个十六进制字符。
    pub fn to_hex(&self) -> String {
        hex(&self.0)
    }
}

/// 封存入口：唯一构造 [`ProtectedCore`] 的公开路径。
///
/// 只拒绝五组全 `Unavailable`（`NothingCommitted`）。一组已提交、四组缺失是合法的
/// 部分核心。本函数不接收字节，**不会**返回 `EmptyRules`。
pub fn seal(parts: ProtectedCoreParts) -> Result<ProtectedCore, ProtectedError> {
    let all_unavailable = parts
        .authority_rules
        .version()
        .is_none()
        && parts.capability_rules.version().is_none()
        && parts.contract_rules.version().is_none()
        && parts.audit_rules.version().is_none()
        && parts.effect_rules.version().is_none();

    if all_unavailable {
        return Err(ProtectedError::NothingCommitted);
    }

    Ok(ProtectedCore {
        authority_rules: parts.authority_rules,
        capability_rules: parts.capability_rules,
        contract_rules: parts.contract_rules,
        audit_rules: parts.audit_rules,
        effect_rules: parts.effect_rules,
    })
}

impl ProtectedCore {
    pub fn authority_rules(&self) -> &RuleCommitment {
        &self.authority_rules
    }

    pub fn capability_rules(&self) -> &RuleCommitment {
        &self.capability_rules
    }

    pub fn contract_rules(&self) -> &RuleCommitment {
        &self.contract_rules
    }

    pub fn audit_rules(&self) -> &RuleCommitment {
        &self.audit_rules
    }

    pub fn effect_rules(&self) -> &RuleCommitment {
        &self.effect_rules
    }

    /// 五组承诺的规范编码的 SHA-256。**不是第六个字段**：每次现算，不存储。
    ///
    /// 编码次序与字段形状见 crate 文档；编码只在本版本内解释，不承诺跨版本稳定。
    pub fn fingerprint(&self) -> CoreFingerprint {
        let mut bytes = Vec::new();

        for (group, commitment) in [
            ("authority", &self.authority_rules),
            ("capability", &self.capability_rules),
            ("contract", &self.contract_rules),
            ("audit", &self.audit_rules),
            ("effect", &self.effect_rules),
        ] {
            bytes.push(group.len() as u8);
            bytes.extend_from_slice(group.as_bytes());
            match commitment.version() {
                None => bytes.push(0),
                Some(version) => {
                    bytes.push(1);
                    bytes.extend_from_slice(&version.get().to_be_bytes());
                    let digest = commitment
                        .digest()
                        .expect("Committed 承诺必有摘要：状态由 committed 唯一构造");
                    bytes.extend_from_slice(digest.as_bytes());
                }
            }
        }

        CoreFingerprint(sha256(&bytes))
    }
}
