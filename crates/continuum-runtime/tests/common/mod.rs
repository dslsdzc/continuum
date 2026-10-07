//! `continuum-runtime` 各测试目标共用的夹具：一枚**可配的假模型适配器**。
//!
//! # 这是 C 那份 `FakeModel` 的第二份副本，两份**不合并**
//!
//! `crates/continuum-provider/tests/common/mod.rs` 里已经有一枚 `FakeModel`（供 C 的用例用）。
//! 本文件是它的**第二份**，**不合并**（`docs/superpowers/p3bcdf-followups.md` §七.8：
//! 「同一约定的两份副本，**不合并**，两份各有其用」）。判据三条：
//!
//! 1. **`continuum-runtime` 的测试引用不到 `continuum-provider` 的 `tests/` 下的任何东西**——
//!    集成测试各是一个独立 crate，`tests/` 不是被依赖方导出的面；
//! 2. 把夹具提取成一个共享 crate 会造出一条**为测试而生**的依赖边，而那条边要进
//!    `tests/dependency_direction.rs` 的 `ALLOWED` 表——**代价大于收益**；
//! 3. **两份的用途本来就不同**：C 那份服务于工具侧与「适配器出不了 crate」的编译期照片，
//!    本份服务于 G 的模型调用路径（可配的健康度、四段流程的调用计数、预算搬运的观测端）。
//!
//! **漂移的代价据实记**：两处若在「同一个约定」上分叉，**只会在各自 crate 的用例上红**，
//! 另一份不会——**这条不设护栏，靠评审**。
//!
//! # 本模块被哪些测试目标 include
//!
//! `tests/` 下的每个文件各编一个独立 crate，本模块由 `mod common;` 逐个 include，
//! 每个目标各编译一份，而**各目标只用得上其中一部分**。本仓要求 0 warning，故整模块关掉
//! `dead_code`：否则「本目标用不到、模块里又没有别人引用」的夹具会在那一份编译里报出来。
//!
//! # 可配面里哪些今天还没有照片，各由谁消费
//!
//! `FakeModel` 的可配面是**一次做齐**的（计划 Task 2 Step 4：「逐项都要有，后续 task 依赖它们」），
//! 故有几项在 Task 2 的三条用例里拍不到。**逐项写明收件人，免得被读成漏项**：
//!
//! | 可配项 | Task 2 里有照片吗 | 收件人 |
//! |---|---|---|
//! | `health()` 的返回值 | 有 | — |
//! | 六个方法＋`health` 的计数器 | 有 | — |
//! | `invoke` / `stream` / `cancel` / `list_models` / `describe_model` / `usage` 的可配结果 | 部分（只拍了默认态） | Task 3 / 6 / 8 / 9 / 10 |
//! | [`InvokeOutcome::Never`] | 无 | Task 8 的 `a_call_that_never_returns_hits_the_deadline` 向一 |
//! | [`InvokeOutcome::ReplyAfter`] | 无 | 同上，向二 |
//! | `invoke` / `stream` 收到的 `InvokeRequest` 记录 | 无 | Task 8 / 9（「适配器收到的是哪个模型的请求」） |
//! | `cancel` 收到的 `CallId` 记录 | 无 | Task 9 的 `aborting_a_stream_calls_cancel_with_the_streams_own_call_id` |
//! | `list_models` 能给出「不在 D 的表里的 id」 | 无 | Task 10 的 `the_path_does_not_ask_the_adapter_what_models_it_serves` |
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
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::task::{Context, Poll};
use std::time::Duration;

/// 测试用单分片流。`futures-core` 只提供 trait 与类型别名，不提供构造函数，
/// `stream::iter` 属于 `futures-util`。为不引入第五个外部依赖，在此手写。
///
/// 形状照 C 那份同名类型（`crates/continuum-provider/tests/common/mod.rs:33-41`）——
/// **两份副本的第一处**，见文件头的取舍。
pub struct OnceStream(Option<Result<StreamChunk, ProviderError>>);

impl Stream for OnceStream {
    type Item = Result<StreamChunk, ProviderError>;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Ready(self.0.take())
    }
}

/// 「要么交出这一个值、要么交出一枚错误」的可配面。
///
/// `T` 要 `Clone`：夹具会被同一个用例调多次，每次都得交出一份**新的**值
/// （`ProviderError` 是 `Clone` 的，故错误一侧不必额外处理）。
///
/// `ModelStream` **不在**其中：它装着装箱的 `Stream`，不 `Clone`，
/// 故 `stream` 另有一枚形态 [`StreamOutcome`]。
pub enum Answer<T> {
    /// 交出这一枚值。
    Ok(T),
    /// 交出这一枚错误，一次调用一枚（每枚都是入参的克隆）。
    Fail(ProviderError),
}

