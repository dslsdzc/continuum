use continuum_trust::ContextProvenance;

/// 一枚候选片段：内容 + 它的来源。`provenance` 由调用方经 P6B 的构造入口给；
/// 本块不构造它（记忆片段除外：`ContextProvenance::memory(claim.id().as_str())`）。
pub struct ContextCandidate<T> {
    pub content: T,
    pub provenance: ContextProvenance,
}
