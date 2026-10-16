//! 本块的错误类型（设计 §5.1）。

use continuum_operator::{BackendId, OperatorError, OperatorId};

/// 本块的错误类型（设计 §5.1）。
///
/// 三臂各自的判定持有者：`UnverifiableDeterminism` 见 §5.4（本块的注册期规则）、
/// `Registry` 是 P1 的注册表判定（本型只带出来）；`NotPermitted` 由 §34 的授权规则落地时加。
#[derive(Debug, thiserror::Error)]
pub enum MediaError {
    /// §5.4 的注册期规则不成立：声明 `Deterministic` 的算子混进了名单外的 backend。
    /// 判定的持有者在本块（§5.4）。
    #[error("算子 {operator} 的候选 backend {backend} 不在逐位可复现名单内（§5.4）")]
    UnverifiableDeterminism {
        operator: OperatorId,
        backend: BackendId,
    },

    /// 注册表拒绝（同 (id, version) 已存在）。判定的持有者是 P1。
    /// **这一臂是包装不是同名臂**：撞车的判定已经有一枚臂（`OperatorError::Duplicate`），
    /// 本块再立一枚 `MediaError::Duplicate` 就是「注册表里有没有这个算子」的第二套词汇表。
    #[error(transparent)]
    Registry(#[from] OperatorError),
}