/// `invoke` 被调时的行为。
pub enum InvokeOutcome {
    /// 立刻回这一枚响应。
    Reply(InvokeResponse),
    /// 立刻返这一枚错误。
    Fail(ProviderError),
    /// **永不返回**：交出的 future 永远 `Pending`（`std::future::pending`）。
    ///
    /// 它**不阻塞线程**——只是不产出。故一个忘记加截止的调用方会**挂住**，
    /// 这正是 Task 8 向一要拍的那件事（那一条要配合 `timeout` 命令跑）。
    Never,
    /// **先睡一段真时间**再回这一枚响应。
    ///
    /// 「**略早于截止返回**」那一向靠它（Task 8 的向二）。睡多久由用例给——
    /// 夹具**不知道截止是多少**，`ModelProvider::invoke` 的签名里没有它，夹具也不该发明一个。
    ReplyAfter(Duration, InvokeResponse),
}

/// `stream` 交出的那一枚流。
pub enum StreamOutcome {
    /// 交出一个单分片流：`call` 是这一枚流自己的 [`CallId`]，`chunk` 是它唯一的那个分片。
    ///
    /// `call` 可配是**承重的**：Task 9 要断言 G 交回的那枚 `CallId` **就是适配器给的那一个**
    /// （G 不新造、不改写），故它不能是一个夹具内部的常量。
    Chunks {
        call: CallId,
        chunk: Result<StreamChunk, ProviderError>,
    },
    /// 不建立流，直接返错。
    Fail(ProviderError),
}

/// 一枚描述符：`id` 只填 `id` 与 `display_name`，其余取固定值。
///
/// Task 10 要的「**不在 D 的表里的 id**」由调用方传进来即是（本函数不查库，
/// 它不知道 D 的表里有什么——那句话的判据在用例那边）。
pub fn descriptor(id: &str) -> ModelDescriptor {
    ModelDescriptor {
        id: ModelId::new(id),
        provider: "fake".into(),
        display_name: format!("Fake {id}"),
        context_window: 8192,
        capabilities: vec!["text".into()],
    }
}

/// 可配的假模型适配器。
///
/// **观测端就在自己身上**（计数器与记录都是本结构体的字段，配 `pub fn *_calls` / `*_requests`
/// 之类的读口），不像 C 的 `RecordingTool` 那样另立一枚 `Recorder`：本夹具的用例持有
/// `Arc<FakeModel>` 本体，把它转成 `Arc<dyn ModelProvider>` 交出去即可，
/// 观测端与适配器**是同一个对象的两个面**，没有第二份状态要同步。
///
/// **六条方法各有一个计数器，另加 `health` 一个（共七个）**：计划 Task 2 Step 4 的清单里
/// 没有 `health`，而 Task 3 的 `one_probe_per_candidate` 要读它（「探活三次」）。
pub struct FakeModel {
    health: ProviderHealth,
    invoke: InvokeOutcome,
    stream: StreamOutcome,
    cancel: Answer<()>,
    list_models: Answer<Vec<ModelDescriptor>>,
    describe_model: Answer<ModelDescriptor>,
    usage: Answer<Usage>,

    health_calls: AtomicUsize,
    invoke_calls: AtomicUsize,
    stream_calls: AtomicUsize,
    cancel_calls: AtomicUsize,
    list_models_calls: AtomicUsize,
    describe_model_calls: AtomicUsize,
    usage_calls: AtomicUsize,

    invoke_requests: Mutex<Vec<InvokeRequest>>,
    stream_requests: Mutex<Vec<InvokeRequest>>,
    cancelled: Mutex<Vec<CallId>>,
}

