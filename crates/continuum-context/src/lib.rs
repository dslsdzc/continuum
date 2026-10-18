//! P6A Context Compiler：§280 的 `ContextPackage` 与装配入口 `compile`。
//! 本 crate 不建执行器、不持久化；标签类型来自 `continuum-trust`（P6B）。

mod package;
mod reference;

use continuum_artifact::ArtifactId;
use continuum_memory::MemoryClaim;
use continuum_trust::{ContextProvenance, ContextTrust};

pub use reference::Reference;

/// §280 的六个字段。字段名与书写序照规范逐字，一个不改、一个不加。
/// 字段私有：唯一的构造点是 `crate::compile`（设计 §4.3）。
/// 不实现 `Default` / `Deserialize`（那会造出第二个构造点）。
#[derive(Debug, Clone, PartialEq)]
pub struct ContextPackage {
    instructions: Vec<String>,
    references: Vec<Reference>,
    artifacts: Vec<ArtifactId>,
    memories: Vec<MemoryClaim>,

    provenance: Vec<ContextProvenance>,
    trust_labels: Vec<ContextTrust>,
}
