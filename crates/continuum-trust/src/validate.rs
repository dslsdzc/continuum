//! `PackageShape` 与 `validate()`：包级标签的独立复核面（I1–I5）。

use crate::error::TrustError;
use crate::label::label;
use crate::provenance::ContextProvenance;
use crate::trust::ContextTrust;

/// `ContextPackage` 四个内容数组的长度视图。**它不是 `ContextPackage`**（后者归 P6A）；
/// 它只把「包里有几段、各在哪个数组」交给本 crate 的校验。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PackageShape {
    pub instructions: usize,
    pub references: usize,
    pub artifacts: usize,
    pub memories: usize,
}

impl PackageShape {
    /// 四个内容数组长度之和，即 `provenance[]` / `trust_labels[]` 应有的条数。
    pub fn total(&self) -> usize {
        self.instructions + self.references + self.artifacts + self.memories
    }
}

/// 交出包之前的**独立复核面**。片段序列＝ `instructions[]` → `references[]` →
/// `artifacts[]` → `memories[]`。检查次序（每一步失败立即返回）：
///
/// 1. `provenance.len() == trust_labels.len()` → [`TrustError::LengthMismatch`]
/// 2. `provenance.len() == shape.total()` → [`TrustError::ShapeMismatch`]
/// 3. 每个 `i`：`trust_labels[i] == label(&provenance[i])` →
///    [`TrustError::LabelDoesNotMatchProvenance`]
/// 4. `instructions[]` 的每个片段：标签满足 `may_sit_in_instructions()`，且 provenance
///    不是 `UserInstruction { historical: true }`
/// 5. `references[]` 的每个片段：`USER_INSTRUCTION` 仅当 provenance 是
///    `UserInstruction { historical: true }`；`artifacts[]` / `memories[]` 的每个片段：
///    `USER_INSTRUCTION` 一律拒绝
///
/// **它不是拒绝函数**：它判包内自洽，不判「某段内容事实上从哪来」。一个把网页声明成
/// `user_instruction()` 的包可以通过校验。`index` 一律是**全局片段下标**（与
/// `provenance[]` 同序），不是数组内下标。本函数不接收内容字符串，也不调用任何 I/O。
pub fn validate(
    shape: PackageShape,
    provenance: &[ContextProvenance],
    trust_labels: &[ContextTrust],
) -> Result<(), TrustError> {
    // I1：等长。
    if provenance.len() != trust_labels.len() {
        return Err(TrustError::LengthMismatch {
            provenance: provenance.len(),
            trust_labels: trust_labels.len(),
        });
    }

    // I2：覆盖四个内容数组。
    if provenance.len() != shape.total() {
        return Err(TrustError::ShapeMismatch {
            expected: shape.total(),
            actual: provenance.len(),
        });
    }

    // I3：逐位映射。每个片段只调一次 label()，错误值与比较共用同一个局部变量。
    for i in 0..provenance.len() {
        let expected = label(&provenance[i]);
        if trust_labels[i] != expected {
            return Err(TrustError::LabelDoesNotMatchProvenance {
                index: i,
                expected,
                actual: trust_labels[i],
            });
        }
    }

    // I4：指令位只收当前指令（含 TRUSTED_RUNTIME 与历史轮的两条独立检查）。
    for i in 0..shape.instructions {
        if !trust_labels[i].may_sit_in_instructions() {
            return Err(TrustError::InstructionSlotCarriesDataOnlyLabel {
                index: i,
                label: trust_labels[i],
            });
        }
        if matches!(
            &provenance[i],
            ContextProvenance::UserInstruction { historical: true }
        ) {
            return Err(TrustError::HistoricalInstructionInInstructions { index: i });
        }
    }

    // I5：references[] 只放行历史用户轮；artifacts[] / memories[] 一律拒绝。
    let references_start = shape.instructions;
    let references_end = references_start + shape.references;
    for idx in references_start..references_end {
        let is_historical = matches!(
            &provenance[idx],
            ContextProvenance::UserInstruction { historical: true }
        );
        if trust_labels[idx].is_user_instruction() && !is_historical {
            return Err(TrustError::DataSlotCarriesUserInstruction {
                index: idx,
                historical: is_historical,
            });
        }
    }

    let artifacts_start = references_end;
    let artifacts_end = artifacts_start + shape.artifacts;
    let memories_end = artifacts_end + shape.memories;

    for idx in artifacts_start..memories_end {
        if trust_labels[idx].is_user_instruction() {
            let historical = matches!(
                &provenance[idx],
                ContextProvenance::UserInstruction { historical: true }
            );
            return Err(TrustError::DataSlotCarriesUserInstruction {
                index: idx,
                historical,
            });
        }
    }

    Ok(())
}