impl FakeModel {
    /// 全默认：`Healthy`、`invoke` 立刻回一句、`stream` 交一枚单分片流（`CallId` 为 `call-1`）、
    /// `cancel` 成功、`list_models` 一枚 `fake-1`、`describe_model` 与 `usage` 成功。
    ///
    /// 默认态刻意「什么都不出错」：用例要判的**否定命题**（「没调 `usage()`」之类）
    /// 与「某一方法返错」是两件事，默认出错会让前者的红变得难以归因。
    pub fn new() -> Self {
        Self {
            health: ProviderHealth::Healthy,
            invoke: InvokeOutcome::Reply(InvokeResponse {
                model: ModelId::new("fake-1"),
                content: "pong".into(),
                usage: Usage {
                    input_tokens: 1,
                    output_tokens: 1,
                },
            }),
            stream: StreamOutcome::Chunks {
                call: CallId::new("call-1"),
                chunk: Ok(StreamChunk {
                    delta: "pong".into(),
                    done: true,
                }),
            },
            cancel: Answer::Ok(()),
            list_models: Answer::Ok(vec![descriptor("fake-1")]),
            describe_model: Answer::Ok(descriptor("fake-1")),
            usage: Answer::Ok(Usage {
                input_tokens: 0,
                output_tokens: 0,
            }),

            health_calls: AtomicUsize::new(0),
            invoke_calls: AtomicUsize::new(0),
            stream_calls: AtomicUsize::new(0),
            cancel_calls: AtomicUsize::new(0),
            list_models_calls: AtomicUsize::new(0),
            describe_model_calls: AtomicUsize::new(0),
            usage_calls: AtomicUsize::new(0),

            invoke_requests: Mutex::new(Vec::new()),
            stream_requests: Mutex::new(Vec::new()),
            cancelled: Mutex::new(Vec::new()),
        }
    }

    /// 配 `health()` 的返回值。
    pub fn with_health(mut self, health: ProviderHealth) -> Self {
        self.health = health;
        self
    }

    /// 配 `invoke` 的行为（成功 / 失败 / 永不返回 / 略早于截止返回）。
    pub fn with_invoke(mut self, outcome: InvokeOutcome) -> Self {
        self.invoke = outcome;
        self
    }

    /// 配 `stream` 交出的那一枚流（含它自己的 `CallId`），或直接返错。
    pub fn with_stream(mut self, outcome: StreamOutcome) -> Self {
        self.stream = outcome;
        self
    }

    /// 配 `cancel` 的结果。
    pub fn with_cancel(mut self, outcome: Answer<()>) -> Self {
        self.cancel = outcome;
        self
    }

    /// 配 `list_models` 交出的清单（**可以含一个不在 D 的表里的 id**），或直接返错。
    pub fn with_list_models(mut self, outcome: Answer<Vec<ModelDescriptor>>) -> Self {
        self.list_models = outcome;
        self
    }

    /// 配 `describe_model` 的结果。
    pub fn with_describe_model(mut self, outcome: Answer<ModelDescriptor>) -> Self {
        self.describe_model = outcome;
        self
    }

    /// 配 `usage` 的结果。
    pub fn with_usage(mut self, outcome: Answer<Usage>) -> Self {
        self.usage = outcome;
        self
    }

    /// `health()` 被问过几次。
    pub fn health_calls(&self) -> usize {
        self.health_calls.load(Ordering::SeqCst)
    }

    /// `invoke()` 被调过几次。
    pub fn invoke_calls(&self) -> usize {
        self.invoke_calls.load(Ordering::SeqCst)
    }

    /// `stream()` 被调过几次。
    pub fn stream_calls(&self) -> usize {
        self.stream_calls.load(Ordering::SeqCst)
    }

    /// `cancel()` 被调过几次。
    pub fn cancel_calls(&self) -> usize {
        self.cancel_calls.load(Ordering::SeqCst)
    }

    /// `list_models()` 被调过几次。
    pub fn list_models_calls(&self) -> usize {
        self.list_models_calls.load(Ordering::SeqCst)
    }

    /// `describe_model()` 被调过几次。
    pub fn describe_model_calls(&self) -> usize {
        self.describe_model_calls.load(Ordering::SeqCst)
    }

    /// `usage()` 被调过几次。
    pub fn usage_calls(&self) -> usize {
        self.usage_calls.load(Ordering::SeqCst)
    }

    /// `invoke` 收到过的请求，按收到的次序。
    ///
    /// 它让「**适配器收到的是哪个模型的请求**」这件事可观测——`InvokeResponse` 里
    /// **没有任何回执字段**指回输入，故那条断言只能由适配器自己抄下来、用例从观测端读回去
    /// （与 C 的 `RecordingTool` 为「收到的授权是哪一枚」立记录的理由相同）。
    pub fn invoke_requests(&self) -> Vec<InvokeRequest> {
        self.invoke_requests.lock().unwrap().clone()
    }

    /// `stream` 收到过的请求，按收到的次序。
    pub fn stream_requests(&self) -> Vec<InvokeRequest> {
        self.stream_requests.lock().unwrap().clone()
    }

    /// `cancel` 收到过的 `CallId`，按收到的次序。
    ///
    /// **空 `Vec` 是一张否定式照片的读数**：「丢弃一条流不等于取消」那条断的就是它为空
    /// （设计 §3.5：取消经 `CallId` 走 `cancel()`，不经流的 drop）。
    pub fn cancelled(&self) -> Vec<CallId> {
        self.cancelled.lock().unwrap().clone()
    }
}

