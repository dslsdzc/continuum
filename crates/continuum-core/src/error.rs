//! Provider 边界上的错误类型。core 不引入 I/O，错误只承载信息。

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProviderError {
    #[error("provider 不可用: {0}")]
    Unavailable(String),
    #[error("模型不存在: {0}")]
    UnknownModel(String),
    #[error("调用已取消: {0}")]
    Cancelled(String),
    #[error("传输失败: {0}")]
    Transport(String),
    #[error("协议错误: {0}")]
    Protocol(String),
}

/// Connector 描述符的构造错误。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CoreError {
    #[error("Connector {connector} 未声明任何操作，退回服务级授权（§125）")]
    ConnectorWithoutOperations { connector: String },
    #[error("Connector 操作标识 {op:?} 格式非法，要求形如 GitHub.push_branch（§125）")]
    MalformedConnectorOp { op: String },
}
