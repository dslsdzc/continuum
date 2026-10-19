use continuum_artifact::ArtifactId;
use continuum_memory::MemoryScope;

use crate::{ContextCandidate, Reference};

/// §279 五类来源的候选集合与记忆检索的三个控制位。
/// 按值消费：`compile` 可以把内容移进产物，不需要 `Clone`。
pub struct ContextRequest {
    pub instructions: Vec<ContextCandidate<String>>,
    pub references: Vec<ContextCandidate<Reference>>,
    pub artifacts: Vec<ContextCandidate<ArtifactId>>,
    pub scopes: Vec<MemoryScope>,
    pub anchor: Option<String>,
    pub memory_query: Option<String>,
}

/// 三个独立的条数上限。**不是 token 预算**（设计 §2.4 第 5 条）。
pub struct ContextBudget {
    pub max_references: usize,
    pub max_artifacts: usize,
    pub max_memories: usize,
}
