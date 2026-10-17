//! 一条观察记录。不是产物：它没有 target，指不到任何语义（设计 §5.3）。

use crate::observation::ObservationKind;

/// 一条观察记录。字段 pub：纯数据，没有不变量可挂。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LearningRecord {
    pub kind: ObservationKind,
    /// 被观察对象在它自己那层的标识（不透明串）。
    pub subject: String,
    /// P5a 的 Evidence 的不透明标识。本 crate 不依赖 `continuum-verify`。
    pub evidence_refs: Vec<String>,
}
