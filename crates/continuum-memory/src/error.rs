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
    #[error("statement 去空白后为空")]
    EmptyStatement,
    #[error("sources 不能为空")]
    EmptySourceSet,
    #[error("scope 集合不能为空")]
    EmptyScopeSet,
    #[error("anchored scope 必须给 anchor")]
    AnchorRequired,
    #[error("当前 scope 不允许 anchor")]
    AnchorNotAllowed,
    #[error("source 不存在")]
    UnknownSource,
    #[error("source 已被删除")]
    SourceRemoved,
    #[error("job id 已存在")]
    DuplicateJobId,
    #[error("job 不存在")]
    UnknownJob,
    #[error("job 不在 Running 状态")]
    JobNotRunning,
    #[error("触发携带的 source 不在输入集合")]
    TriggerSourceOutsideInput,
    #[error("触发 source 的 kind 不符")]
    WrongTriggerSourceKind,
    #[error("触发集合为空")]
    EmptyTriggerSet,
    #[error("触发时间晚于作业开始时间")]
    InvalidTriggerTime,
}
