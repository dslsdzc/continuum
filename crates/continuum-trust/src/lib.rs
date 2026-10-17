//! 本 crate 只装 §281 的**上下文**信任词汇。节点信任域在 `continuum-node`，
//! 工具画像的 `Trust` 在 `continuum-capability`，权威信任域在第 5 层（Authority Host）。
//! 不要把 `NodeTrust` / `PrivacyClass` / `continuum_capability::Trust` 搬进来；
//! 本 crate 对 workspace 内零依赖，这既是命名纪律，也是结构保证。
//!
//! 本 crate 只定义 `ContextPackage` 两个字段的元素类型与形状（`provenance[]` 与
//! `trust_labels[]`），不定义 `ContextPackage` 本身，也不建拒绝、不落库。

pub mod provenance;
pub mod trust;

pub use provenance::{ArtifactProducer, ContextProvenance};
pub use trust::ContextTrust;
