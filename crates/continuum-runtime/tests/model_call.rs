//! G 这一侧的失败面：分类表与 `ModelCallError` 的照片（设计 §6.1、§11），
//! 以及**假模型适配器夹具自身的用例**（Task 2）。
//!
//! **本文件到本 task 为止只有这两部分**：四段流程（取快照、调排序、`call`、`call_stream`）
//! 的用例由后续 task 追补。

mod common;

use common::FakeModel;
use continuum_core::model::{CallId, InvokeRequest, Message, ModelId, ProviderHealth, Role};
use continuum_core::ProviderError;
use continuum_graph::failure::FailureClass;
use continuum_model_registry::{
    BudgetView, CandidateScore, FamilyPreference, FamilyRelation, RankingPolicy, Ratio,
    RoutableModel, RoutingReason, RoutingRequest, SkillDimension, TaskSkillRequirement,
};
use continuum_provider::model::ModelProvider;
use continuum_runtime::model_call::{classify, into_call_error};
use continuum_runtime::ModelCallError;
use std::sync::Mutex;

/// `ProviderError` 的五个变体**逐项**各得一个类别（设计 §6.1 的表）。
///
/// **逐项各写一次断言，不抽代表**：这张表的实现是手写的 `match`，
/// 每个臂能各自漂移，一条表驱动的循环会把「某一臂被改错」藏在别的臂后面。
/// 照片形态因此是**五条独立的断言**，而不是「一个循环里断言五次」。
#[test]
fn each_provider_error_variant_maps_to_its_class() {
    assert_eq!(
        classify(&ProviderError::Transport("连接被重置".into())),
        Some(FailureClass::Transient),
        "传输失败通常是连接抖动 → Transient"
    );
    assert_eq!(
        classify(&ProviderError::Unavailable("上游 503".into())),
        Some(FailureClass::Transient),
        "供应商侧暂时不可用 → Transient"
    );
    assert_eq!(
        classify(&ProviderError::Protocol("响应不合契约".into())),
        Some(FailureClass::Permanent),
        "契约不符、重试无益 → Permanent"
    );
    assert_eq!(
        classify(&ProviderError::UnknownModel("登记表里的 id 与适配器对不上".into())),
        Some(FailureClass::Permanent),
        "配置缺陷 → Permanent"
    );
    assert_eq!(
        classify(&ProviderError::Cancelled("调用方发起的取消".into())),
        None,
        "调用方自己发的取消不是失败，不进入分类（设计 §6.1）"
    );
}

/// **两侧对钉**：同一次转换里，`Cancelled` 得 `None`、`Transport` 得 `Some(..)`。
///
/// **为什么这条要单列**：只钉「四个失败臂各得类别」会漏掉另一侧——
/// 本条的 `None` 那一侧把「取消不是失败」这条判定放进了**体**里。
/// **两侧各挡一边，故两侧都要在**：一个只钉一侧的用例挡不住「某一侧恒真」的实现
/// （两个方向各举得出一个仍能编译的变异体，红分别落在本条的哪一侧，见下）。
///
/// **爆炸面据实写：不是「只有本条会红」，是跨用例三处。** 两个方向的变异各红三处：
///
/// - 把 `classify` 的 `Cancelled` 那一臂改成 `Some(FailureClass::Transient)` ⇒ 红在
///   `each_provider_error_variant_maps_to_its_class` 里 `Cancelled` 那条断言、
///   **本条的 `None` 那一侧**、以及 `the_error_type_carries_the_provider_error_verbatim`
///   里 `Cancelled` 那一臂的 `panic!`（它拿到的是 `Provider { class: Transient, … }`）；
/// - 把 `Transport` 那一臂改成 `None` ⇒ 红在 `each_…` 里 `Transport` 那条断言、
///   **本条的 `Some(..)` 那一侧**、以及 `the_error_type_…` 里 `Transport` 那一臂的 `panic!`。
///
/// **三处的成因**：`each_…` 的第五条断言与 `the_error_type_…` 的第五臂断的是同一件事，
/// 而 `into_call_error` 的类别与分支都取自 `classify`，故表一改、类型的搬运跟着改。
/// **故本条的价值不在「唯一会红的用例」**，而在把两侧放进**同一个体**——
/// 只有这样才能对「某一侧恒真」的实现在同一处同时立起两侧守卫。
///
/// **上文的「红在哪一处」用断言锚点写、不写行号**：行号会被本文件的下一次编辑平移
/// （本注释自己就在被引断言的上方），锚点不会。
#[test]
fn a_cancelled_call_is_not_a_failure() {
    let cancelled = classify(&ProviderError::Cancelled("调用方发起的取消".into()));
    let transport = classify(&ProviderError::Transport("连接被重置".into()));

    assert_eq!(
        cancelled, None,
        "Cancelled 不是失败：调用方自己发的中止不该被记成一次失败（设计 §6.1）"
    );
    assert_eq!(
        transport,
        Some(FailureClass::Transient),
        "Transport 仍是失败——这一侧挡住「一律返回 None」的实现"
    );
}

