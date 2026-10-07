//! 「模型调用没有任何强制点」的编译期照片（设计 §2.1）。
//!
//! # 一、形态：**判据是编译通过本身，本文件没有一条 `assert`**
//!
//! 与设计 §3.4 的 `assert_send`（`:367-368`）同形态：被钉的对象是**类型的形状**
//! （「这个类型里放不下某样东西」／「这个面上有这几项」），不是运行期行为。
//! 本文件的三条用例跑起来都是**空转**——它们值不值钱，全在「编译得过」。
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
//! 设计 §2.1 的表（`:102-114`，表头 `:106`）逐条核三个强制点，三行分别是
//! `:108`（1）、`:109`（2）、`:110`（3）。**逐行写明状态；拍不出的那一行写理由，不静默略过**：
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
//! 3. **运行期照片这阶段不具备条件。** 可观测的 Journal 端要 `model_call.rs` 的调用路径先存在，
//!    而 `select` / `call` 今天还没有（计划 `:898` 的 Task 7、`:1003` 的 Task 8）。
//!    今天 `src/model_call.rs` 上能跑的只有 `plan_candidates`（`:88`）、`snapshot`（`:149`）、
//!    `classify`（`:203`）与 `into_call_error`（`:227`），**它们一次外部调用都不发起**，
//!    故「一次模型调用不写 Journal」这件事今天**没有可观测的现场**。
//!    **本条只写「今天没有」**：它在 Task 8 之后会不会有，本 task 判不了，也不替它下结论。
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
//! # 四、第三条用例是占位，本 task 不写它
//!
//! `the_async_segments_future_is_send`（设计 §3.4，`:367-368`）**本 task 只放占位**：
//! 它要取 `select` / `call` 返回的 future，而那两枚函数尚不存在。
//! **收件人按计划是 Task 8**：计划 `:1003` 的 Task 8 一节，它的 Files 一节 `:1009` 明列本文件
//! （「Task 5 的占位，此处补实体」），它的 Step 5 `:1100-1102` 写的正是「补 Task 5 的占位：
//! 异步段的 future 是 `Send`」。
//! **（简报写的是「实体在 Task 10」，与计划不符。实测计划 `:1221` 的 Task 10 是另一件事
//! （不调 `usage()` / `list_models()` / `describe_model()` 的否定式照片）。本文件按计划记 Task 8，
//! 差异记进 task-5 报告，不在这里改计划。）** 记此以免本文件被读成漏了一条。

use continuum_core::model::{InvokeRequest, Message, ModelId, Role};
use continuum_provider::ModelProvider;

/// 设计 §2.1 强制点 (1)（`:108`）的落点：`InvokeRequest` 上**没有授权位**。
///
/// 构造用的是**恰三个字段的全字段字面量**——全字段字面量不许省略，故它同时钉住两件事：
/// 「这三个字段在」与「**除这三个之外没有别的字段**」。后者才是本用例要的：
/// **给 `InvokeRequest` 加第四个字段（例如 `pub authorized: bool`）⇒ 本文件编译不过**
/// （实测错误码与位置见 task-5 报告；简报预告的是 `E0063`）。
///
/// **三个字段的名字与类型都写全**，并且**取回时各写一次类型标注**：
/// 计划 `:405-409` 记了 D 的 Task 9 的实测——**整数字面量随字段类型推断**，
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
