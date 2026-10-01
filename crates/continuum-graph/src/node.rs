//! ADFIR Node（§236）。

use crate::ids::NodeId;
use continuum_operator::OperatorId;
use continuum_port::PortId;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub operator: OperatorRef,
    pub inputs: Vec<PortId>,
    pub outputs: Vec<PortId>,
    pub constraints: Vec<String>,
    pub capabilities: Vec<String>,
    /// 结构保存，判定属 P4。
    pub execution_policy: Value,
    /// 结构保存，判定属 P5。
    pub verification_policy: Value,
    pub state: NodeState,
}

impl Node {
    pub fn new(id: NodeId, operator: OperatorId, version: continuum_operator::OperatorVersion) -> Self {
        Self {
            id,
            operator: OperatorRef::new(operator, version),
            inputs: Vec::new(),
            outputs: Vec::new(),
            constraints: Vec::new(),
            capabilities: Vec::new(),
            execution_policy: Value::Null,
            verification_policy: Value::Null,
            state: NodeState::Pending,
        }
    }
}

/// §236 的 `operator` 字段：id 与版本共同定位一个 Operator。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperatorRef {
    pub id: OperatorId,
    pub version: continuum_operator::OperatorVersion,
}

impl OperatorRef {
    pub fn new(id: OperatorId, version: continuum_operator::OperatorVersion) -> Self {
        Self { id, version }
    }
}

/// §237 的十三个状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NodeState {
    Pending,
    Ready,
    Queued,
    Running,
    Waiting,
    Blocked,
    Suspended,
    Verifying,
    Completed,
    Failed,
    Cancelled,
    Invalidated,
    Lost,
}
