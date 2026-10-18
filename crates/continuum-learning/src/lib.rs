//! Runtime Learning：六类学习目标、五类观察、记录与提案（设计 §5）。
//!
//! 本 crate 只有一条 workspace 依赖边：`continuum-protected`。学习侧读
//! [`ProtectedCore`](continuum_protected::ProtectedCore) 的指纹，产物
//! [`LearningProposal`] 绑定该指纹；核心一变，全部旧提案过期。记录
//! [`LearningRecord`] 是观察数据，不是产物：它没有 target 字段。
//!
//! 六类目标取自 §320 的五类加 §216 的 Scheduling Heuristic；五类观察取自 §216 的
//! 中文原句。两枚枚举的字符串只用于同一 crate 内的互逆与照片，不落库。
//!
//! 本 crate **没有错误类型**，也不做持久化：记录与提案只在内存中。

mod observation;
mod proposal;
mod record;
mod target;

pub use observation::ObservationKind;
pub use proposal::LearningProposal;
pub use record::LearningRecord;
pub use target::LearningTarget;
