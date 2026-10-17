//! Runtime Learning：六类学习目标、五类观察、记录与提案（设计 §5）。
//!
//! 本 crate 只有一条 workspace 依赖边：`continuum-protected`。学习侧读
//! `ProtectedCore` 的指纹，产物 `LearningProposal` 绑定该指纹；核心一变，全部旧
//! 提案过期。记录 `LearningRecord` 是观察数据，不是产物：它没有 target 字段。
//!
//! 本 crate **没有错误类型**，也不做持久化：记录与提案只在内存中。
