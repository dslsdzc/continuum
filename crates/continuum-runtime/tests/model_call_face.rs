//! 「模型调用没有任何强制点」的编译期照片（设计 §2.1）。
//!
//! # 一、形态：**判据是编译通过本身，本文件里没有一条断言宏**
//!
//! 具体说：没有 `assert!` / `assert_eq!`（不比较任何运行期的值）。Task 8 补进来的
//! [`assert_send`] 虽然名字里带 `assert`，但它**不是断言宏、也不比较任何值**——
//! 它是一个空函数，全部作用在**签名上那条 `T: Send` 的界**。
//!
//! 与设计 §3.4 的 `assert_send` 那一段同形态：被钉的对象是**类型的形状**
//! （「这个类型里放不下某样东西」／「这个面上有这几项」／「这个签名的 future 是 `Send`」），
//! 不是运行期行为。**本文件的 `#[test]` 跑起来都是空转**——它们值不值钱，全在「编译得过」。
//!
//! **订正（2026-10-08，Task 8）**：本行原写「本文件的**三条**用例跑起来都是空转」，
//! 而 Task 5 落地时本文件只有 **2** 条 `#[test]`（`probe` 那种编译期形态**不是** `#[test]`，
//! 它一个断言都不跑）。Task 8 补上第三条 `#[test]`（`the_async_segments_future_is_send`）之后
//! 「三条」这个数目才对得上——**但那是巧合，不是原话的辩护**：本行当时的数目就是错的。
//! **故本行不写数目**：数目会随本文件的下一次编辑再漂一次，而「跑起来是空转、值在编译得过」
//! 这句话对每一条都成立、不需要数。
//!
//! **由此推出一条与本仓别处相反的口径，写在这里免得被当成笔误**：本文件的「红」**就是编译不过**。
//! 本仓通用的那条纪律（「变异导致编译失败——那不是变红」）针对的是**运行期断言**的变异体
//! （那种变异体要它编得过、跑起来让断言红）。**本文件的对象不是它们**：它要表达的性质本来就是
//! 「这个类型放不下第四样东西」／「这个面上只有这几项」，**失效形态只有编译这一种**。
//! 故本文件的判据取在**编译失败**上，与别处相反；一旦编译失败，本文件里的任何一条都跑不到
//! （编译不过就一个测试目标都不产出），**这正是要的结果**。
//!
//! # 二、设计 §2.1 的三行：哪一行拍了、哪一行拍不出
//!
//! 设计 §2.1 的表（**实测**：表体 `:106-110`，表头 `:106`，三行分别是 `:108`／`:109`／`:110`）
//! 逐条核三个强制点。**逐行写明状态；拍不出的那一行写理由，不静默略过**：
//!
//! | 行 | 那句话（照设计 §2.1 引，未逐字核共享面原文） | 本文件里的状态 |
//! |---|---|---|
//! | (1) `:108` | 「模型侧的 `invoke` 收的是 `InvokeRequest`（`crates/continuum-core/src/model.rs:44-48`），**里面没有任何授权位**」 | **有照片**：[`the_invoke_request_has_no_authorization_slot`] |
//! | (2) `:109` | 「一次模型调用不写外部世界（它返回内容），故它不是效应；G 也**不写 Effect Journal**」 | **本阶段拍不出照片**，理由见下 |
//! | (3) `:110` | 「（§315 的请求面里）**也没有承载它的位置**」 | **有照片，且与 (1) 是同一张** |
//!
//! ## 为什么 (1) 与 (3) 落在同一张照片上，且本文件不为它们各写一条
//!
//! 两行问的是同一件事的两个名字——「请求面上有没有第四个位置」。**授权位与凭据位在这个类型上
//! 都是「第四个字段」**，一张恰三字段的全字段字面量同时拒绝这两样。
//!
//! **这个「分不开」据实写明，别把它读成一条顶两条的功**：若将来给 `InvokeRequest` 加一个
//! 既非授权位也非凭据位的字段，**两行会一起红，而本文件指不出是哪一行失了守**。
//! 要分开，得让两行各自有一个**只属于自己**的类型层落点；今天两个位都不存在，
//! **故没有这样的落点可写**——这不是省略，是没有。
//!
//! ## 为什么 (2) 拍不出照片（三条理由，缺一不可）
//!
//! 1. **编译期照片只能拍「类型的形状」，而 (2) 是「行为的缺席」。** 一个不写 Effect Journal 的
//!    实现与一个写了 Journal 的实现，**签名可以逐字相同**——「没有写」在类型上不可见，
//!    故不存在一张「编译得过」或「编译不过」能把它拍下来。这一层与 (1)／(3) 的差别是**根本的**：
//!    那两行说的是「类型里放不下一枚值」，这一行说的是「一段代码没有做一件事」。
//! 2. **换用源码文本守卫也不落在本 task 的判据上。** `tests/model_call_discipline.rs` 的第三条
//!    （`NO_POWER_NAMES`）已经把「G 的模块面不出现能力／效应／凭据的 crate 路径与类型名」钉住
//!    （设计 §2.2 第一条的落点），但那条是**拼法**上的下界，**不是「没有写」的证明**：
//!    一个不写 Journal 的模块可以引用 `continuum-effect` 的类型而仍然不写 Journal，反之亦然。
//! 3. **运行期照片这阶段不具备条件。** 「一次模型调用不写 Journal」要一张**运行期**照片，
//!    而它要的是一个**可观测的 Journal 端**：谁写、写到哪张表、从哪读回来。那条端到端现场
//!    属驱动接线，今天尚未建（G 也没有 Journal 的落点，设计 §9）。
//!
//!    **订正（2026-10-08，Task 8）**：本行原写「`select` / `call` 今天还没有…今天
//!    `src/model_call.rs` 上能跑的只有 `plan_candidates` / `snapshot` / `classify` /
//!    `into_call_error`，**它们一次外部调用都不发起**」。**Task 8 落地 `call` 之后这两句都不成立**：
//!    `call` 正是那个**发起外部调用**的函数。**那份理由不是本条的判据**（判据是「没有可观测的
//!    Journal 端」），故理由换掉、结论不变。
//!
//!    **Task 8 起多了一条更硬的构造性事实，但它仍不是运行期照片**：`call` 的**参数表里没有 `Tx`**
//!    （设计 §3.4），而 `Tx` 是 G 唯一的库入口——**故 `call` 在类型上写不了库**，
//!    更写不了 Journal。它是**构造性**的一条，不是「跑出来没写」的一条；后者的照片仍是
//!    Task 11 的 `the_path_writes_nothing_to_the_database`（逐表断行数不变）的事。
//!
//! # 三、第二张照片的证明力边界
//!
//! [`the_model_provider_face_is_callable_in_seven_ways`] 钉的是「§315 的七项在模型侧都在
//! （`crates/continuum-provider/src/model.rs:15-21`），且 G 的发起面拿得到它们」——
//! 它是 §2.1 强制点 (1) 那条正面依据的**对照面**：请求面上没有授权位（第一张照片），
//! 而调用面上有的恰是这七项（这一张）。
//!
//! **它钉的是「有这七个」，不钉「没有第八个」**。「`ModelProvider` 上不存在一个 `authorize` 方法」
//! 这一侧**本阶段没有照片**——要它需要 `trybuild` 与一份样例，而 `ModelProvider` 是 **C 的 trait**
//! （不是 G 的），它的面在 C 的计划里管；**本计划不为别人的 trait 新增一条 dev 依赖**。
//! **故这条边界是「没拍」，不是「拍过、通过」**，别把它读成一条已验的否定结论。
//!
//! # 四、第三条用例：异步段交出的 future 是 `Send`（Task 8 补的实体）
//!
//! [`the_async_segments_future_is_send`]（设计 §3.4 的 `assert_send` 那一段）**Task 5 只放了占位**
//! ——它要取 `select` / `call` 返回的 future，而那两枚函数当时还不存在。**Task 8 把实体补上**
//! （收件人按计划是 Task 8 的 Step 5「补 Task 5 的占位：异步段的 future 是 `Send`」；
//! **订正 2026-10-08**：本段原先引的是**本计划的行号**，那五处引用在本文件写就之后随计划的下一次
//! 编辑一起漂了。**故本文件对本计划的引用一律改成按内容**——「一个文件引用另一个文件的行号时，
//! 后者的编辑者不会知道」。**引用本计划的行号一处都不再留**，见下。）
//!
//! ## 一条据实记的边界：`call_stream` 今天不在这条用例里
//!
//! 计划的 Step 5 列的是三个 future（`select` / `call` / `call_stream`），
//! **而 `call_stream` 由 Task 9 落地、今天不存在**，写进去只会编不过。
//! **故本条今天断言的是 `select` 与 `call` 两枚**；`call_stream` 的实体落地时，
//! 由那一个 task 把它加进同一个 `probe_the_async_segments_are_send`（加一行，不是另立一条用例）。
//! **这不是省略，是收件人换了**（Task 5 的占位交给 Task 8，`call_stream` 那一份交给 Task 9）。
//! 差异记进 task-8 报告。
//!
//! **简报曾写「实体在 Task 10」，与计划不符**（实测计划 Task 10 是另一件事：不调
//! `usage()` / `list_models()` / `describe_model()` 的否定式照片）。本文件按计划记 Task 8。

