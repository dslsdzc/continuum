//! `TrustError`：本 crate 唯一的错误类型。

use crate::trust::ContextTrust;

/// 包级标签校验的失败。**不包装别的错误**：本 crate 零内部依赖，没有可包装的上游。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TrustError {
    /// `provenance[]` 与 `trust_labels[]` 长度不等。
    #[error("provenance 有 {provenance} 条，trust_labels 有 {trust_labels} 条")]
    LengthMismatch { provenance: usize, trust_labels: usize },

    /// `provenance[]` 的长度不等于四个内容数组长度之和。
    #[error("包内应有 {expected} 个片段，provenance 有 {actual} 条")]
    ShapeMismatch { expected: usize, actual: usize },

    /// 第 `index` 个片段（**全局片段下标**，与 `provenance[]` 同序）的标签不等于
    /// `label(provenance[index])`。
    #[error("第 {index} 个片段的标签应为 {expected}，实际是 {actual}")]
    LabelDoesNotMatchProvenance {
        index: usize,
        expected: ContextTrust,
        actual: ContextTrust,
    },

    /// `instructions[]` 里的第 `index` 个片段带了 data-only 标签。
    /// `index` 是**全局片段下标**（与 `provenance[]` 同序，不是数组内下标）。
    #[error("第 {index} 个片段带 data-only 标签 {label}，不得进入 instructions[]")]
    InstructionSlotCarriesDataOnlyLabel { index: usize, label: ContextTrust },

    /// `instructions[]` 里的第 `index` 个片段是**历史**用户轮（`historical: true`）。
    /// 历史只能进 `references[]`；把它放进指令位等于把旧话当新指令。
    /// `index` 是全局片段下标。
    #[error("第 {index} 个片段是历史用户轮，不得进入 instructions[]")]
    HistoricalInstructionInInstructions { index: usize },

    /// 三个数据数组里的第 `index` 个片段带了 `USER_INSTRUCTION`。
    /// **`references[]` 只放行 `historical: true` 的历史用户轮**；`artifacts[]` /
    /// `memories[]` 一律拒绝，当前指令（`historical: false`）在任何数据数组都拒绝。
    /// `index` 是全局片段下标。
    #[error("第 {index} 个片段带 USER_INSTRUCTION（historical={historical}），不得进入该数据数组")]
    DataSlotCarriesUserInstruction { index: usize, historical: bool },
}
