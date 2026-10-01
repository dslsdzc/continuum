//! Artifact 数据模型（§240）与双身份（ENG-003）。

use crate::content::ContentHash;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// `§239` 的类型集合。枚举外的类型不可表达。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactType {
    SourceTree,
    Patch,
    TestResult,
    Text,
    Json,
    Blob,
}

/// `§243`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrivacyClass {
    Public,
    Personal,
    Private,
    Secret,
    LocalOnly,
}

/// 图引用与版本身份。稳定，不随内容变化。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ArtifactId(String);

impl ArtifactId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ArtifactId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Artifact {
    pub id: ArtifactId,
    pub artifact_type: ArtifactType,
    pub content_hash: ContentHash,
    pub size: u64,
    pub producer_node: Option<String>,
    pub input_artifacts: Vec<ArtifactId>,
    pub metadata: Value,
    pub provenance: Value,
    pub privacy_class: PrivacyClass,
    pub version: u32,
}