use continuum_core::model::{InvokeRequest, Message, ModelId, Role};
use continuum_model_registry::{ExecutionCandidate, RankingPolicy};
use continuum_provider::ModelProvider;
use continuum_runtime::model_call::{call, select, CallInput, Candidate, RouteInput};
use std::sync::Arc;

/// 设计 §2.1 强制点 (1)（`:108`）的落点：`InvokeRequest` 上**没有授权位**。
///
/// 构造用的是**恰三个字段的全字段字面量**——全字段字面量不许省略，故它同时钉住两件事：
/// 「这三个字段在」与「**除这三个之外没有别的字段**」。后者才是本用例要的：
/// **给 `InvokeRequest` 加第四个字段（例如 `pub authorized: bool`）⇒ 本文件编译不过**
/// （实测错误码与位置见 task-5 报告；简报预告的是 `E0063`）。
///
/// **三个字段的名字与类型都写全**，并且**取回时各写一次类型标注**：
/// 本计划的 Task 1 一节里「四个纯数据类型的处置」那一段记了 D 的 Task 9 的实测
/// ——**整数字面量随字段类型推断**，
/// 不写全后缀时改字段类型**照样编译**；故 `max_tokens` 的字面量写成 `Some(64u32)`，
/// 三个字段再各按 `ModelId` / `Vec<Message>` / `Option<u32>` 标注一次。
/// 否则「字段类型被改宽／改窄」这类变异会**静默通过**这张照片。
///
/// **为什么这个形态比 `trybuild` 好**：被钉的对象是**请求面本身的性质**（「放不下一个授权位」），
/// 写成一个能编译的字面量比写一份「这样写编译不过」的样例更直接，
/// 也不需要为它新增 `trybuild` dev 依赖。
#[test]
fn the_invoke_request_has_no_authorization_slot() {
    let request = InvokeRequest {
        model: ModelId::new("probe-model"),
        messages: vec![Message {
            role: Role::User,
            content: "probe".to_owned(),
        }],
        max_tokens: Some(64u32),
    };

    // 字段名与类型再各收一次。这一段的用处是**把类型钉死**：只构造不取回的话，
    // 字段类型的改动可能被字面量一侧的推断吸收掉（见上面 `u32` 后缀那条）。
    let model: ModelId = request.model;
    let messages: Vec<Message> = request.messages;
    let max_tokens: Option<u32> = request.max_tokens;
    let _ = (model, messages, max_tokens);
}