#[async_trait]
impl ModelProvider for FakeModel {
    async fn list_models(&self) -> Result<Vec<ModelDescriptor>, ProviderError> {
        self.list_models_calls.fetch_add(1, Ordering::SeqCst);
        match &self.list_models {
            Answer::Ok(models) => Ok(models.clone()),
            Answer::Fail(e) => Err(e.clone()),
        }
    }

    /// **不读 `list_models` 的配置**：本夹具的 `describe_model` 是独立可配的。
    ///
    /// C 那份 `FakeModel` 的 `describe_model` 是在 `list_models` 的结果里查表
    /// （查不到返 `UnknownModel`）——本份**刻意不同**：Task 10 要一条
    /// 「`list_models` 返错 / 返一个表外 id，而正常路径照过」的用例，
    /// 若两者串在一起，那条用例就没有一个干净的配置面（改一处会带动另一处）。
    async fn describe_model(&self, _id: &ModelId) -> Result<ModelDescriptor, ProviderError> {
        self.describe_model_calls.fetch_add(1, Ordering::SeqCst);
        match &self.describe_model {
            Answer::Ok(model) => Ok(model.clone()),
            Answer::Fail(e) => Err(e.clone()),
        }
    }

    async fn invoke(&self, request: InvokeRequest) -> Result<InvokeResponse, ProviderError> {
        self.invoke_calls.fetch_add(1, Ordering::SeqCst);
        // **记录发生在读配置之前**：一条永不返回的 `invoke` 也得先把「收到过这一枚请求」
        // 记下来，否则 Task 8 向一的用例读不到它。
        self.invoke_requests.lock().unwrap().push(request);

        match &self.invoke {
            InvokeOutcome::Reply(response) => Ok(response.clone()),
            InvokeOutcome::Fail(e) => Err(e.clone()),
            InvokeOutcome::Never => {
                std::future::pending::<Result<InvokeResponse, ProviderError>>().await
            }
            InvokeOutcome::ReplyAfter(delay, response) => {
                let delay = *delay;
                let response = response.clone();
                // **不在 runtime 的工作线程上睡**：`std::thread::sleep` 直接写在 `async fn` 里
                // 会把当前 worker 占住（`#[tokio::test]` 默认是 current-thread，连时钟都会被挡住），
                // 于是「稍等一会儿就返回」变成一个会拖住整条 runtime 的实现细节。
                // 放到阻塞池上睡，交给它的就只是一段真时间。
                //
                // **这里不是 `tokio::time::sleep`**：它要 `tokio` 的 `time` feature，
                // 而那颗 feature 由 Task 8 登记（它才是 `time` 的第一个使用者）。
                // `spawn_blocking` 只要 `rt`，那是本 crate 今天已有的 `rt-multi-thread` 带来的。
                tokio::task::spawn_blocking(move || std::thread::sleep(delay))
                    .await
                    .expect("阻塞池上的那段睡眠不会 panic，也不会被取消");
                Ok(response)
            }
        }
    }

    async fn stream(&self, request: InvokeRequest) -> Result<ModelStream, ProviderError> {
        self.stream_calls.fetch_add(1, Ordering::SeqCst);
        self.stream_requests.lock().unwrap().push(request);

        match &self.stream {
            StreamOutcome::Fail(e) => Err(e.clone()),
            StreamOutcome::Chunks { call, chunk } => {
                let chunks: Pin<
                    Box<dyn Stream<Item = Result<StreamChunk, ProviderError>> + Send>,
                > = Box::pin(OnceStream(Some(chunk.clone())));
                Ok(ModelStream {
                    call: call.clone(),
                    chunks,
                })
            }
        }
    }

    async fn cancel(&self, call: &CallId) -> Result<(), ProviderError> {
        self.cancel_calls.fetch_add(1, Ordering::SeqCst);
        self.cancelled.lock().unwrap().push(call.clone());
        match &self.cancel {
            Answer::Ok(()) => Ok(()),
            Answer::Fail(e) => Err(e.clone()),
        }
    }

    async fn usage(&self) -> Result<Usage, ProviderError> {
        self.usage_calls.fetch_add(1, Ordering::SeqCst);
        match &self.usage {
            Answer::Ok(usage) => Ok(usage.clone()),
            Answer::Fail(e) => Err(e.clone()),
        }
    }

    async fn health(&self) -> ProviderHealth {
        self.health_calls.fetch_add(1, Ordering::SeqCst);
        self.health
    }
}
