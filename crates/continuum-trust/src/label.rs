//! `label()`：唯一的标签投影。

use crate::provenance::{ArtifactProducer, ContextProvenance};
use crate::trust::ContextTrust;

/// 把一枚来源描述投影成 §281 的标签。**本函数是全仓唯一的标签产生点**：
/// P6A 的 `compile` 在每个片段上调用它一次。
///
/// 映射是**全函数、确定性、无 I/O**——同一枚 `ContextProvenance` 永远给同一枚标签。
/// 加一个 `ContextProvenance` 变体时本函数编译不过（穷尽 `match`），实现者必须
/// 重新回答「它属于哪一枚标签」。
pub fn label(provenance: &ContextProvenance) -> ContextTrust {
    match provenance {
        ContextProvenance::UserInstruction { .. } => ContextTrust::UserInstruction,
        ContextProvenance::TrustedRuntime => ContextTrust::TrustedRuntime,
        ContextProvenance::ProjectData => ContextTrust::ProjectData,
        ContextProvenance::WebUntrusted => ContextTrust::WebUntrusted,
        ContextProvenance::ToolOutput { external: true } => ContextTrust::WebUntrusted,
        ContextProvenance::ToolOutput { external: false } => ContextTrust::ToolOutput,
        ContextProvenance::Memory { .. } => ContextTrust::Memory,
        ContextProvenance::ModelOutput => ContextTrust::ModelOutput,
        ContextProvenance::Artifact { producer } => match producer {
            ArtifactProducer::Model => ContextTrust::ModelOutput,
            ArtifactProducer::Tool => ContextTrust::ToolOutput,
            ArtifactProducer::External => ContextTrust::WebUntrusted,
            ArtifactProducer::Project | ArtifactProducer::Unknown => ContextTrust::ProjectData,
        },
    }
}
