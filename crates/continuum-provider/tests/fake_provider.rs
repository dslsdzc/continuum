use async_trait::async_trait;
use continuum_core::connector::{ConnectorDescriptor, ConnectorId, ConnectorOp};
use continuum_core::model::{
    CallId, InvokeRequest, InvokeResponse, ModelDescriptor, ModelId, ModelStream, ProviderHealth,
    StreamChunk, Usage,
};
use continuum_core::tool::{ToolDescriptor, ToolId, ToolInvocation, ToolResult};
use continuum_core::ProviderError;
use continuum_provider::connector::Connector;
use continuum_provider::model::ModelProvider;
use continuum_provider::tool::ToolProvider;
use futures_core::Stream;
use serde_json::json;
use std::pin::Pin;
use std::task::{Context, Poll};

/// 测试用单分片流。`futures-core` 只提供 trait 与类型别名，不提供构造函数，
/// `stream::iter` 属于 `futures-util`。为不引入第五个外部依赖，在此手写。
struct OnceStream(Option<Result<StreamChunk, ProviderError>>);

impl Stream for OnceStream {
    type Item = Result<StreamChunk, ProviderError>;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Ready(self.0.take())
    }
}

struct FakeModel;

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

struct FakeTool;

#[async_trait]
impl ToolProvider for FakeTool {
    async fn list_tools(&self) -> Result<Vec<ToolDescriptor>, ProviderError> {
        Ok(vec![ToolDescriptor {
            id: ToolId::new("echo"),
            description: "回显输入".into(),
            input_schema: json!({"type": "object"}),
        }])
    }

    async fn describe_tool(&self, id: &ToolId) -> Result<ToolDescriptor, ProviderError> {
        self.list_tools()
            .await?
            .into_iter()
            .find(|t| &t.id == id)
            .ok_or_else(|| ProviderError::Unavailable(id.as_str().to_owned()))
    }

    async fn invoke(&self, call: ToolInvocation) -> Result<ToolResult, ProviderError> {
        Ok(ToolResult {
            output: call.input,
            is_error: false,
        })
    }

    async fn cancel(&self, _call: &CallId) -> Result<(), ProviderError> {
        Ok(())
    }
}

struct FakeConnector;

#[async_trait]
impl Connector for FakeConnector {
    fn descriptor(&self) -> ConnectorDescriptor {
        ConnectorDescriptor::new(
            ConnectorId::new("GitHub"),
            vec![ConnectorOp::new("GitHub.push_branch").expect("合法操作标识")],
        )
        .expect("非空操作列表")
    }

    async fn invoke(
        &self,
        op: &ConnectorOp,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, ProviderError> {
        if !self.descriptor().operations().iter().any(|o| o == op) {
            return Err(ProviderError::Protocol(format!(
                "未声明的操作: {}",
                op.as_str()
            )));
        }
        Ok(input)
    }
}

#[tokio::test]
async fn fake_implementations_satisfy_the_frozen_interfaces() {
    let m = FakeModel;
    assert_eq!(m.list_models().await.unwrap().len(), 1);
    assert_eq!(
        m.describe_model(&ModelId::new("fake-1")).await.unwrap().provider,
        "fake"
    );
    assert!(matches!(
        m.describe_model(&ModelId::new("nope")).await,
        Err(ProviderError::UnknownModel(_))
    ));
    assert_eq!(m.health().await, ProviderHealth::Healthy);

    let t = FakeTool;
    assert_eq!(t.list_tools().await.unwrap().len(), 1);

    let c = FakeConnector;
    assert_eq!(c.descriptor().operations().len(), 1);
    assert!(c
        .invoke(
            &ConnectorOp::new("GitHub.merge").expect("合法操作标识"),
            json!({})
        )
        .await
        .is_err());
    assert!(c
        .invoke(
            &ConnectorOp::new("GitHub.push_branch").expect("合法操作标识"),
            json!({"b": "x"})
        )
        .await
        .is_ok());
}

#[tokio::test]
async fn model_stream_is_consumable() {
    let m = FakeModel;
    let stream = m
        .stream(InvokeRequest {
            model: ModelId::new("fake-1"),
            messages: vec![],
            max_tokens: None,
        })
        .await
        .expect("流应可建立");

    // 手动 poll，证明该接口能被消费而不只是能被构造
    let mut chunks = stream.chunks;
    let waker = std::task::Waker::noop();
    let mut cx = std::task::Context::from_waker(waker);

    match chunks.as_mut().poll_next(&mut cx) {
        std::task::Poll::Ready(Some(Ok(chunk))) => {
            assert_eq!(chunk.delta, "pong");
            assert!(chunk.done, "首个分片应标记 done");
        }
        other => panic!("首个分片应为 Ready(Some(Ok(..)))，实际 {other:?}"),
    }

    match chunks.as_mut().poll_next(&mut cx) {
        std::task::Poll::Ready(None) => {}
        other => panic!("流应已结束，实际 {other:?}"),
    }
}
