//! Artifact 数据模型、内容寻址与 lineage。P1 执行层的数据底座。

pub mod artifact;
pub mod content;
pub mod store;

pub use artifact::{Artifact, ArtifactId, ArtifactType, PrivacyClass};
pub use content::ContentHash;
pub use store::{ArtifactError, ArtifactStore};
