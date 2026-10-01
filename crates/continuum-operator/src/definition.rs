//! Operator 定义（§244）。

use continuum_artifact::ArtifactType;
use serde::{Deserialize, Serialize};

/// ENG-002 的判定依据：只有 `Deterministic` 的 Operator 参与按输入哈希复用。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Determinism {
    Deterministic,
    NonDeterministic,
}

/// §307 的判定依据：`NonIdempotent` 不进入自动重试。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SideEffectClass {
    Pure,
    Idempotent,
    NonIdempotent,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OperatorId(String);

impl OperatorId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// `OperatorError` 的格式串以 `{id}` 引用 OperatorId，
// thiserror 要求该字段实现 Display。
impl std::fmt::Display for OperatorId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OperatorVersion(u32);

impl OperatorVersion {
    pub fn new(version: u32) -> Self {
        Self(version)
    }

    pub fn as_u32(&self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BackendId(String);

impl BackendId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Operator {
    pub id: OperatorId,
    pub version: OperatorVersion,
    pub input_schema: Vec<ArtifactType>,
    pub output_schema: Vec<ArtifactType>,
    pub determinism: Determinism,
    pub side_effect_class: SideEffectClass,
    /// §245：具体后端由 Router 决定，本层只保存候选。
    pub backend_candidates: Vec<BackendId>,
}

/// §308 要求长任务 Operator 支持 checkpoint。本子项目只定义接口。
/// 未实现该接口的 Operator 不参与检查点。
pub trait Checkpointable {
    type Checkpoint: Send + Sync;

    fn checkpoint(&self) -> Result<Self::Checkpoint, String>;
    fn restore(&self, checkpoint: &Self::Checkpoint) -> Result<(), String>;
}
