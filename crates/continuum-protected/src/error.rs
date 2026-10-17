//! `ProtectedError`：本 crate 唯一的错误类型（设计 §3.5 末、§4.1）。

/// 封存入口与提交入口的错误。
///
/// `seal` 只返回 [`ProtectedError::NothingCommitted`]；[`ProtectedError::EmptyRules`]
/// 只从 `RuleCommitment::committed` 出，`seal` 永远进不去这一臂。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProtectedError {
    /// 五组全部 `Unavailable`：这样的核心不保护任何东西，拒绝封存。
    #[error("五组规则全部缺失：ProtectedCore 不能封存一个空壳")]
    NothingCommitted,
    /// `committed` 收到空字节：一份「空规则」不是一份规则。
    #[error("规则字节为空：空规则不是规则")]
    EmptyRules,
}
