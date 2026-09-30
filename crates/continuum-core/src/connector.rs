//! §124 / §125 Connector 的接口类型。
//!
//! §125 要求按操作细分，不得退化为服务级授权。该约束对三条构造路径
//! 全部生效：`new()`、结构体字面量、以及反序列化。后者靠字段私有
//! 加 `try_from` 回流到 `new()` 实现。

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
///
/// 空串与不含 `.` 的标识一律拒绝：前者不构成操作，
/// 后者只能表达服务级授权。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String")]
pub struct ConnectorOp(String);

impl ConnectorOp {
    pub fn new(op: impl Into<String>) -> Result<Self, CoreError> {
        let op = op.into();
        if op.is_empty() || !op.contains('.') {
            return Err(CoreError::MalformedConnectorOp { op });
        }
        Ok(Self(op))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ConnectorOp {
    type Error = CoreError;

    fn try_from(op: String) -> Result<Self, CoreError> {
        ConnectorOp::new(op)
    }
}

/// 字段私有，`new` 是唯一构造入口。反序列化经 `try_from`
/// 回流到 `new`，因此三条构造路径都受 §125 约束。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "RawConnectorDescriptor")]
pub struct ConnectorDescriptor {
    id: ConnectorId,
    operations: Vec<ConnectorOp>,
}

#[derive(Deserialize)]
struct RawConnectorDescriptor {
    id: ConnectorId,
    operations: Vec<ConnectorOp>,
}

impl TryFrom<RawConnectorDescriptor> for ConnectorDescriptor {
    type Error = CoreError;

    fn try_from(raw: RawConnectorDescriptor) -> Result<Self, CoreError> {
        ConnectorDescriptor::new(raw.id, raw.operations)
    }
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

    pub fn id(&self) -> &ConnectorId {
        &self.id
    }

    pub fn operations(&self) -> &[ConnectorOp] {
        &self.operations
    }
}
