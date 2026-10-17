//! 记忆块的唯一错误类型（设计 §3.5）。
//!
//! 臂只随各自 task 的拒绝点追加，不预置未使用的臂。`Persist` 是数据库错误的透传，
//! 不是逻辑拒绝臂。

use continuum_persist::PersistError;

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum MemoryError {
    #[error(transparent)]
    Persist(#[from] PersistError),
    #[error("external_ref 去空白后为空")]
    EmptyExternalRef,
    #[error("source_id 已存在")]
    DuplicateSourceId,
}
