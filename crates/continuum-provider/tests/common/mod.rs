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

/// `FakeTool` 的三种 `invoke` 形态（Task 7 加后两种）。
///
/// **为什么把 `FakeTool` 三态化**：设计 §11 那一行写的是「用 `FakeTool`」同时验工具级失败与
/// provider 级失败两条，而一个单元结构体产不出两条失败通道。三态化是让那一行字面成立的
/// **最小**改动。**这是本计划对夹具形状的读数，不是设计给的判据**——设计没有规定夹具长什么样
/// （计划 Task 7 Step 1；设计 §11）。
enum FakeToolOutcome {
    /// 回显输入，`is_error: false`（今天的、也是 `fake_provider.rs` 用到的那一种）。
    Echo,
    /// 工具**跑起来了但自身失败**：`Ok(ToolResult { is_error: true, .. })`。
    ToolLevel,
    /// **适配器自己**没跑成：`Err(ProviderError::Transport(..))`。
    ProviderLevel,
}

/// 工具侧夹具（Task 2 从 `tests/fake_provider.rs` 原样搬来；Task 7 加失败通道）。
///
/// 它声明一个工具 `echo`，`describe_tool` 对声明之外的 id 返 `ProviderError::Unavailable`
/// ——**这正是设计 §5.2 记的那个既有实例**：工具侧没有 `UnknownModel` 那样的「不是我的」取值，
/// 故适配器只能把「无此工具」报成一个瞬时类。注册表要做的正是别让它污染路由层的判据。
///
/// **两条失败通道承载的是 C 加在适配器上的约定**（设计 §7.1）：工具级失败走
/// `Ok(ToolResult { is_error: true, .. })`、provider 级失败走 `Err`。**这条约定 §316 里没有
/// 出处、也没有任何强制**——真实适配器完全可以对工具级失败返 `Err(Protocol)`；用本夹具写的
/// 用例钉的**只是这份夹具对约定的服从，不是真实适配器**，而真实适配器上这条分流**拍不到**
/// （设计 §9）。照片在 `tests/contract.rs`。
///
/// **本夹具与 F 的库级夹具是同一约定的两份副本，裁决明写「不合并」**
/// （`docs/superpowers/p3bcdf-followups.md` §七 第 8 条）：跨 crate 的 `tests/` 目录不可互相导入，
/// F 的库级用例装进本注册表时只能另写一份（它按本文件的形状写）。代价据实记：**两者若漂移，
/// 「工具级失败走 `Ok(is_error: true)`」这条约定只在本 crate 的用例上红**，F 那份不会；
/// 这条不设护栏，靠评审。
pub struct FakeTool {
    outcome: FakeToolOutcome,
}

impl FakeTool {
    /// 今天的行为：回显输入、`is_error: false`。
    pub fn echo() -> Self {
        Self {
            outcome: FakeToolOutcome::Echo,
        }
    }

    /// 工具**跑起来了但自身失败**——按约定报 `Ok(ToolResult { is_error: true, .. })`。
    pub fn failing_at_tool_level() -> Self {
        Self {
            outcome: FakeToolOutcome::ToolLevel,
        }
    }

    /// **适配器自己**没跑成——按约定报 `Err(ProviderError::Transport(..))`。
    pub fn failing_at_provider_level() -> Self {
        Self {
            outcome: FakeToolOutcome::ProviderLevel,
        }
    }
}

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

    /// 三条通道各按 C 的约定交出结果（设计 §7.1）。**这是夹具对约定的服从**，
    /// 不是「适配器都这么干」——真实适配器上这条分流拍不到（设计 §9）。
    ///
    /// 工具级那条的 `output` 取一个固定的、与输入**不同**的值：若它回显输入，`output` 在
    /// `Echo` 与 `ToolLevel` 两臂就**完全相同**，`is_error` 于是**成了**两者唯一的区分者
    /// ——而设计从没规定工具级失败的 `output` 该是什么（计划 Step 1 也只写 `is_error: true`），
    /// 回显会把这条**本计划自选**的载荷说成必须的。
    ///
    /// **订正（评审 2026-10-06）**：本行初稿写的是「若它回显输入，`is_error` 就**成不了**…
    /// 的唯一载荷」，**理由写反了**——回显恰恰让 `is_error` 成为唯一区分者；**做法（选固定
    /// 非回显值）是对的，写反的是理由**。错误说法的来历留在原地备查。
    async fn invoke(
        &self,
        call: AuthorizedToolInvocation<'_>,
    ) -> Result<ToolResult, ProviderError> {
        match self.outcome {
            FakeToolOutcome::Echo => Ok(ToolResult {
                output: call.input().clone(),
                is_error: false,
            }),
            FakeToolOutcome::ToolLevel => Ok(ToolResult {
                output: json!({"error": "工具跑起来了但自身失败"}),
                is_error: true,
            }),
            FakeToolOutcome::ProviderLevel => {
                Err(ProviderError::Transport("适配器自己没跑成".to_owned()))
            }
        }
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

    /// 声明之外的 id 返 `ProviderError::Unavailable`——**同 `FakeTool`，来由见设计 §5.2**：
    /// 工具侧没有 `UnknownModel` 那样的「不是我的」取值，故只能把「无此工具」报成一个
    /// 瞬时类。**这是夹具的便利（照抄既有夹具的形状），不是被推荐的行为样例**——
    /// 注册表要做的正是别让这种报法污染路由层的判据（`src/registry.rs` 的
    /// `ToolCallError` 文档把这条记成历史缺陷／反模式）。本文件的用例都不走这一支。
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