/// §315 的七项在模型侧都在（`crates/continuum-provider/src/model.rs:15-21`），
/// 且 G 的发起面拿得到它们。
///
/// **七个各写一次，不抽代表**：抽一个代表就只剩一条调用，其余六项在类型上不再被提到，
/// 而「七项都在」正是这条要钉的东西。**逐项各写一次，是本用例唯一的内容**。
///
/// **这是个泛型函数、且从不被真正调用**——这是刻意的，不是没写完：判据是**编译通过本身**，
/// 而 Rust 对泛型函数体在**定义处**就做名字解析与类型检查（不实例化也查），
/// 故 `P: ModelProvider` 这个界上的七项里**任何一项缺席都会让本文件编不过**。
///
/// **`?Sized` 不是装饰**：C 的注册表持有的是 `Arc<dyn ModelProvider>`，
/// 故「拿得到这七项」必须对 `dyn ModelProvider` 也成立。
///
/// **边界（见文件头第三节）**：本函数钉「有这七个」，**不钉「没有第八个」**。
async fn probe<P: ModelProvider + ?Sized>(p: &P, id: &ModelId, req: InvokeRequest) {
    // 1/7 `list_models`
    let _models = p.list_models().await;
    // 2/7 `describe_model`
    let _described = p.describe_model(id).await;
    // 3/7 `invoke`。它按值收 `InvokeRequest`，故 ④ 那一次要另拿一份（`InvokeRequest: Clone`）。
    let _reply = p.invoke(req.clone()).await;
    // 4/7 `stream`
    let stream = p.stream(req).await;
    // 5/7 `cancel`。它收的是 `&CallId`，而这一枚 `CallId` **取自 ④ 的结果**
    // （`ModelStream::call`，`crates/continuum-core/src/model.rs:94`），不另造一枚——
    // §315 说「取消经 `CallId` 走 `cancel()`」，那枚 `CallId` 指的就是它。
    if let Ok(stream) = &stream {
        let _cancelled = p.cancel(&stream.call).await;
    }
    // 6/7 `usage`
    let _usage = p.usage().await;
    // 7/7 `health`（七项里唯一不返回 `Result` 的一项）
    let _health = p.health().await;
}

