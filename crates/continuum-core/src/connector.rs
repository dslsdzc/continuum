//! §124 / §125 Connector 的接口类型。
//!
//! §125 要求按操作细分，不得退化为服务级授权。
//! 该约束由 `ConnectorDescriptor::new` 拒绝空操作列表在类型层表达。

use crate::error::CoreError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConnectorId(String);

impl ConnectorId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// 操作标识，形如 `GitHub.push_branch`（§125）。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConnectorOp(String);

impl ConnectorOp {
    pub fn new(op: impl Into<String>) -> Self {
        Self(op.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectorDescriptor {
    pub id: ConnectorId,
    pub operations: Vec<ConnectorOp>,
}

impl ConnectorDescriptor {
    /// 操作列表为空时返回 `CoreError::ConnectorWithoutOperations`。
    pub fn new(id: ConnectorId, operations: Vec<ConnectorOp>) -> Result<Self, CoreError> {
        if operations.is_empty() {
            return Err(CoreError::ConnectorWithoutOperations {
                connector: id.as_str().to_owned(),
            });
        }
        Ok(Self { id, operations })
    }
}
