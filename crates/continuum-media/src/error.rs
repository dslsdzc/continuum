//! 本块的错误类型（设计 §5.1）。

use continuum_operator::OperatorError;

/// 本块的错误类型。三臂见 Task 4／5。
#[derive(Debug, thiserror::Error)]
pub enum MediaError {
    /// 注册表拒绝（同 (id, version) 已存在）。判定的持有者是 P1。
    /// **这一臂是包装不是同名臂**：撞车的判定已经有一枚臂（`OperatorError::Duplicate`），
    /// 本块再立一枚 `MediaError::Duplicate` 就是「注册表里有没有这个算子」的第二套词汇表。
    #[error(transparent)]
    Registry(#[from] OperatorError),
}
