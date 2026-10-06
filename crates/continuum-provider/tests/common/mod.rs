//! `continuum-provider` 各测试目标共用的夹具。
//!
//! 本模块被多个测试目标 `mod common;` include——每个测试目标各编译一份，而各目标只用得上其中
//! 一部分夹具。本仓要求 0 warning，故整模块关掉 `dead_code`：否则「本目标用不到、模块里又
//! 没有别人引用」的夹具会在那一份编译里报出来。
//!
//! **据实记（本轮实测，见 task 报告的 m0 一轮）**：Task 1 时点上 `FakeModel` 与 `OnceStream`
//! 都还有人引用（后者经 `FakeModel::stream`），把下面这行删掉跑
//! `cargo test -p continuum-provider` **不会**出警告。它提前关掉，是因为本模块的用法就是被多个
//! 目标共用，后续 task 搬进来的夹具（工具侧的 `FakeTool`）不会每个目标都用得上。
#![allow(dead_code)]

use async_trait::async_trait;
use continuum_core::model::{
    CallId, InvokeRequest, InvokeResponse, ModelDescriptor, ModelId, ModelStream, ProviderHealth,
    StreamChunk, Usage,
};
use continuum_core::ProviderError;
use continuum_provider::model::ModelProvider;
use futures_core::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

/// 测试用单分片流。`futures-core` 只提供 trait 与类型别名，不提供构造函数，
/// `stream::iter` 属于 `futures-util`。为不引入第五个外部依赖，在此手写。
pub struct OnceStream(Option<Result<StreamChunk, ProviderError>>);

impl Stream for OnceStream {
    type Item = Result<StreamChunk, ProviderError>;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Ready(self.0.take())
    }
}

pub struct FakeModel;

#[async_trait]
impl ModelProvider for FakeModel {
    async fn list_models(&self) -> Result<Vec<ModelDescriptor>, ProviderError> {
        Ok(vec![ModelDescriptor {
            id: ModelId::new("fake-1"),
            provider: "fake".into(),
            display_name: "Fake One".into(),
            context_window: 8192,
            capabilities: vec!["text".into()],
        }])
    }

    async fn describe_model(&self, id: &ModelId) -> Result<ModelDescriptor, ProviderError> {
        let all = self.list_models().await?;
        all.into_iter()
            .find(|m| &m.id == id)
            .ok_or_else(|| ProviderError::UnknownModel(id.as_str().to_owned()))
    }

    async fn invoke(&self, request: InvokeRequest) -> Result<InvokeResponse, ProviderError> {
        Ok(InvokeResponse {
            model: request.model,
            content: "pong".into(),
            usage: Usage {
                input_tokens: 1,
                output_tokens: 1,
            },
        })
    }

    async fn stream(&self, _request: InvokeRequest) -> Result<ModelStream, ProviderError> {
        let chunks: Pin<Box<dyn Stream<Item = Result<StreamChunk, ProviderError>> + Send>> =
            Box::pin(OnceStream(Some(Ok(StreamChunk {
                delta: "pong".into(),
                done: true,
            }))));
        Ok(ModelStream {
            call: CallId::new("call-1"),
            chunks,
        })
    }

    async fn cancel(&self, _call: &CallId) -> Result<(), ProviderError> {
        Ok(())
    }

    async fn usage(&self) -> Result<Usage, ProviderError> {
        Ok(Usage {
            input_tokens: 0,
            output_tokens: 0,
        })
    }

    async fn health(&self) -> ProviderHealth {
        ProviderHealth::Healthy
    }
}
