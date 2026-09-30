//! §315 Provider Adapter 的 ModelProvider。
//!
//! 方法集固定为 §315 的清单：list_models / describe_model / invoke / stream / cancel / usage / health。
//! §80 的 discover / capabilities 已裁决为以 §315 为准，见 P0 设计第 7.1 节。

use async_trait::async_trait;
use continuum_core::model::{
    CallId, InvokeRequest, InvokeResponse, ModelDescriptor, ModelId, ModelStream, ProviderHealth,
    Usage,
};
use continuum_core::ProviderError;

#[async_trait]
pub trait ModelProvider: Send + Sync {
    async fn list_models(&self) -> Result<Vec<ModelDescriptor>, ProviderError>;
    async fn describe_model(&self, id: &ModelId) -> Result<ModelDescriptor, ProviderError>;
    async fn invoke(&self, request: InvokeRequest) -> Result<InvokeResponse, ProviderError>;
    async fn stream(&self, request: InvokeRequest) -> Result<ModelStream, ProviderError>;
    async fn cancel(&self, call: &CallId) -> Result<(), ProviderError>;
    async fn usage(&self) -> Result<Usage, ProviderError>;
    async fn health(&self) -> ProviderHealth;
}
