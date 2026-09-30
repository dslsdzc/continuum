//! §124 Service Connector。
//!
//! §124 未给方法清单，§125 约束授权粒度：按操作细分，不得退化为服务级授权。
//! 因此 trait 没有服务级入口，每次调用必须携带 `ConnectorOp`。

use async_trait::async_trait;
use continuum_core::connector::{ConnectorDescriptor, ConnectorOp};
use continuum_core::ProviderError;
use serde_json::Value;

#[async_trait]
pub trait Connector: Send + Sync {
    fn descriptor(&self) -> ConnectorDescriptor;

    /// `op` 必须是 `descriptor().operations` 中的一项。
    async fn invoke(&self, op: &ConnectorOp, input: Value) -> Result<Value, ProviderError>;
}
