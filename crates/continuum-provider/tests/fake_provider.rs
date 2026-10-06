mod common;

use async_trait::async_trait;
use common::FakeModel;
use continuum_core::connector::{ConnectorDescriptor, ConnectorId, ConnectorOp};
use continuum_core::model::{CallId, InvokeRequest, ModelId, ProviderHealth};
use continuum_core::tool::{ToolDescriptor, ToolId, ToolInvocation, ToolResult};
use continuum_core::ProviderError;
use continuum_provider::connector::Connector;
use continuum_provider::model::ModelProvider;
use continuum_provider::tool::ToolProvider;
use serde_json::json;

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
