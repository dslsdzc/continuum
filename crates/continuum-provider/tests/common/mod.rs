//! `continuum-provider` 各测试目标共用的夹具。
//!
//! 本模块被多个测试目标 `mod common;` include——每个测试目标各编译一份，而各目标只用得上其中
//! 一部分夹具。本仓要求 0 warning，故整模块关掉 `dead_code`：否则「本目标用不到、模块里又
//! 没有别人引用」的夹具会在那一份编译里报出来。
//!
//! **据实记（Task 1 实测）**：Task 1 时点上 `FakeModel` 与 `OnceStream` 都还有人引用
//! （后者经 `FakeModel::stream`），把下面这行删掉跑 `cargo test -p continuum-provider`
//! **不会**出警告。**订正（Task 2 实测，2026-10-06）**：`FakeTool` 搬进来之后那句**变真了**——
//! 把下面这行删掉跑 `cargo build -p continuum-provider --all-targets`，`registry_models` 那一份
//! 编译即报 `warning: struct \`FakeTool\` is never constructed`（日志 `.tmp/t2-dead-code-allow-check.log`）。
//! **同一句绝对措辞在不同 task 的时点上可以一真一假**，故本轮把时点补进这句话本身。
#![allow(dead_code)]

use async_trait::async_trait;
use continuum_core::model::{
    CallId, InvokeRequest, InvokeResponse, ModelDescriptor, ModelId, ModelStream, ProviderHealth,
    StreamChunk, Usage,
};
use continuum_core::tool::{ToolDescriptor, ToolId, ToolResult};
use continuum_core::ProviderError;
use continuum_provider::model::ModelProvider;
use continuum_provider::tool::{AuthorizedToolInvocation, ToolProvider};
use futures_core::Stream;
use serde_json::json;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
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

/// 工具侧夹具（Task 2 从 `tests/fake_provider.rs` 原样搬来）。
///
/// 它声明一个工具 `echo`，`describe_tool` 对声明之外的 id 返 `ProviderError::Unavailable`
/// ——**这正是设计 §5.2 记的那个既有实例**：工具侧没有 `UnknownModel` 那样的「不是我的」取值，
/// 故适配器只能把「无此工具」报成一个瞬时类。注册表要做的正是别让它污染路由层的判据。
pub struct FakeTool;

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

    async fn invoke(
        &self,
        call: AuthorizedToolInvocation<'_>,
    ) -> Result<ToolResult, ProviderError> {
        Ok(ToolResult {
            output: call.input().clone(),
            is_error: false,
        })
    }

    async fn cancel(&self, _call: &continuum_core::model::CallId) -> Result<(), ProviderError> {
        Ok(())
    }
}

/// 记录型适配器交出什么。两格，各有用例：
/// [`Echo`](RecorderOutcome::Echo) 是 `FakeTool` 那条约定的照做（工具跑起来了），
/// [`Fail`](RecorderOutcome::Fail) 是 provider 级失败（工具没跑成）。
pub enum RecorderOutcome {
    /// 回显输入，`is_error: false`。
    Echo,
    /// 返 `Err(ProviderError::Transport(..))`——**适配器自己**没跑成。
    Fail,
}

/// 记录型工具夹具（Task 4）。
///
/// **为什么需要它**：`invoke_tool` 交出的 `ToolResult` 里**没有任何回执字段**可比对
/// （设计 §7.1 的限度：类型层管住「工具 id 从哪来」，管不住「适配器照着它做」），
/// 故「适配器收到的授权是哪一枚」只能由适配器自己在 `invoke` 里抄进一块共享内存，
/// 用例再从观测端读回来。
///
/// 它同时记下**自己被碰过几次**——`list_tools` / `describe_tool` / `invoke` 三者任一
/// 被调到就计一次。「一次都没被碰过」这句断言需要一个看得见的主语：未命中的 id
/// 在注册表里查不到适配器，故路由**应当**在查表那一步就断掉、一个适配器都不必问；
/// 一个「先逐个问适配器、再判未命中」的实现在这条上会红。只数 `invoke` 不够——
/// 那种实现碰的是 `list_tools`，而不是 `invoke`。
///
/// **`cancel` 不计**：`invoke_tool` 不调它，本文件的用例也没有一处会走到它。
/// 把它一并计上便是一句没有用例的断言，故不收（要用时再加，并同时补照片）。
pub struct RecordingTool {
    id: ToolId,
    outcome: RecorderOutcome,
    touches: Arc<AtomicUsize>,
    seen: Arc<Mutex<Option<ToolId>>>,
}

/// 记录型夹具的观测端：与适配器共享同一块内存，用例只读不写。
#[derive(Clone)]
pub struct Recorder {
    touches: Arc<AtomicUsize>,
    seen: Arc<Mutex<Option<ToolId>>>,
}

impl Recorder {
    /// 这个适配器被碰过几次（`list_tools` / `describe_tool` / `invoke` 三者任一被调都算）。
    pub fn touches(&self) -> usize {
        self.touches.load(Ordering::SeqCst)
    }

    /// 最近一次 `invoke` 收到的授权是哪一枚工具；一次都没收到过则是 `None`。
    pub fn seen_tool_id(&self) -> Option<ToolId> {
        self.seen.lock().unwrap().clone()
    }
}

impl RecordingTool {
    /// 造一个服务于 `id` 的适配器，返回它与配套的观测端。
    pub fn new(id: ToolId, outcome: RecorderOutcome) -> (Self, Recorder) {
        let touches = Arc::new(AtomicUsize::new(0));
        let seen = Arc::new(Mutex::new(None));
        let tool = Self {
            id,
            outcome,
            touches: Arc::clone(&touches),
            seen: Arc::clone(&seen),
        };
        (tool, Recorder { touches, seen })
    }
}

#[async_trait]
impl ToolProvider for RecordingTool {
    async fn list_tools(&self) -> Result<Vec<ToolDescriptor>, ProviderError> {
        self.touches.fetch_add(1, Ordering::SeqCst);
        Ok(vec![ToolDescriptor {
            id: self.id.clone(),
            description: "记录收到的授权".into(),
            input_schema: json!({"type": "object"}),
        }])
    }

    async fn describe_tool(&self, id: &ToolId) -> Result<ToolDescriptor, ProviderError> {
        self.touches.fetch_add(1, Ordering::SeqCst);
        if id == &self.id {
            Ok(ToolDescriptor {
                id: self.id.clone(),
                description: "记录收到的授权".into(),
                input_schema: json!({"type": "object"}),
            })
        } else {
            Err(ProviderError::Unavailable(id.as_str().to_owned()))
        }
    }

    async fn invoke(
        &self,
        call: AuthorizedToolInvocation<'_>,
    ) -> Result<ToolResult, ProviderError> {
        self.touches.fetch_add(1, Ordering::SeqCst);
        *self.seen.lock().unwrap() = Some(call.authorization().tool_id().clone());
        match self.outcome {
            RecorderOutcome::Echo => Ok(ToolResult {
                output: call.input().clone(),
                is_error: false,
            }),
            RecorderOutcome::Fail => {
                Err(ProviderError::Transport("适配器自己没跑成".to_owned()))
            }
        }
    }

    async fn cancel(&self, _call: &continuum_core::model::CallId) -> Result<(), ProviderError> {
        Ok(())
    }
}