/// `ModelCallError` **不压平** `ProviderError`：`Provider { class, source }` 的 `source`
/// 是给进去的那一枚（**逐变体各断一次**）；`Cancelled` 走 `ModelCallError::Cancelled`
/// 那一枚，**且那一枚不带 `class` 字段**。
///
/// 设计 §6.1 明写 `Routing` 要**带出 D 的错**、`Provider` 要**带出类别**，
/// 两者都不是「一枚同名的变体」——本条就是这条判据的照片。
///
/// **「`Cancelled` 不带 `class`」为什么值得单列**：它不是字段的取舍，它是
/// 「一次正常中止不会被记成一次失败」这条判据**在类型上的落点**——若 `Cancelled`
/// 也带 `class`，调用方会在一个没有失败的地方读到 `FailureClass`。
#[test]
fn the_error_type_carries_the_provider_error_verbatim() {
    // 一、Transport：`source` 逐字回读，`class` 是设计 §6.1 的那一格。
    match into_call_error(ProviderError::Transport("连接被重置".into())) {
        ModelCallError::Provider { class, source } => {
            assert_eq!(class, FailureClass::Transient, "Transport 的类别该是 Transient");
            assert_eq!(
                source,
                ProviderError::Transport("连接被重置".into()),
                "source 该是给进去的那一枚，逐字回读"
            );
        }
        other => panic!("Transport 该走 Provider 那一枚，实际 {other:?}"),
    }

    // 二、Unavailable。
    match into_call_error(ProviderError::Unavailable("上游 503".into())) {
        ModelCallError::Provider { class, source } => {
            assert_eq!(class, FailureClass::Transient, "Unavailable 的类别该是 Transient");
            assert_eq!(
                source,
                ProviderError::Unavailable("上游 503".into()),
                "source 该是给进去的那一枚，逐字回读"
            );
        }
        other => panic!("Unavailable 该走 Provider 那一枚，实际 {other:?}"),
    }

    // 三、Protocol。
    match into_call_error(ProviderError::Protocol("响应不合契约".into())) {
        ModelCallError::Provider { class, source } => {
            assert_eq!(class, FailureClass::Permanent, "Protocol 的类别该是 Permanent");
            assert_eq!(
                source,
                ProviderError::Protocol("响应不合契约".into()),
                "source 该是给进去的那一枚，逐字回读"
            );
        }
        other => panic!("Protocol 该走 Provider 那一枚，实际 {other:?}"),
    }

    // 四、UnknownModel。
    match into_call_error(ProviderError::UnknownModel("登记表里的 id 对不上".into())) {
        ModelCallError::Provider { class, source } => {
            assert_eq!(class, FailureClass::Permanent, "UnknownModel 的类别该是 Permanent");
            assert_eq!(
                source,
                ProviderError::UnknownModel("登记表里的 id 对不上".into()),
                "source 该是给进去的那一枚，逐字回读"
            );
        }
        other => panic!("UnknownModel 该走 Provider 那一枚，实际 {other:?}"),
    }

    // 五、Cancelled：走 `Cancelled` 那一枚，不是 `Provider` 那一枚。
    match into_call_error(ProviderError::Cancelled("调用方发起的取消".into())) {
        ModelCallError::Cancelled { source } => {
            assert_eq!(
                source,
                ProviderError::Cancelled("调用方发起的取消".into()),
                "source 该是给进去的那一枚，逐字回读"
            );
        }
        other => panic!("Cancelled 该走 Cancelled 那一枚，实际 {other:?}"),
    }

    // **「那一枚不带 `class` 字段」是本条里唯一的一张编译期照片**：
    // 下面这枚字面量只给 `source`。若 `Cancelled` 也长出一个 `class` 字段，
    // 这一行**编不过**（少给一个字段），故「不带 class」由编译通过本身钉住。
    // 它不能写成运行期断言——「一个字段不存在」不是运行期可观察的性质。
    let _cancelled_has_no_class_field = ModelCallError::Cancelled {
        source: ProviderError::Cancelled("调用方发起的取消".into()),
    };
}

