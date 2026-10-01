//! Artifact 数据模型、内容寻址与 lineage。P1 执行层的数据底座。

pub mod artifact;
pub mod content;
pub mod persist;
pub mod store;

pub use artifact::{Artifact, ArtifactId, ArtifactType, PrivacyClass};
pub use content::ContentHash;
pub use persist::{load_artifact, p1_artifact_migrations, save_artifact};
pub use store::{ArtifactError, ArtifactStore};
