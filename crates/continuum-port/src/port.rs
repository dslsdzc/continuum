//! Port 定义与连接兼容性判定（§239）。

use continuum_artifact::ArtifactType;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PortId(String);

impl PortId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// `PortError` 的两条格式串以 `{from}` / `{to}` 引用 PortId，
// thiserror 要求该字段实现 Display。
impl std::fmt::Display for PortId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Input,
    Output,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Port {
    id: PortId,
    direction: Direction,
    name: String,
    artifact_type: ArtifactType,
}

impl Port {
    pub fn new(
        id: PortId,
        direction: Direction,
        name: impl Into<String>,
        artifact_type: ArtifactType,
    ) -> Self {
        Self {
            id,
            direction,
            name: name.into(),
            artifact_type,
        }
    }

    pub fn id(&self) -> &PortId {
        &self.id
    }

    pub fn direction(&self) -> Direction {
        self.direction
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn artifact_type(&self) -> ArtifactType {
        self.artifact_type
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PortError {
    #[error("端口 {from} 与 {to} 类型不同：{from_type:?} 与 {to_type:?}")]
    TypeMismatch {
        from: PortId,
        from_type: ArtifactType,
        to: PortId,
        to_type: ArtifactType,
    },
    #[error("端口 {from} 与 {to} 方向相同，连接必须一进一出")]
    DirectionMismatch { from: PortId, to: PortId },
}

/// 判定两个端口能否连接。
///
/// 条件为方向相反且 `artifact_type` 相同（§239）。
/// `from` 为输出端，`to` 为输入端——顺序不影响判定结果，但错误信息按此顺序给出。
pub fn compatible(from: &Port, to: &Port) -> Result<(), PortError> {
    if from.direction == to.direction {
        return Err(PortError::DirectionMismatch {
            from: from.id.clone(),
            to: to.id.clone(),
        });
    }
    if from.artifact_type != to.artifact_type {
        return Err(PortError::TypeMismatch {
            from: from.id.clone(),
            from_type: from.artifact_type,
            to: to.id.clone(),
            to_type: to.artifact_type,
        });
    }
    Ok(())
}
