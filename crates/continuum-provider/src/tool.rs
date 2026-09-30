//! §316 Tool Adapter 的 ToolProvider。

use async_trait::async_trait;
use continuum_core::model::CallId;
use continuum_core::tool::{ToolDescriptor, ToolId, ToolInvocation, ToolResult};
use continuum_core::ProviderError;

#[async_trait]
pub trait ToolProvider: Send + Sync {
    async fn list_tools(&self) -> Result<Vec<ToolDescriptor>, ProviderError>;
    async fn describe_tool(&self, id: &ToolId) -> Result<ToolDescriptor, ProviderError>;
    async fn invoke(&self, call: ToolInvocation) -> Result<ToolResult, ProviderError>;
    async fn cancel(&self, call: &CallId) -> Result<(), ProviderError>;
}