// ===== Task 2：夹具自身的用例 =====
//
// **为什么这三条要与夹具同批落地**（「先钉机制再看结论」）：夹具是后续每一条用例的仪表。
// 一台恒不生效的仪表——配置项配了不读、计数器计了不增、记录抄了不存——会让后续的用例**假绿**，
// 而假绿与「真的没调那一次」在断言的读数上**完全一样**（都是 0、都是同一枚默认值）。
// 故三条各钉一侧机制：**配置项真的被读、六个计数器真的各自会增、五个预算维度真的被抄下来**。

/// 配 `Unavailable` 的假适配器，`health()` 就报 `Unavailable`；**两侧对钉**：改配 `Healthy` 就报 `Healthy`。
///
/// 两向缺一不可：只钉向一时，一个「忽略配置项、永远返回 `Healthy`」的夹具全绿，
/// 而它在 Task 3 的四条用例里同样会绿掉三条——`Unavailable` 与 `Degraded` 都读不出来。
///
/// **红的条件（档位：取反）**：夹具把配置项读反，或忽略它、恒返回 `Healthy` → 向一红；
/// 恒返回 `Unavailable` → 向二红。
#[tokio::test]
async fn the_fake_adapter_reports_the_health_it_was_configured_with() {
    let unavailable = FakeModel::new().with_health(ProviderHealth::Unavailable);
    assert_eq!(
        unavailable.health().await,
        ProviderHealth::Unavailable,
        "配成 Unavailable 的适配器，health() 该报 Unavailable"
    );

    let healthy = FakeModel::new().with_health(ProviderHealth::Healthy);
    assert_eq!(
        healthy.health().await,
        ProviderHealth::Healthy,
        "配成 Healthy 的适配器，health() 该报 Healthy——这一侧挡住「恒报 Unavailable」的实现"
    );
}

/// 六个方法各调一次，**六个计数器逐个断言为 1**；`health` 是本 task 补上的第七个（见下）。
///
/// **它是后续三类否定式照片的共用仪表**：Task 10 的「不调 `usage()`」「不调 `list_models()`」
/// 断的是**计数为 0**，Task 3 的「探活三次」断的是**计数为 3**。
/// 「计数为 0」这类断言有一处天然的假绿面：**一个恒不增的计数器**会让它永远成立——
/// 本条正是那一侧的守卫，它钉的是「**这些计数器真的会增**」。
///
/// **`health` 是本 task 补上的第七个，不在计划 Task 2 Step 4 的计数器清单里**：
/// 计划同一段又说这台仪表要服务「探活三次」那一类照片（Task 3 的 `one_probe_per_candidate`），
/// 而那一条要读的正是 `health` 的计数——清单漏了它。补上第七条的代价是一行断言，
/// 收益是这台仪表**覆盖它自己声称要服务的三类**，且 Task 3 落地时不必回头改夹具。
///
/// **红的条件（档位：移除）**：删掉某一个计数器（或它的递增语句）→ 对应的那一条红。
#[tokio::test]
async fn the_fake_adapter_counts_every_method_call() {
    let fake = FakeModel::new();

    let _ = fake.invoke(a_request()).await;
    let _ = fake.stream(a_request()).await;
    let _ = fake.cancel(&CallId::new("call-1")).await;
    let _ = fake.usage().await;
    let _ = fake.list_models().await;
    let _ = fake.describe_model(&ModelId::new("fake-1")).await;
    let _ = fake.health().await;

    assert_eq!(fake.invoke_calls(), 1, "invoke 被调了一次");
    assert_eq!(fake.stream_calls(), 1, "stream 被调了一次");
    assert_eq!(fake.cancel_calls(), 1, "cancel 被调了一次");
    assert_eq!(fake.usage_calls(), 1, "usage 被调了一次");
    assert_eq!(fake.list_models_calls(), 1, "list_models 被调了一次");
    assert_eq!(fake.describe_model_calls(), 1, "describe_model 被调了一次");
    assert_eq!(fake.health_calls(), 1, "health 被调了一次");
}

