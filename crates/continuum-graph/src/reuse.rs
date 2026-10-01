//! 增量重算的复用判定（§305，ENG-002 裁定）。

use continuum_artifact::ContentHash;
use continuum_operator::{Determinism, Operator, OperatorVersion};
use serde::{Deserialize, Serialize};

/// 可复用的凭据。只有 `Deterministic` 的 Operator 才产生它。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheKey {
    pub input_hash: ContentHash,
    pub operator_version: OperatorVersion,
}

/// 为非确定 Operator 返回 `None`。该判定是 ENG-002 裁定的落地点：
/// 非确定节点的输出不写缓存键，因此永远不满足复用前提。
pub fn cache_key(operator: &Operator, input_hash: ContentHash) -> Option<CacheKey> {
    match operator.determinism {
        Determinism::Deterministic => Some(CacheKey {
            input_hash,
            operator_version: operator.version,
        }),
        Determinism::NonDeterministic => None,
    }
}

/// `§305` 的复用条件，加 ENG-002 的 determinism 前置。
///
/// 四项：Operator 确定性、输入哈希未变、Operator 版本未变、Contract 未受影响。
pub fn can_reuse(
    operator: &Operator,
    cached: Option<&CacheKey>,
    current_input_hash: &ContentHash,
    contract_affected: bool,
) -> bool {
    if operator.determinism != Determinism::Deterministic {
        return false;
    }
    if contract_affected {
        return false;
    }
    match cached {
        Some(key) => {
            key.input_hash == *current_input_hash && key.operator_version == operator.version
        }
        None => false,
    }
}