/// 见 [`probe`] 的文档。本用例自己不发任何请求：**判据是本文件编译通过本身**。
#[test]
fn the_model_provider_face_is_callable_in_seven_ways() {
    // 取一次 `probe` 的函数项，使它在编译单元里**有引用**——否则整个模块会报 `dead_code`，
    // 而本仓要求 0 warning（`tests/common/mod.rs:22-23` 记了同一条约束）。
    // 取 `dyn ModelProvider` 这一实例化还有一层意思：它同时说明这七项经 **trait object**
    // 也拿得到，而那正是 C 的注册表持有的形态（`Arc<dyn ModelProvider>`）。
    let _ = probe::<dyn ModelProvider>;
}

/// 设计 §3.4 的 `assert_send` 本体。**收 `&T` 而不是 `T`**：本用例要断的是
/// 「那个 future 的交出形态是 `Send`」，按 `&T` 取则**不把 future 消费掉**，
/// 下面还能 `.await` 它（设计 §3.4 原话：「`assert_send(&fut);`，再 `.await` 它」）。
fn assert_send<T: Send>(_: &T) {}

/// 取 `select` / `call` 交出的 future，断言它们是 `Send`，**再 `.await` 它们**。
///
/// **本函数从不被调用**，与 [`probe`] 同形（理由见文件头第一节）：Rust 对**非泛型**函数的
/// 函数体**在定义处**就做完类型检查（不实例化也查），而 `assert_send(&fut)` 这一句要求
/// `fut: Send`——**取证不需要真的跑一遍**。
///
/// **为什么用「编译得过」而不是 `trybuild`**：trybuild 钉的是「这样写编译不过」，
/// 而这里要钉的是「**这个签名的 future 是 `Send`**」——被钉的对象是签名本身的性质，
/// 写进用例比写成样例更直接，也不必为它新增一条 dev 依赖。
///
/// **它的证明力有边界，写明**（设计 §3.4 原话）：它只覆盖被断言的**这两枚** future，
/// **不证明 G 的每一处异步代码都不持 `Tx`**；后者由「异步段的参数表里没有 `Tx`」这条
/// 构造性事实兜住（`plan_candidates` 是唯一的 `Tx` 收口）。
///
/// **为什么两枚 future 要分开取、不能像 [`probe`] 那样列表**：它们的**输入类型各不相同**
/// （`select` 收候选集与策略，`call` 收一枚候选、句柄与载荷），
/// 故这里逐枚各取一次——**逐枚各断一次，不抽代表**（与 §315 那七项同一条判据）。
///
/// **红的条件（档位：取反）**：给 `select`（或 `call`）加一个**跨 `await` 活着**的
/// `tx: &Tx<'_>` 参数 → 本文件**编译不过**。
/// **「编译不过不算变红」在这里是刻意的例外，写明**：本判据的**红形态就是编译失败**
/// （与 P3A 用 trybuild 钉「写不出来」同类），因为被钉的对象是**类型层的性质**、
/// 不是运行期行为。编译不过时本文件一条用例都产出不了——**那正是要的结果**
/// （见文件头第一节末段）。
async fn probe_the_async_segments_are_send(
    candidates: Vec<Candidate>,
    route_input: RouteInput,
    // 这里的 `+ Send + Sync` **不是可选的装饰**：`select` 收的就是这个形态的参数
    // （它为什么收这个形态，写在 `src/model_call.rs` 那条签名的文档里），
    // 而把界擦成 `&dyn RankingPolicy` 就会**把要钉的那条性质从本 probe 里抹掉**。
    policy: &(dyn RankingPolicy + Send + Sync),
    candidate: &ExecutionCandidate,
    adapter: &Arc<dyn ModelProvider>,
    call_input: CallInput<'_>,
) {
    // ② 异步段交出的那一枚。
    let selecting = select(candidates, route_input, policy);
    assert_send(&selecting);
    let _ = selecting.await;

    // ③ 异步段交出的那一枚。
    let calling = call(candidate, adapter, call_input, None);
    assert_send(&calling);
    let _ = calling.await;
}

/// 见 [`probe_the_async_segments_are_send`] 的文档。本用例自己不发任何调用：
/// **判据是本文件编译通过本身**。
///
/// **第三条 future（`call_stream`）今天不在这里**，理由与收件人写在文件头第四节。
#[test]
fn the_async_segments_future_is_send() {
    // 取一次函数项，使它在编译单元里**有引用**——否则整个模块会报 `dead_code`，
    // 而本仓要求 0 warning（`tests/common/mod.rs:22-23` 记了同一条约束）。
    let _ = probe_the_async_segments_are_send;
}