/// 记录用的 `RankingPolicy` **抄下来的那一份预算**：五个 `Option<i64>`，逐维照录。
///
/// **为什么不直接留一枚 `BudgetView`**：`BudgetView` **不派生任何东西**
/// （D §6.1：没有当场消费方的派生一律不加），故它既比对不了、也打印不了。
/// 而**这五个字段都是 `Copy`**，抄进本类型之后逐维断言即可——这正是设计 §8.2 说的
/// 「这条照片**不需要** `BudgetView` 派生 `Clone` 或 `PartialEq`」：值被移进 `RoutingRequest`，
/// 策略从 `&RoutingRequest` 抄 `Copy` 字段。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CapturedBudget {
    money: Option<i64>,
    wall_time: Option<i64>,
    token: Option<i64>,
    gpu_time: Option<i64>,
    network_transfer: Option<i64>,
}

impl CapturedBudget {
    /// 从一枚请求的预算视图里抄。
    ///
    /// **五个字段逐个写出来**：少写一行（字段）本行编不过，故「抄没有抄全」在编译期就有一侧守卫；
    /// 另一侧（把某一维抄成一个常数）编得过，由用例的五条断言逐维兜住。
    fn of(request: &RoutingRequest) -> Self {
        let budget = &request.budget;
        Self {
            money: budget.money,
            wall_time: budget.wall_time,
            token: budget.token,
            gpu_time: budget.gpu_time,
            network_transfer: budget.network_transfer,
        }
    }
}

/// 记录用的排序策略：把 `evaluate` 收到的请求里那五个 `Option<i64>` 抄进自己的记录。
///
/// 它是设计 §8.2 那条两向对钉（`None` ≠ `Some(0)`）的**观测端**：驱动把 `BudgetView` 装进
/// `RoutingRequest`，策略从请求里读回来，用例比对「送进去的那一份」与「读回来的那一份」。
///
/// **为什么「抄」是一个可以单独调用的方法，而不是只写在 `evaluate` 里**：
/// `RankingPolicy::evaluate` 收一枚 `&RoutableModel`，而**本仓 crate 外造不出一枚**——
/// `ModelProfile::try_new` 是 `pub(crate)`，公面唯一的产生者是 `persist::load_profile`，
/// 那要先有一个播过画像行的库。库那条夹具是 Task 6 的事，Task 2 不建库，故本 task 的照片
/// 走 [`RecordingPolicy::capture`] 这一层。**`evaluate` → `capture` 那条接线由 Task 7 经 `rank`
/// 端到端钉住**：本文件里 `evaluate` 只有 `self.capture(request)` 一句，别处没有第二份抄写。
struct RecordingPolicy {
    seen: Mutex<Vec<CapturedBudget>>,
}

impl RecordingPolicy {
    fn new() -> Self {
        Self {
            seen: Mutex::new(Vec::new()),
        }
    }

    /// 抄下这一枚请求的预算，并交出一枚**常数**打分——本策略不排序，只记录。
    ///
    /// 打分取常数是刻意的：本条的照片只判「抄到了什么」，
    /// 让打分也参与判断会把两件事混进同一条用例的红里。
    fn capture(&self, request: &RoutingRequest) -> CandidateScore {
        self.seen.lock().unwrap().push(CapturedBudget::of(request));
        CandidateScore {
            compatibility: Ratio::try_new(1.0).expect("1.0 是合法的比值"),
            confidence: Ratio::try_new(1.0).expect("1.0 是合法的比值"),
            reason: RoutingReason::new(
                FamilyRelation::SameFamily,
                Vec::new(),
                Vec::new(),
                Vec::new(),
            ),
        }
    }

    /// 抄下来的每一份记录，按被抄的次序。
    fn captured(&self) -> Vec<CapturedBudget> {
        self.seen.lock().unwrap().clone()
    }
}

impl RankingPolicy for RecordingPolicy {
    fn evaluate(&self, request: &RoutingRequest, _model: &RoutableModel) -> CandidateScore {
        self.capture(request)
    }
}

/// 一份手工构造的路由请求：只有 `budget` 是参数（本条用例只关心那五个 `Option<i64>`）。
fn a_routing_request(budget: BudgetView) -> RoutingRequest {
    RoutingRequest {
        requirements: TaskSkillRequirement::try_new(vec![SkillDimension::Coding])
            .expect("一个维度是非空集合"),
        family: FamilyPreference::Auto,
        availability: Vec::new(),
        budget,
    }
}

/// 记录用的 `RankingPolicy` 把 `&RoutingRequest` 的**五个 `Option<i64>`** 一次抄全。
///
/// **本 task 只钉「抄得下来」**：五个值各取一个**互不相同**的取值，逐维断言。
/// **两向对钉**（`None` 不许折成 `Some(0)`、`Some(0)` 不许折成 `None`）**是 Task 7 的事**：
/// 这里的 `RoutingRequest` 是手工构造的，判不了「驱动怎么投影」——
/// `money = None` 与 `token = Some(0)` 这两个取值由 Task 7 的用例给。
///
/// **五维的取值刻意都非 `None`、都非 `Some(0)`、且互不相同**：三条各挡一类**等价变异体**
/// （与真值相等的变异在照片上不可观察，于是同一形状的**真**缺陷也照样绿）——
///
/// - 某一维取 `None` ⇒「把这一维抄成常数 `None`」与真值相等，本条对它**没有守卫**；
/// - 某一维取 `Some(0)` ⇒「抄成常数 `Some(0)`」同理；
/// - 两维取同一个值 ⇒「把这一维抄成另一维」那一枚不可观察。
///
/// **实测过一次**：本条初稿给 `money` 取的是 `None`，`money: None` 那一枚变异体**跑了绿**
/// （`.tmp/t2-mut-M3_constant_money.log`，作废的尝试）。**处置是换取值、不是补用例**——
/// 补一条断言挡不住等价变异体。
///
/// **红的条件（档位：移除）**：某一维被抄成常数（`None`、或别的维的值）→ 该维的断言红；
/// 整个 `capture` 不再记录 → `captured().len()` 那一条红。
#[test]
fn the_recording_policy_captures_the_five_budget_dimensions() {
    let policy = RecordingPolicy::new();
    let request = a_routing_request(BudgetView {
        money: Some(-1),
        wall_time: Some(2),
        token: Some(7),
        gpu_time: Some(-3),
        network_transfer: Some(1_000_000),
    });

    policy.capture(&request);

    let seen = policy.captured();
    assert_eq!(
        seen.len(),
        1,
        "一次 capture 该恰好抄下一份记录——0 份意味着「记录」这条机制根本没生效"
    );
    let captured = seen[0];
    assert_eq!(captured.money, Some(-1), "money 该是送进去的那一个 Some(-1)");
    assert_eq!(
        captured.wall_time,
        Some(2),
        "wall_time 该是送进去的那一个 Some(2)"
    );
    assert_eq!(captured.token, Some(7), "token 该是送进去的那一个 Some(7)");
    assert_eq!(captured.gpu_time, Some(-3), "gpu_time 该是送进去的那一个 Some(-3)");
    assert_eq!(
        captured.network_transfer,
        Some(1_000_000),
        "network_transfer 该是送进去的那一个 Some(1_000_000)"
    );
}

/// 一枚最小可用的 `InvokeRequest`——计数器那条用例只关心「调了几次」，不关心载荷。
fn a_request() -> InvokeRequest {
    InvokeRequest {
        model: ModelId::new("fake-1"),
        messages: vec![Message {
            role: Role::User,
            content: "ping".into(),
        }],
        max_tokens: Some(16),
    }
}
