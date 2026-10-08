//! G 这一侧的用例，按四段流程与失败面分节：
//!
//! 1. **失败面**：分类表与 `ModelCallError` 的照片（设计 §6.1、§11）——Task 1；
//! 2. **夹具自身的用例**：假模型适配器真的读配置、真的计数（Task 2）；
//! 3. **候选集的构造（同步段）与可用性快照**——Task 6 ＋ Task 3；
//! 4. **`select`**：组装请求、调 `rank`、把句柄与排序结果成对带出——Task 7；
//! 5. **`call`**：装配请求、发起一次调用、两侧对钉的截止——Task 8；
//! 6. **`call_stream` 与中止入口**：流式那一支的装配、`CallId` 的原样带出、
//!    「丢弃流不是取消」的两侧对钉、以及 `abort` 的失败面——Task 9。
//!
//! **订正（2026-10-08，Task 7）**：本行原写「本文件到本 task 为止只有这两部分：四段流程
//! （取快照、调排序、`call`、`call_stream`）的用例由后续 task 追补」——**那句在 Task 3／6
//! 落地时就已经不成立**（候选集与快照两节在那之后进的同一个文件），Task 7 落地后更远。
//! 错的只是这句概述，各节的判据不变。

mod common;

use common::{descriptor, Answer, FakeModel, InvokeOutcome, StreamOutcome};
use continuum_core::model::{
    CallId, InvokeRequest, InvokeResponse, Message, ModelId, ModelStream, ProviderHealth, Role,
    StreamChunk, Usage,
};
use continuum_core::ProviderError;
use continuum_graph::failure::FailureClass;
use continuum_graph::p1_graph_migrations;
use continuum_model_registry::{
    list_registered, p3d_model_migrations, BudgetView, CandidateScore, ExecutionCandidate,
    FamilyPreference, FamilyRelation, LifecycleState, RankingPolicy, Ratio, RoutableModel,
    RoutingError, RoutingReason, RoutingRequest, SkillDimension, TaskSkillRequirement,
};
use continuum_persist::{builtin_migrations, Db, PersistError, Tx, Value};
use continuum_provider::model::ModelProvider;
use continuum_provider::ProviderRegistry;
use continuum_runtime::model_call::{
    abort, call, call_stream, classify, into_call_error, plan_candidates, select, snapshot,
    CallInput, CallPlan, Candidate, RouteInput,
};
use continuum_runtime::ModelCallError;
use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

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

/// 一枚常数打分：`compatibility` 与 `confidence` 都取 1.0，family 记 `SameFamily`。
///
/// **两条策略共用它**（本文件的 [`RecordingPolicy`] 与 Task 7 的 [`ReversingPolicy`]）。
/// 打分取常数是刻意的：那些用例判的是「抄到了什么」或「输出是什么次序」，
/// **让打分也参与判断会把两件事混进同一条用例的红里**——读的人分不清红是「次序配错了」
/// 还是「分数变了」。
fn constant_score() -> CandidateScore {
    CandidateScore {
        compatibility: Ratio::try_new(1.0).expect("1.0 是合法的比值"),
        confidence: Ratio::try_new(1.0).expect("1.0 是合法的比值"),
        reason: RoutingReason::new(FamilyRelation::SameFamily, Vec::new(), Vec::new(), Vec::new()),
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
    /// **`evaluate` 收到的 `availability` 的抄本**，按被抄的次序（Task 7 追补）。
    ///
    /// 与 `seen` 分开两枚 `Vec` 而不是合成一枚记录类型：两处的消费方不同
    /// （预算那条读 `seen`，可用性那条读这里），合起来会让任一条用例的读口都多带一半无关字段。
    availability: Mutex<Vec<Vec<(ModelId, ProviderHealth)>>>,
}

impl RecordingPolicy {
    fn new() -> Self {
        Self {
            seen: Mutex::new(Vec::new()),
            availability: Mutex::new(Vec::new()),
        }
    }

    /// 抄下这一枚请求的预算与可用性，并交出一枚**常数**打分——本策略不排序，只记录。
    ///
    /// 打分取常数是刻意的（见 [`constant_score`]）。
    fn capture(&self, request: &RoutingRequest) -> CandidateScore {
        self.seen.lock().unwrap().push(CapturedBudget::of(request));
        self.availability
            .lock()
            .unwrap()
            .push(request.availability.clone());
        constant_score()
    }

    /// 抄下来的每一份记录，按被抄的次序。
    fn captured(&self) -> Vec<CapturedBudget> {
        self.seen.lock().unwrap().clone()
    }

    /// 抄下来的每一份可用性，按被抄的次序。
    fn captured_availability(&self) -> Vec<Vec<(ModelId, ProviderHealth)>> {
        self.availability.lock().unwrap().clone()
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

/// 一份**五个量纲都不构成约束**（全 `None`）的预算视图。
///
/// Task 7 的用例里与预算无关的那几条用它：`Some` 的取值今天只能由测试构造
/// （设计 §10 第 2 条），而「不构成约束」是一个不需要编数值的形态。
fn no_constraints() -> BudgetView {
    BudgetView {
        money: None,
        wall_time: None,
        token: None,
        gpu_time: None,
        network_transfer: None,
    }
}

/// 一枚 [`RouteInput`]：只有预算可变，需求与 family 取固定值。
///
/// **需求与 family 也由测试直接构造**：它们来自规划侧，而那一边尚未建（设计 §12 第 1 条），
/// 故「驱动真的这么传」在这条路径上钉不了——本函数就是那条事实的落点。
fn a_route_input(budget: BudgetView) -> RouteInput {
    RouteInput {
        requirements: TaskSkillRequirement::try_new(vec![SkillDimension::Coding])
            .expect("一个维度是非空集合"),
        family: FamilyPreference::Auto,
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

// ===== Task 6 ＋ Task 3：候选集的构造（同步段）与可用性快照 =====
//
// **这两组用例随同一个提交落地，是因为计划里的一处任务序倒置**：`Candidate` 的字段私有、
// **唯一的构造点在 `plan_candidates` 里**，故 Task 3 的快照用例在 Task 6 的 `plan_candidates`
// 落地之前**编不过**。计划的处置是「Task 3 的 Step 1/2 与 Task 6 合并执行——快照的实现与
// 它的用例随 Task 6 一起落地」；计划明确**不取**那条替代（给 `Candidate` 开一条
// `#[cfg(test)]` 的 crate 内构造口），理由是那会在测试与生产之间开第二条构造通道，而字段私有
// 本身就是设计 §3.1 第 1 条的落点。**两组各自的判据不混**：候选集那组钉 §3.2 的三条合取，
// 快照那组钉 §3.3 的粒度与 §4.4 第 2 条。Task 3 的四条落点见本节末。
//
// **夹具的来路（设计 §11 前置二）**：`ModelProfile` 在本 crate 外构造不出来（字段私有、
// `try_new` 是 `pub(crate)`），而 `save_profile` 收一枚 `ModelProfile`——**这条链不能自举**，
// 故画像行只能**由裸 SQL 播下**，再走类型化接口（`load_profile`）读回。**代价据实记**：
// 本文件从此嵌着 D 的表结构（`model_registry` 的列名、`model_profile` 的十一列与两个 JSON
// 容器列的编码），**D 改掉本夹具依赖的那些列时这里会红**——这正是想要的（G 的候选集是
// D 的表的下游，两者不一致必须看得见），但它的射程只到「G 依赖的那些列」，不是「D 一改表就红」。

/// 建一个装了三组迁移的临时库。
///
/// 三个迁移**都装**：`builtin_migrations()` 给 `events` / `audit_log`、`p1_graph_migrations()`
/// 给 `node_attempt`、`p3d_model_migrations()` 给三张模型表。**本 task 的用例只读最后一组**
/// ——前两组在本文件里今天没有消费方（G 的路径不碰 `events` / `audit_log` / `node_attempt`），
/// 装着是为了让本文件所有用例**共用一个库形状**，免得「行数不变」那类用例另建一个只差几条
/// 迁移的库、两个夹具各自漂移。
///
/// **临时目录随 `Db` 一起返回**：`TempDir` 一被 drop 就把库文件删了，故调用方要把它绑到一个
/// 活到用例结束的名字上（用例里一律写 `let (_dir, db) = a_model_db();`）。
fn a_model_db() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("model-call.db");
    let mut migrations = builtin_migrations();
    migrations.extend(p1_graph_migrations());
    migrations.extend(p3d_model_migrations());
    let db = Db::open_with(&path, migrations).unwrap();
    db.migrate().unwrap();
    (dir, db)
}

/// 播一行 `model_registry`：`id` 与十态里的一个。
///
/// **状态串经 `LifecycleState::as_str()` 取，不手写编码**——手写会把 D 的枚举编码在 G 的
/// 测试里抄成第二份，而那是这条耦合里唯一不必付的一份。**列名是手写的**（那一份付掉了）。
///
/// **不调 `register_model`**：它恒播 `discovered`，而本节要各种状态，故直接播。
fn register_with_state(tx: &Tx<'_>, id: &str, state: LifecycleState) {
    tx.execute(
        "INSERT INTO model_registry (id, lifecycle_state) VALUES (?1, ?2)",
        &[Value::text(id), Value::text(state.as_str())],
    )
    .unwrap();
}

/// 播一行 `model_profile`（十一列全给）。**必须先有同 id 的登记项**：
/// `model_profile.id` 是 `REFERENCES model_registry(id)` 的外键，而 `open_with` 打开了
/// `PRAGMA foreign_keys=ON`。
///
/// 三个列表列是**手写**的 JSON 文本（不是 `to_string` 的产物），`cost_profile` /
/// `latency_profile` 取 `NULL`（= 未登记），`confidence` 取合法的十进制串。
/// 本节没有任何用例读这些取值，它们只要求**合法**——否则失败会发生在 `load_profile` 的解码里，
/// 报错位置就指不到被测函数。
fn insert_profile(tx: &Tx<'_>, id: &str) {
    tx.execute(
        "INSERT INTO model_profile
           (id, version, provider, model_revision, modalities, tools, failure_modes,
            cost_profile, latency_profile, evidence_count, confidence)
         VALUES (?1, 'version-beta', 'provider-gamma', 'revision-delta',
                 ?2, ?3, '[]', NULL, NULL, 42, '0.94')",
        &[
            Value::text(id),
            Value::text(r#"["text"]"#),
            Value::text(r#"["tool-epsilon"]"#),
        ],
    )
    .unwrap();
}

/// 把若干 id 交给一个适配器服务（`register_model` 是一组 id 一个句柄）。
fn serve<A: ModelProvider + 'static>(registry: &mut ProviderRegistry, ids: &[&str], adapter: Arc<A>) {
    let handle: Arc<dyn ModelProvider> = adapter;
    registry
        .register_model(ids.iter().map(|id| ModelId::new(*id)).collect(), handle)
        .unwrap();
}

/// 一条候选的**完整**来路：登记 + 画像 + `active` + 注册表里有一个自己的适配器。
///
/// 返回那枚 `FakeModel`（不是 `Arc<dyn ModelProvider>`）：快照那四条要读它的计数器与它配过的
/// 健康度，而这两样都在具体类型上。
fn seed_routable(
    tx: &Tx<'_>,
    registry: &mut ProviderRegistry,
    id: &str,
    health: ProviderHealth,
) -> Arc<FakeModel> {
    register_with_state(tx, id, LifecycleState::Active);
    insert_profile(tx, id);
    let adapter = Arc::new(FakeModel::new().with_health(health));
    serve(registry, &[id], Arc::clone(&adapter));
    adapter
}

/// 候选集里的 id，按候选的次序。
fn ids_of(candidates: &[Candidate]) -> Vec<String> {
    candidates
        .iter()
        .map(|c| c.model().profile().id().as_str().to_owned())
        .collect()
}

/// 在一份快照里按 id 取那一条的健康度。
///
/// **按 id 查而不按下标**：本函数服务的断言是「(id, 健康度) 逐项对应」，
/// 而按下标查会把「次序」混进「对应」里——两条判据各由各的断言钉（次序那条另有一条）。
fn health_of<'a>(snapshot: &'a [(ModelId, ProviderHealth)], id: &str) -> &'a ProviderHealth {
    snapshot
        .iter()
        .find(|(model, _)| model.as_str() == id)
        .map(|(_, health)| health)
        .unwrap_or_else(|| panic!("快照里没有 {id} 这一条：{snapshot:?}"))
}

/// 合取成立 → 进候选集（**正面照片**）。
///
/// **只钉三个「不进」而不钉这一条时，一个「永远返回空向量」的实现全绿**——而空向量在本函数里
/// 会被折成一枚 `NoEligibleCandidate`，整条路径永远不可用（这正是 fail-open 的反面）。
///
/// **红的条件（档位：取反）**：把传进 `RoutableModel::try_new` 的状态参数换成常量 `Active`
/// （丢弃真实状态）→ **本条不红**（这一个模型本来就是 `active`），红在同批的
/// `a_stale_registered_model_is_not_a_candidate`——**两条配对才是完整的守卫**。
#[test]
fn a_model_with_a_profile_past_the_gate_and_an_adapter_is_a_candidate() {
    let (_dir, db) = a_model_db();
    let tx = db.begin().unwrap();
    let mut registry = ProviderRegistry::new();
    let adapter = seed_routable(&tx, &mut registry, "m-served", ProviderHealth::Healthy);

    let candidates = plan_candidates(&tx, &registry).expect("三条合取都成立，该有候选");

    assert_eq!(
        ids_of(&candidates),
        vec!["m-served".to_owned()],
        "登记 + 画像 + 过闸门 + 注册表里有适配器 → 该在候选集里"
    );
    // **句柄身份**：候选里那枚句柄该就是注册表里那一个（解析自 `model_for`）。
    // 只有一条候选时「按 id 配对」退化成这一条；**非退化那一半（三条候选、三个句柄、
    // 排序之后仍配对）是 Task 7 的 `the_result_pairs_every_candidate_with_its_own_adapter`**，
    // 本条不代它钉。
    let expected: Arc<dyn ModelProvider> = adapter.clone();
    assert!(
        Arc::ptr_eq(candidates[0].adapter(), &expected),
        "候选里那枚句柄该是注册表里那一个，不是别处新造的"
    );
}

/// 登记了、**没有画像** → 不进候选集；同批的其余候选仍在。
///
/// **这一条只让一个条件为假**：它登记了适配器（合取 (c) 成立）、状态也是 `active`
/// （(b) 的后半成立），**唯独没有画像行**。故它红的理由不会有第二种读法。
///
/// **红的条件（档位：移除）**：把 `load_profile` 的 `Ok(None)` 那一臂从「丢弃」改成
/// `.expect("有画像")` → 夹具里这个无画像的模型让它 panic，而用例断言的是
/// 「它不在候选集里、其余候选仍在」→ 红。
#[test]
fn a_model_without_a_profile_is_not_a_candidate() {
    let (_dir, db) = a_model_db();
    let tx = db.begin().unwrap();
    let mut registry = ProviderRegistry::new();

    register_with_state(&tx, "m-bare", LifecycleState::Active);
    serve(&mut registry, &["m-bare"], Arc::new(FakeModel::new()));
    seed_routable(&tx, &mut registry, "m-profiled", ProviderHealth::Healthy);

    let candidates = plan_candidates(&tx, &registry).expect("另一个候选仍在，不是空集");

    assert_eq!(
        ids_of(&candidates),
        vec!["m-profiled".to_owned()],
        "没有画像的登记项该被丢弃，而有画像的那一条仍在"
    );
}

/// 画像齐全、有适配器，但状态是 `stale` → 不进候选集。
///
/// **与 D 的十态用例的分工写明**：十态**逐项**的照片在 D
/// （`crates/continuum-model-registry/src/lifecycle.rs` 的 `only_the_six_routable_states_pass_the_gate`）；
/// G 只钉**与 §3.2 相异的那一格**（`stale`——它是被裁决额外挡下的第四态，
/// 与 §249 禁的三态不同源），**不重跑十态**（唯一入口原则，设计 §11 末的刻意不重复 (a)）。
/// `RoutableModel` 字段私有、唯一构造点在 `try_new` 内，故 G 这边**交不出**一个未过闸门的
/// 候选——「`NotRoutable` 不可达」的构造性事实由类型兜住，不由本条承重。
///
/// **红的条件（档位：取反）**：把传进 `RoutableModel::try_new` 的状态参数换成常量（如 `Active`）
/// → `stale` 那一条过闸门，红。
#[test]
fn a_stale_registered_model_is_not_a_candidate() {
    let (_dir, db) = a_model_db();
    let tx = db.begin().unwrap();
    let mut registry = ProviderRegistry::new();

    seed_routable(&tx, &mut registry, "m-active", ProviderHealth::Healthy);
    // `stale`：画像齐全、有适配器，**只有状态不过闸门**。
    register_with_state(&tx, "m-stale", LifecycleState::Stale);
    insert_profile(&tx, "m-stale");
    serve(&mut registry, &["m-stale"], Arc::new(FakeModel::new()));

    let candidates = plan_candidates(&tx, &registry).expect("m-active 仍在");

    assert_eq!(
        ids_of(&candidates),
        vec!["m-active".to_owned()],
        "`stale` 是 §249 之外被单独挡下的一态，它不该进候选集"
    );
}

/// 登记 + 画像 + 过闸门，但注册表里**没有它** → 不进候选集。
///
/// **这一条只让一个条件为假**：唯一缺的是适配器（(a)、(b) 都成立）。
///
/// **红的条件（档位：收紧）**：把「丢弃这一条」改成「整批失败」
/// （`continue` → `return Err(…NoEligibleCandidate)`）→ 用例在 `.expect("m-served 仍在")` 处红。
///
/// **计划给的那一枚写不出来，据实记**：计划写的是「把 `Err(RegistryError::NotFound { .. })`
/// 那一臂改成 `?`（让它向上传播）」，而 `RegistryError` 到 `ModelCallError` **没有 `From` 实现**
/// （`ModelCallError` 的五个变体里没有一枚装得下适配器注册表的失败），故那一版**编不过**。
/// **「放宽」这一档在本条上没有可写的变异体**：放宽 = 不丢弃，而 `Candidate` 按值持有一枚
/// 适配器句柄，**没有句柄就构造不出候选**——「放宽」在这一处是**类型上做不到**
/// （不是「有规则在挡」）。故本条的变异体取「收紧」那一档，档位标「收紧」。
#[test]
fn a_model_with_no_adapter_is_not_a_candidate() {
    let (_dir, db) = a_model_db();
    let tx = db.begin().unwrap();
    let mut registry = ProviderRegistry::new();

    register_with_state(&tx, "m-lonely", LifecycleState::Active);
    insert_profile(&tx, "m-lonely");
    seed_routable(&tx, &mut registry, "m-served", ProviderHealth::Healthy);

    let candidates = plan_candidates(&tx, &registry).expect("m-served 仍在");

    assert_eq!(
        ids_of(&candidates),
        vec!["m-served".to_owned()],
        "注册表里没有适配器的登记项该被丢弃，有适配器的那一条仍在"
    );
}

/// 「无适配器即不是候选」**两侧对钉**（设计 §3.2 的丢弃规则）。
///
/// **只钉向一会让一个「永远返回空」的实现全绿**：向一断的是「不在候选集里」，
/// 而那与「候选集永远为空」在读数上一样。**向二就是那条守卫**，同时它也是
/// 「**丢弃是显式的、不是静默的**」这句话的照片——登记回来之后它**真的回来了**，
/// 而不是「一直都不在」。
///
/// 两向在**同一个测试体内**、**同一个 `Tx`**、**同一份登记表**上做完：中间只改了注册表这一样。
///
/// **红的条件**：向一（档位：移除）删掉空判定的短路 → 向一红（实测报「却得到 0 条候选」）；
/// 向二（档位：取反）让候选集恒为空 → 向二红（实测红在「它该回到候选集」那条断言上）。
#[test]
fn a_model_kept_out_for_want_of_an_adapter_comes_back_once_it_is_registered() {
    let (_dir, db) = a_model_db();
    let tx = db.begin().unwrap();
    let mut registry = ProviderRegistry::new();

    register_with_state(&tx, "m-x", LifecycleState::Active);
    insert_profile(&tx, "m-x");

    // 向一：表里有、注册表里没有 → 丢弃；而它是**唯一**候选 → 整批以 `NoEligibleCandidate` 回。
    let err = match plan_candidates(&tx, &registry) {
        Ok(candidates) => panic!(
            "唯一候选没有适配器，该以 NoEligibleCandidate 回，却得到 {} 条候选",
            candidates.len()
        ),
        Err(e) => e,
    };
    assert!(
        matches!(
            err,
            ModelCallError::Routing(RoutingError::NoEligibleCandidate)
        ),
        "丢弃完之后候选集为空，该是 D 的 NoEligibleCandidate（不是新造一枚同名的错），得到 {err:?}"
    );

    // 向二：同一条 `register_model` 之后它**回到**候选集。
    serve(&mut registry, &["m-x"], Arc::new(FakeModel::new()));
    let candidates = plan_candidates(&tx, &registry).expect("登记回适配器之后该有候选");
    assert_eq!(
        ids_of(&candidates),
        vec!["m-x".to_owned()],
        "登记回适配器之后它该回到候选集——丢弃是显式的，不是「一直都不在」"
    );
}

/// 候选集**只从 D 的 `model_registry` 出发**（设计 §3.2 的三来源规则、§4.5 的「逐行来自主键表」）。
///
/// 假适配器的 `list_models()` 自报一个**不在 D 的表里**的 id；G **不调 `list_models()`**，
/// 故它不该出现在候选集里（登记是**路由**的权威，`list_*` 是**描述**的权威）。
///
/// **这一条同时是 `DuplicateModelCandidate` 在 G 路径上不可达的照片的一半**：
/// 候选集的 id **两两不同**，判据是它们逐行来自 `model_registry` 的主键列。
/// **用计数断言**（去重后计数 == 原计数），不用「看起来没有」。
///
/// **红的条件：计划给的那一枚写不出来，据实记。** 计划写的是「把候选集的 id 来源从
/// `list_registered` 换成适配器的 `list_models()`」，而那**编不过**：`list_models()` 是
/// `async` 的，`plan_candidates` 是**同步**函数（设计 §3.1 的签名即判据）——
/// 实测那一版报 `error[E0728]: await is only allowed inside async functions`
/// （`.tmp/t6-mut-M5-ids-from-list-models.log`）。
///
/// **两条结构断言的处境不同，分开写（订正 2026-10-08：原写「两条都没有变异体照片」）**：
///
/// - **「候选 id ⊆ `list_registered` 的 id 集合」：没有可写的变异体**（这是推理结论，不是穷举）。
///   成因是构造性的两条：(i) 任何候选都得是一枚 `RoutableModel`，它唯一的构造点 `try_new`
///   要一枚 `ModelProfile`，而画像只能从 `model_profile` 表读出、那张表的 `id` 又是
///   **`model_registry(id)` 的外键**；(ii) `ProviderRegistry` 的公开面只有按 id 查
///   （`model_for`），**没有枚举**，故「从注册表造候选」这条路走不到。即这一半在本仓是
///   **D 的表结构 ＋ crate 边界**兜住的，不是 G 这段代码兜住的。
/// - **「候选 id 两两不同」：写得出来，且已实测跑红**（档位：取反）。把 `list_registered`
///   的结果整份复制再接回到它自己后面（同一个 id 因此出现两遍）→ 本条与另外 6 条用例红，
///   本条的 `distinct.len() == ids.len()` 那一条断言红。
///   **故「两条结构断言一枚都写不出来」为假：只有 ⊆ 那一半写不出来。**
///
/// **⊆ 那一半的回归护栏价值**：将来若有人给候选集另开一条来源（或 D 撤掉那条外键），它会红。
///
/// **非空锚是必须的**：三条断言（⊆、`!any(ghost)`、去重计数）在一个**空的候选集**上**全部
/// 空真**，`.expect("登记表里有一个模型")` 拿到的也是 `Ok(vec![])`、**不会 panic**——故没有非空锚
/// 时，一个「永远返回空候选集」的实现（`M4`）让本条**整条绿**。下面 `ids == ["m-served"]`
/// 那一条就是非空锚；**订正 2026-10-08：原写「`M4` 让本条红」与实测相反**，实测 `M4` 下本条是绿的，
/// 这正是加这条锚的起因。
#[test]
fn the_candidate_set_comes_only_from_the_registry_table() {
    let (_dir, db) = a_model_db();
    let tx = db.begin().unwrap();
    let mut registry = ProviderRegistry::new();

    register_with_state(&tx, "m-served", LifecycleState::Active);
    insert_profile(&tx, "m-served");
    serve(
        &mut registry,
        &["m-served"],
        Arc::new(FakeModel::new().with_list_models(Answer::Ok(vec![descriptor(
            "ghost-not-registered",
        )]))),
    );

    let candidates = plan_candidates(&tx, &registry).expect("登记表里有一个模型");

    let registered: Vec<String> = list_registered(&tx)
        .unwrap()
        .into_iter()
        .map(|(id, _)| id.as_str().to_owned())
        .collect();
    let ids = ids_of(&candidates);

    // **非空锚**：没有它，上面三条结构断言在空候选集上全是空真——一个「恒返空候选集」的实现
    // 整条绿。位置在结构断言**之前**：空集在这里就红，读的人不必猜到后面的断言为何空过。
    assert_eq!(
        ids,
        vec!["m-served".to_owned()],
        "候选集该恰是登记表里那一条；空集说明这个实现根本没在造候选（下面的结构断言在空集上空真）"
    );
    assert!(
        ids.iter().all(|id| registered.contains(id)),
        "候选集的 id 集合该 ⊆ `list_registered` 的 id 集合：候选 {ids:?}，登记 {registered:?}"
    );
    assert!(
        !ids.iter().any(|id| id == "ghost-not-registered"),
        "适配器自报而登记表里没有的 id 不该进候选集（G 不调 `list_models()`）：{ids:?}"
    );
    let distinct: BTreeSet<&String> = ids.iter().collect();
    assert_eq!(
        distinct.len(),
        ids.len(),
        "候选集的 id 该两两不同（逐行来自 `model_registry` 的主键列）：{ids:?}"
    );
}

/// 候选集为空 → `NoEligibleCandidate`（设计 §3.1 第 9 条）。**两个来路各一条**：
/// (a) 一个模型都没登记；(b) 登记了、也有适配器，但**全部无画像**（`load_profile` 回 `None`，
/// 因此到不了 `try_new` 那道闸门）。
///
/// **措辞收窄（2026-10-08）**：来路 (b) 原写「全部被闸门挡下」，而这里构造的是「全部无画像」
/// ——设计 §11 把两者并列为同一来路，故原话不算错，但射程比构造宽；按本仓口径收窄成实际构造的那一个。
///
/// **各断言是哪一枚 `Err`**（`ModelCallError::Routing(RoutingError::NoEligibleCandidate)`），
/// 不只「返回了 `Err`」——两层都点名。
///
/// **计划还给本条派了另一半：「断言 `rank` 一次都没被调用」**（用一个会 panic 的
/// `RankingPolicy` 作证）。**那一半在本 task 里写不出来，据实记在这里**：
/// `plan_candidates` 的签名里**没有 `&dyn RankingPolicy`**（设计 §3.1 的签名即判据），
/// 而 G 这一侧调 `rank` 的地方只有 `select`（Task 7，今天不存在）。
/// 即「空候选集不调 `rank`」在 Task 6 上是**类型上做不到**，不是「有规则在挡」——
/// 一个没有策略参数的函数**拿不出**一次 `rank` 调用。写一个恒不被调用的 panic 策略会是一条
/// **恒真**的断言（外加一条 `dead_code` 警告），故不写。
/// **该半的照片归 Task 11**（端到端那一节，`select` 硬依赖 Task 7）——**订正 2026-10-08**：
/// 原写「归 Task 7」，而实读计划全文，这条用例只在 Task 6 的简报里出现过一次，
/// Task 7 与 Task 11 两节都没有它；协调者裁定落到 Task 11。
///
/// **红的条件（档位：移除）**：把空判定的短路删掉 → 函数返回 `Ok(vec![])`，
/// 两条断言各在其「该是 `Err`」那一处红。
#[test]
fn an_empty_candidate_set_is_no_eligible_candidate() {
    // 来路 (a)：一个模型都没登记（注册表里也就无从有适配器）。
    let (_dir, db) = a_model_db();
    let tx = db.begin().unwrap();
    let registry = ProviderRegistry::new();
    let err = match plan_candidates(&tx, &registry) {
        Ok(candidates) => panic!(
            "表是空的，该以 NoEligibleCandidate 回，却得到 {} 条候选",
            candidates.len()
        ),
        Err(e) => e,
    };
    assert!(
        matches!(
            err,
            ModelCallError::Routing(RoutingError::NoEligibleCandidate)
        ),
        "一个都没登记 → NoEligibleCandidate，得到 {err:?}"
    );

    // 来路 (b)：登记了、也有适配器，但全部无画像（都在 `load_profile` 那一步被丢弃）。
    let (_dir2, db2) = a_model_db();
    let tx2 = db2.begin().unwrap();
    let mut registry2 = ProviderRegistry::new();
    register_with_state(&tx2, "m-bare", LifecycleState::Active);
    serve(&mut registry2, &["m-bare"], Arc::new(FakeModel::new()));
    let err2 = match plan_candidates(&tx2, &registry2) {
        Ok(candidates) => panic!(
            "全部无画像，该以 NoEligibleCandidate 回，却得到 {} 条候选",
            candidates.len()
        ),
        Err(e) => e,
    };
    assert!(
        matches!(
            err2,
            ModelCallError::Routing(RoutingError::NoEligibleCandidate)
        ),
        "全部无画像 → 同一枚 NoEligibleCandidate（两种情形走同一条失败路径），得到 {err2:?}"
    );
}

/// 读库失败 → `Storage`，**且内层是库层报出来的那一枚 `PersistError`**（两层都断言）。
///
/// **造法**：建一个**不带任何业务表**的库（`Db::open_with(&path, Vec::new())`，照
/// `crates/continuum-workspace/src/gate.rs` 的 `a_failed_audit_write_…` 那三行），
/// `list_registered` 因而必然撞上一枚**真实的**库错误——不比伪造一个失败的 `Tx` 更假。
///
/// **名字里原有一条 `and_leaves_nothing_behind`，本条不写它**：那个库上一个业务表都没有，
/// 「G 没有半写副作用」在这里**不可观测**（G 也建不出表来）。完整的行数不变断言
/// （五张表逐表不变）是 Task 11 的 `the_path_writes_nothing_to_the_database` 的事，
/// **不在这里拿一句恒真的话顶替**。
///
/// **红的条件（档位：放宽）**：把 `list_registered` 的失败吞掉（`unwrap_or_default()`）
/// → 读库失败被当成「一个模型都没有」，于是报的是 `NoEligibleCandidate` 而不是 `Storage`，红。
/// （**不写「把 `?` 拿掉」**：`ModelCallError::Storage` 上是 `#[source]` 不是 `#[from]`，
/// 所以实现里那一处本来就是 `map_err(ModelCallError::Storage)?`，`?` 从来不生效在裸的
/// `PersistError` 上。）
#[test]
fn a_storage_failure_is_reported_as_storage() {
    let dir = tempfile::tempdir().unwrap();
    let db = Db::open_with(&dir.path().join("no-tables.db"), Vec::new()).unwrap();
    db.migrate().unwrap();
    let tx = db.begin().unwrap();
    let registry = ProviderRegistry::new();

    let err = match plan_candidates(&tx, &registry) {
        Ok(candidates) => panic!(
            "库里没有表，该以 Storage 回，却得到 {} 条候选",
            candidates.len()
        ),
        Err(e) => e,
    };
    match err {
        ModelCallError::Storage(inner) => assert!(
            matches!(inner, PersistError::Database(_)),
            "内层该是库层报出来的那一枚 PersistError::Database，得到 {inner:?}"
        ),
        other => panic!("读库失败该报 Storage，得到 {other:?}"),
    }
}

// ----- Task 3 的四条：可用性快照（设计 §3.3、§4.4 第 2 条） -----

/// 三个候选 → 服务它们的那个适配器的 `health()` 被问**三次**。钉「不跳着问」。
///
/// **三个候选共用一个适配器是承重的**：设计 §3.3 写死的粒度是**逐候选**取快照，不是逐适配器。
/// 三个候选各配一个适配器时，「逐候选」与「逐适配器」给出同一个计数（3），两条实现都能过；
/// **只有共用适配器才能把两者分开**——按句柄去重的实现给出 1。
/// **它不是一条性能选择**，是设计 §4.4 第 1 条「`availability` 必须覆盖候选集」的构造性保证
/// （逐候选取 ⇒ 天然全覆盖）。
///
/// **红的条件（档位：移除）**：在 `snapshot` 里按适配器去重（同一句柄只问一次、结果复用）
/// → 计数为 1，红。
///
/// **一条刻意不写的**：探活**并发与否**。设计 §3.3 明写「是否并发探活不影响可观察结果」
/// （`rank` 只按 id 查条目，对次序无判据），故它是实现选择、**没有照片**——
/// 本条不写一条「必须是顺序」的断言。
#[tokio::test]
async fn one_probe_per_candidate() {
    let (_dir, db) = a_model_db();
    let tx = db.begin().unwrap();
    let mut registry = ProviderRegistry::new();
    let shared = Arc::new(FakeModel::new());
    for id in ["m-a", "m-b", "m-c"] {
        register_with_state(&tx, id, LifecycleState::Active);
        insert_profile(&tx, id);
    }
    serve(&mut registry, &["m-a", "m-b", "m-c"], Arc::clone(&shared));

    let candidates = plan_candidates(&tx, &registry).expect("三个候选都在");
    assert_eq!(candidates.len(), 3, "夹具该给出三个候选");

    let probed = snapshot(&candidates).await;

    assert_eq!(probed.len(), 3, "三个候选该有三条快照");
    assert_eq!(
        shared.health_calls(),
        3,
        "快照是逐候选取的：同一个适配器服务三个候选时 health() 该被问三次，不是一次"
    );
}

/// 快照**按候选自己的 `ModelId`** 记下健康度，且条目次序与候选集同序（设计 §3.3）。
///
/// 三个适配器配**两两不同**的健康度：取值若有两个相同，「(id, 健康度) 配对错位」这一类缺陷
/// 会在那一对上不可观察（等价变异体）。
///
/// **红的条件（档位：取反）**：把 `(id, health)` 写反，或按候选次序把健康度错位配到另一个 id 上
/// → 三条断言里对不上的那些红。
#[tokio::test]
async fn the_snapshot_is_indexed_by_the_candidates_own_model_id() {
    let (_dir, db) = a_model_db();
    let tx = db.begin().unwrap();
    let mut registry = ProviderRegistry::new();
    seed_routable(&tx, &mut registry, "m-a", ProviderHealth::Healthy);
    seed_routable(&tx, &mut registry, "m-b", ProviderHealth::Degraded);
    seed_routable(&tx, &mut registry, "m-c", ProviderHealth::Unavailable);

    let candidates = plan_candidates(&tx, &registry).expect("三个候选都在");
    let probed = snapshot(&candidates).await;

    // 逐项对应：三条各断一次（不抽代表）。
    assert_eq!(
        health_of(&probed, "m-a"),
        &ProviderHealth::Healthy,
        "m-a 那一条该是它自己那个适配器的健康度"
    );
    assert_eq!(
        health_of(&probed, "m-b"),
        &ProviderHealth::Degraded,
        "m-b 那一条该是它自己那个适配器的健康度"
    );
    assert_eq!(
        health_of(&probed, "m-c"),
        &ProviderHealth::Unavailable,
        "m-c 那一条该是它自己那个适配器的健康度"
    );
    // 次序：条目与候选集逐位对应（设计 §3.3「快照按候选的次序生成」）。
    let probed_ids: Vec<String> = probed.iter().map(|(id, _)| id.as_str().to_owned()).collect();
    assert_eq!(
        probed_ids,
        ids_of(&candidates),
        "快照的条目该与候选集同序、逐位对应"
    );
}

/// 快照的**长度与 id 集合**都等于候选集（**不是只断 `len`**）。
///
/// 这是设计 §4.5 里 `UnknownAvailability` 在 G 路径上**不可达**的那条构造性断言的一半
/// （另一半在 Task 11 的端到端对应）。
///
/// **红的条件（档位：收紧）**：在快照里加一条「只对 `Healthy` 的候选给条目」的过滤
/// → `len` 与 id 集合**同时**不等，红。
#[tokio::test]
async fn the_snapshot_is_as_long_as_the_candidate_set() {
    let (_dir, db) = a_model_db();
    let tx = db.begin().unwrap();
    let mut registry = ProviderRegistry::new();
    seed_routable(&tx, &mut registry, "m-a", ProviderHealth::Healthy);
    seed_routable(&tx, &mut registry, "m-b", ProviderHealth::Degraded);
    seed_routable(&tx, &mut registry, "m-c", ProviderHealth::Unavailable);

    let candidates = plan_candidates(&tx, &registry).expect("三个候选都在");
    let probed = snapshot(&candidates).await;

    assert_eq!(
        probed.len(),
        candidates.len(),
        "快照的条目数该等于候选集的大小"
    );
    let probed_ids: BTreeSet<String> = probed.iter().map(|(id, _)| id.as_str().to_owned()).collect();
    let candidate_ids: BTreeSet<String> = ids_of(&candidates).into_iter().collect();
    assert_eq!(
        probed_ids, candidate_ids,
        "快照的 id 集合该与候选集的 id 集合相等（不是只断 len）"
    );
}

/// `Unavailable` **原样**留在快照里；`Degraded` 同理——**G 不在这一层做任何二次裁剪**。
///
/// 设计 §4.4 第 2 条：只过滤 `Unavailable` 是 `rank` 的事，而 `Degraded` 与 `Healthy`
/// 之间**没有判据**。故快照这一层既**不丢** `Unavailable`、也**不升格** `Degraded`。
///
/// **两侧对钉**：向一钉「不丢」（改配 `Degraded` 时若实现把它丢了，向二红）；
/// 向二钉「不升格」（把 `Degraded` 记成 `Healthy` 的实现让向二红）。
///
/// **红的条件（档位：收紧）**：在快照里就把 `Unavailable` 滤掉（丢掉该候选）
/// → 本条向一（快照里找不到 m-u）与 `the_snapshot_is_as_long_as_the_candidate_set` **同时**红。
///
/// **档位订正（2026-10-08，协调者裁定）**：原标「放宽」，与上文 `the_snapshot_is_as_long_as_
/// the_candidate_set` 标「收紧」矛盾——那两条变异体做的是**同一类事**（加一条过滤、丢条目：
/// 一个是「只对 `Healthy` 的候选给条目」，一个是「把 `Unavailable` 滤掉」）。按本仓口径
/// （**加过滤＝收紧；去掉一个过滤的效果＝放宽**），**两枚都是收紧**。
#[tokio::test]
async fn a_unavailable_adapter_is_carried_through_verbatim() {
    // 向一：`Unavailable` 原样带过——候选集本身也非空（G 不因健康度丢候选）。
    let (_dir, db) = a_model_db();
    let tx = db.begin().unwrap();
    let mut registry = ProviderRegistry::new();
    seed_routable(&tx, &mut registry, "m-u", ProviderHealth::Unavailable);
    let candidates = plan_candidates(&tx, &registry).expect("健康度不过闸门，候选集该非空");
    let probed = snapshot(&candidates).await;
    assert_eq!(
        health_of(&probed, "m-u"),
        &ProviderHealth::Unavailable,
        "`Unavailable` 该原样留在快照里，不被丢掉"
    );

    // 向二：`Degraded` 也是原样——**不是被丢掉、也不是被升格成 `Healthy`**。
    let (_dir2, db2) = a_model_db();
    let tx2 = db2.begin().unwrap();
    let mut registry2 = ProviderRegistry::new();
    seed_routable(&tx2, &mut registry2, "m-d", ProviderHealth::Degraded);
    let candidates2 = plan_candidates(&tx2, &registry2).expect("候选集非空");
    let probed2 = snapshot(&candidates2).await;
    assert_eq!(
        health_of(&probed2, "m-d"),
        &ProviderHealth::Degraded,
        "`Degraded` 该是 `Degraded`，既不被丢掉、也不被升格成 `Healthy`"
    );
}

// ===== Task 7：组装请求、调 `rank`、成对带出句柄 =====

/// **反序**策略：打分取常数（见 [`constant_score`]），`compare` 按 `ModelId` **降序**。
///
/// # 为什么要一枚会改次序的策略
///
/// `plan_candidates` 的候选按 id **升序**出（`list_registered` 的 SQL 是 `ORDER BY id ASC`），
/// 而缺省的 `compare` 在分数打平时也按 id **升序**兜底——**两者恰好同序**。
/// 于是两枚变异体在缺省策略下**不可观察**：
///
/// - 「G 把句柄按**输入次序**配回去」——输入次序与 D 的输出次序相同，错配与正确配对
///   给出同一个结果，全绿；
/// - 「G 对 `alternatives` **再排一次序**」——再排一次的次序与原次序相同，也全绿。
///
/// 本策略让 D 的输出次序**与输入次序相反**（m-a / m-b / m-c 进 → m-c / m-b / m-a 出），
/// 那两枚变异体这才落得到红。**它同时是「策略可替换」的一次实际使用**（设计 §5.1）：
/// G 不假定 D 用哪一份策略，只按 `rank` 的输出走。
///
/// **本策略自己不是被测对象**：它的 `evaluate` 与 `compare` 都短到读一眼就能核，
/// 故没有为它单立用例——它承重的地方是上面那两条断言（次序真被改了，断言才有效力）。
struct ReversingPolicy;

impl RankingPolicy for ReversingPolicy {
    fn evaluate(&self, _request: &RoutingRequest, _model: &RoutableModel) -> CandidateScore {
        constant_score()
    }

    fn compare(&self, a: &ExecutionCandidate, b: &ExecutionCandidate) -> Ordering {
        b.model().as_str().cmp(a.model().as_str())
    }
}

/// 组装进 `RoutingRequest` 的 `availability`，其 **id 集合**等于候选集的 id 集合
/// （**集合相等，不是 `len` 相等**）。
///
/// 这是设计 §4.5 里 `UnknownAvailability` 在 G 路径上**不可达**的那条构造性断言：
/// 快照逐候选取（Task 3 的 `the_snapshot_is_as_long_as_the_candidate_set` 钉它自身的
/// 长度与 id），而本条钉的是**组装进请求之后仍相等**——中间隔了一次搬运（快照 → 请求字段），
/// 而搬运是可以丢东西的。
///
/// **三个候选的健康度取三档各一**：`Degraded` 不是摆设——「只把 `Healthy` 的条目放进
/// `availability`」那一枚变异体要靠它才落得到红（若三条都是 `Healthy`，那枚变异体是
/// 等价变异体）。`Unavailable` 那一条同样要留在 `availability` 里：`rank` 才做过滤，
/// **G 不在这一层裁剪**（设计 §4.4 第 2 条）。
///
/// **红的条件（档位：收紧）**：在组装请求时加一条过滤（只把 `Healthy` 的条目放进
/// `availability`）→ id 集合不等，红。
#[tokio::test]
async fn the_request_reaching_the_policy_carries_the_candidates_availability() {
    let (_dir, db) = a_model_db();
    let tx = db.begin().unwrap();
    let mut registry = ProviderRegistry::new();
    seed_routable(&tx, &mut registry, "m-a", ProviderHealth::Healthy);
    seed_routable(&tx, &mut registry, "m-b", ProviderHealth::Degraded);
    seed_routable(&tx, &mut registry, "m-c", ProviderHealth::Unavailable);

    let candidates = plan_candidates(&tx, &registry).expect("健康度不过闸门，三条都该在");
    let candidate_ids: BTreeSet<String> = ids_of(&candidates).into_iter().collect();

    let policy = RecordingPolicy::new();
    let _plan = select(candidates, a_route_input(no_constraints()), &policy)
        .await
        .expect("m-a 与 m-b 都不是 Unavailable，该有输出");

    let seen = policy.captured_availability();
    // **非空锚**：没有它，下面那个循环在空记录上整条空真——一个「根本不调策略」的实现全绿。
    assert!(
        !seen.is_empty(),
        "策略一次都没收到请求，说明请求根本没走到 rank（下面那条断言会空过）"
    );
    for availability in &seen {
        let asked: BTreeSet<String> = availability
            .iter()
            .map(|(id, _)| id.as_str().to_owned())
            .collect();
        assert_eq!(
            asked, candidate_ids,
            "交给 rank 的 availability 的 id 集合该等于候选集的 id 集合（不是只断 len）：\
             候选 {candidate_ids:?}，请求里 {asked:?}"
        );
    }
}

/// 探到的健康度**真的通到排序**（设计 §11）：`Unavailable` 的模型不被选中；**两侧对钉**。
///
/// # 为什么两向都要
///
/// 只钉向一时，一个「永远返回空候选集」的实现**全绿**：向一断的是「m-a 不在输出里」，
/// 而那与「输出永远是空的」在读数上一样。**向二就是那条守卫**——改回 `Healthy` 之后它
/// **回到输出，且成为 `selected()`**（不只是「出现了」）。
///
/// # 两向为什么在同一条用例里
///
/// 两向在**同一个 id**上做：向一里它是 `Unavailable`、向二里它是 `Healthy`，
/// 别的一切不变。这样才能把红归因到「健康度」这一样上，而不是「换了一组候选」。
///
/// **排序为什么落到 id 升序**：本策略的打分是常数、family 都记 `SameFamily`，故缺省的
/// `compare` 前四档全平，由第四档（`ModelId` 升序）定序——`m-a` 因此排在 `m-b` 前面。
/// 这条依赖是缺省比较口径的，不是本用例发明的。
///
/// **红的条件**：向一（档位：**移除**）把 `availability` 一律填 `Healthy`（不取快照）
/// → m-a 不被滤掉且成为 selected()，向一红。**向二挡的是相反方向**：一个把探到的
/// `Healthy` 也丢掉的实现让向二红。
#[tokio::test]
async fn a_probed_health_actually_reaches_the_ranking() {
    // 向一：m-a 探到 Unavailable → 不被选中；m-b 仍是可执行的候选。
    let (_dir, db) = a_model_db();
    let tx = db.begin().unwrap();
    let mut registry = ProviderRegistry::new();
    seed_routable(&tx, &mut registry, "m-a", ProviderHealth::Unavailable);
    seed_routable(&tx, &mut registry, "m-b", ProviderHealth::Healthy);

    let candidates = plan_candidates(&tx, &registry).expect("健康度不是闸门，两条都该在候选集里");
    let plan = select(candidates, a_route_input(no_constraints()), &RecordingPolicy::new())
        .await
        .expect("m-b 可执行，该有输出");

    assert_eq!(
        plan.selected().0.model().as_str(),
        "m-b",
        "探到 Unavailable 的 m-a 该被 rank 滤掉，选中的该是 m-b"
    );
    assert!(
        plan.alternatives().is_empty(),
        "滤掉之后只剩一条，表尾该是空的"
    );

    // 向二：同一个 id 改回 Healthy → 它回到输出，且成为 selected()。
    let (_dir2, db2) = a_model_db();
    let tx2 = db2.begin().unwrap();
    let mut registry2 = ProviderRegistry::new();
    seed_routable(&tx2, &mut registry2, "m-a", ProviderHealth::Healthy);
    seed_routable(&tx2, &mut registry2, "m-b", ProviderHealth::Healthy);

    let candidates2 = plan_candidates(&tx2, &registry2).expect("两条都该在");
    let plan2 = select(candidates2, a_route_input(no_constraints()), &RecordingPolicy::new())
        .await
        .expect("两条都可执行，该有输出");

    assert_eq!(
        plan2.selected().0.model().as_str(),
        "m-a",
        "改回 Healthy 之后它该回到输出，且成为 selected()（不是只出现在表尾）"
    );
    let mut ids = vec![plan2.selected().0.model().as_str().to_owned()];
    ids.extend(
        plan2
            .alternatives()
            .iter()
            .map(|(candidate, _)| candidate.model().as_str().to_owned()),
    );
    assert_eq!(
        ids,
        vec!["m-a".to_owned(), "m-b".to_owned()],
        "两条都在，且按 id 升序（缺省 compare 的第四档）"
    );
}

/// G 对 `BudgetView` 的**全部动作是搬运**（设计 §8.1、§8.2）：五个 `Option<i64>` 原样到达策略。
///
/// # 两向各自钉什么，以及为什么缺一不可
///
/// - **向一**：传 `budget.money = None` → 请求里 `money` **仍是 `None`**；
/// - **向二**：传 `budget.token = Some(0)` → 它**仍是 `Some(0)`**。
///
/// **fail-open 的那一侧是向二的反面**：把 `Some(0)` 读成 `None` 就是「**把额度为零读成
/// 不构成约束**」——路由会在额度耗尽时照常花钱，而它在结果上**看不出来**（`rank` 的
/// 输出一个字都不变）。反向的 `None → Some(0)` 是 fail-closed（凭空没有候选），错得刺眼。
/// **故两向都要**：只写向一时，向二那一档变异**不红**。
///
/// **两向为什么在同一个测试体内、同一个库上**：两次 `select` 之间只改了预算这一样，
/// 红的归因因此唯一。`select` 按值收候选集，故第二次要重新 `plan_candidates`
/// ——那是**同一个库上的同一次构造**，不是另建一个形状不同的夹具。
///
/// **红的条件**：向一（档位：**放宽**）把 `None` 折成 `Some(0)` → 向一红；
/// 向二（档位：**收紧**）把 `Some(0)` 当「没有约束」折成 `None` → 向二红。
#[tokio::test]
async fn the_budget_reaches_the_policy_verbatim_both_ways() {
    let (_dir, db) = a_model_db();
    let tx = db.begin().unwrap();
    let mut registry = ProviderRegistry::new();
    seed_routable(&tx, &mut registry, "m-a", ProviderHealth::Healthy);
    let policy = RecordingPolicy::new();

    // 向一：money = None（该量纲当前不构成约束）。
    let candidates = plan_candidates(&tx, &registry).expect("候选在");
    let _ = select(
        candidates,
        a_route_input(BudgetView {
            money: None,
            wall_time: None,
            token: Some(7),
            gpu_time: None,
            network_transfer: None,
        }),
        &policy,
    )
    .await
    .expect("候选可执行，该有输出");

    // 向二：token = Some(0)（额度为零，**不是**「没有约束」）。
    let candidates2 = plan_candidates(&tx, &registry).expect("候选在");
    let _ = select(
        candidates2,
        a_route_input(BudgetView {
            money: Some(3),
            wall_time: None,
            token: Some(0),
            gpu_time: None,
            network_transfer: None,
        }),
        &policy,
    )
    .await
    .expect("候选可执行，该有输出");

    let seen = policy.captured();
    assert_eq!(
        seen.len(),
        2,
        "两次 select 各该抄下一份预算记录（0 份意味着策略根本没有收到请求）"
    );
    assert_eq!(
        seen[0].money, None,
        "向一：送进去的 None 该原样到达策略——折成 Some(0) 就是把「不构成约束」读成「额度为零」"
    );
    assert_eq!(
        seen[1].token,
        Some(0),
        "向二：送进去的 Some(0) 该原样到达策略——折成 None 就是「把额度为零读成不构成约束」，\
         而那是 fail-open 的那一侧"
    );
}

/// **句柄与候选按 id 配对**（设计 §3.1 第 6 条、§11）：三个候选、三个**互不相同**的适配器，
/// 逐项断言候选自己那条 id 与句柄所服务的那个模型是同一个。
///
/// # 这条是本 task 新增类型的唯一承重用例
///
/// 没有它，一个「句柄随便给一个」的实现——按 id 发起调用时会打到**另一个模型**——
/// 会全绿，而那是这条路径上最坏的一种错（调用打给了错的模型，而返回值看起来一切正常）。
///
/// **三个句柄互不相同是承重的**：句柄在两条候选之间共用时，「配错」在那两条之间不可观察
/// （等价变异体），故夹具刻意一 id 一个适配器。
///
/// **为什么用 [`ReversingPolicy`]**：本用例的红条件是「把句柄按**输入次序**配回去」。
/// 输入次序是 id 升序，而缺省 `compare` 在打平时也按 id 升序——**两者同序时错配不可观察**。
/// 反序策略让 D 的输出与输入次序相反，错配才落得到红。
///
/// **红的条件（档位：取反）**：把句柄表按候选集的输入次序配回去（而不是按 id）→ 红。
#[tokio::test]
async fn the_result_pairs_every_candidate_with_its_own_adapter() {
    let (_dir, db) = a_model_db();
    let tx = db.begin().unwrap();
    let mut registry = ProviderRegistry::new();
    let a = seed_routable(&tx, &mut registry, "m-a", ProviderHealth::Healthy);
    let b = seed_routable(&tx, &mut registry, "m-b", ProviderHealth::Healthy);
    let c = seed_routable(&tx, &mut registry, "m-c", ProviderHealth::Healthy);

    let candidates = plan_candidates(&tx, &registry).expect("三个候选都在");
    let plan = select(candidates, a_route_input(no_constraints()), &ReversingPolicy)
        .await
        .expect("三条都可执行，该有输出");

    // 该模型自己的那个句柄——按**夹具的构造**给出（不是从被测实现里读回来的）。
    let expected = |id: &str| -> Arc<dyn ModelProvider> {
        match id {
            "m-a" => a.clone(),
            "m-b" => b.clone(),
            "m-c" => c.clone(),
            other => panic!("夹具里没有 {other} 这个模型"),
        }
    };

    let (selected, adapter) = plan.selected();
    assert!(
        Arc::ptr_eq(adapter, &expected(selected.model().as_str())),
        "选中的那对不是同一个模型的那一对：候选是 {}，句柄服务的却是别的模型",
        selected.model().as_str()
    );

    let alternatives = plan.alternatives();
    assert_eq!(alternatives.len(), 2, "三个候选里表头之外该有两条表尾");
    for (candidate, adapter) in &alternatives {
        assert!(
            Arc::ptr_eq(adapter, &expected(candidate.model().as_str())),
            "表尾里的配对错了：候选是 {}，句柄服务的却是别的模型",
            candidate.model().as_str()
        );
    }

    // **覆盖完整**：三对恰好覆盖那三个模型，一个不多一个不少——只断言「每一对都配得上」
    // 时，一个丢掉一条候选的实现（例如把两条配到同一个句柄上）仍可全绿。
    let mut ids = vec![selected.model().as_str().to_owned()];
    ids.extend(
        alternatives
            .iter()
            .map(|(candidate, _)| candidate.model().as_str().to_owned()),
    );
    ids.sort();
    assert_eq!(
        ids,
        vec!["m-a".to_owned(), "m-b".to_owned(), "m-c".to_owned()],
        "三对该恰好覆盖三个候选"
    );
}

/// `CallPlan::alternatives()` 是**同一份有序列表的表尾**，次序与 D 的输出相同
/// （**G 不重新排序、不重新解析**——设计 §4.6：G 在排序这件事上不做第二份判断）。
///
/// **为什么用 [`ReversingPolicy`]**：本用例的红条件是「G 对 alternatives 再排一次序」
/// （例如按候选集的输入次序重排）。输入次序与缺省 `compare` 的兜底档同序，故**必须**让
/// D 的输出与输入反序，那枚变异体才落得到红。下面的 `input_ids` 那一条断言就把这件事写明了：
/// **输入是升序、输出是降序**，两者不同序，故本用例不是空转。
///
/// **红的条件（档位：放宽）**：在 G 里对 alternatives 再排一次序（或按输入次序给出）
/// → 红。
#[tokio::test]
async fn alternatives_is_the_tail_of_the_same_list() {
    let (_dir, db) = a_model_db();
    let tx = db.begin().unwrap();
    let mut registry = ProviderRegistry::new();
    seed_routable(&tx, &mut registry, "m-a", ProviderHealth::Healthy);
    seed_routable(&tx, &mut registry, "m-b", ProviderHealth::Healthy);
    seed_routable(&tx, &mut registry, "m-c", ProviderHealth::Healthy);

    let candidates = plan_candidates(&tx, &registry).expect("三个候选都在");
    let input_ids = ids_of(&candidates);
    let plan = select(candidates, a_route_input(no_constraints()), &ReversingPolicy)
        .await
        .expect("三条都可执行，该有输出");

    let mut ids = vec![plan.selected().0.model().as_str().to_owned()];
    ids.extend(
        plan.alternatives()
            .iter()
            .map(|(candidate, _)| candidate.model().as_str().to_owned()),
    );

    assert_eq!(
        input_ids,
        vec!["m-a".to_owned(), "m-b".to_owned(), "m-c".to_owned()],
        "夹具的输入次序该是 id 升序（`list_registered` 是 ORDER BY id ASC）——\
         若它与输出同序，下面那条断言对「再排一次序」就没有效力"
    );
    assert_eq!(
        ids,
        vec!["m-c".to_owned(), "m-b".to_owned(), "m-a".to_owned()],
        "selected() 与 alternatives() 拼起来该恰是 D 那份有序列表（次序原样，不重排）"
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

// ===== Task 8：`call`——装配请求、发起一次调用、截止的两侧 =====

/// 播下若干条可路由的候选、各配一个**自己的**假适配器，选一次，把 `CallPlan` 与
/// 「id → 那枚 `FakeModel`」的句柄表一起交回。
///
/// **`call` 的两个入参直接从 `plan.selected()` 取**（候选与它自己的句柄是同一次配好的，
/// 设计 §3.1 第 1、6 条），而**观测端**（适配器收到了什么、被调了几次）在那枚 `FakeModel`
/// 上，故句柄表要一起带出来。
///
/// **`TempDir` 与 `Db` 不返回**：规划完就不再需要它们——`select` 不读库（设计 §3.4），
/// 而 `CallPlan` 不持有任何来自库的借用。**这不是遗漏**：它们在本函数返回前就 drop 干净了。
///
/// **一条 id 一个适配器**（不是共用一个）：句柄在两条候选之间共用时，
/// 「配错了哪一条」在那一对上不可观察（等价变异体）。
async fn a_plan_over(
    adapters: Vec<(&str, FakeModel)>,
    // `select` 收的就是这个形态（它为什么带 `Sync`，写在那条签名的文档里）；
    // 这里照抄，**不擦成 `&dyn RankingPolicy`**——擦掉的话本函数就调不动 `select` 了。
    policy: &(dyn RankingPolicy + Sync),
) -> (CallPlan, Vec<(String, Arc<FakeModel>)>) {
    let (_dir, db) = a_model_db();
    let tx = db.begin().unwrap();
    let mut registry = ProviderRegistry::new();
    let mut handles = Vec::new();
    for (id, adapter) in adapters {
        register_with_state(&tx, id, LifecycleState::Active);
        insert_profile(&tx, id);
        let handle = Arc::new(adapter);
        serve(&mut registry, &[id], Arc::clone(&handle));
        handles.push((id.to_owned(), handle));
    }

    let candidates = plan_candidates(&tx, &registry).expect("播下的每一条都该是可路由的候选");
    drop(tx);
    let plan = select(candidates, a_route_input(no_constraints()), policy)
        .await
        .expect("候选集非空，该有输出");
    (plan, handles)
}

/// 从句柄表里按 id 取那枚 `FakeModel`（**夹具的**那一枚，不是从被测实现里读回来的）。
fn handle_of(handles: &[(String, Arc<FakeModel>)], id: &str) -> Arc<FakeModel> {
    handles
        .iter()
        .find(|(known, _)| known == id)
        .map(|(_, handle)| Arc::clone(handle))
        .unwrap_or_else(|| panic!("夹具里没有 {id} 这一条"))
}

/// 一枚最小的对话载荷，供 `CallInput` 用。
fn a_dialog() -> Vec<Message> {
    vec![Message {
        role: Role::User,
        content: "ping".into(),
    }]
}

/// 适配器回的那一枚响应。`ReplyAfter` 要**按值**收一枚，故用例自己给，
/// 不从夹具的默认态里抠（`FakeModel` 的默认响应是它自己的私有字段）。
fn a_reply() -> InvokeResponse {
    InvokeResponse {
        model: ModelId::new("fake-1"),
        content: "pong".into(),
        usage: Usage {
            input_tokens: 1,
            output_tokens: 1,
        },
    }
}

/// 一个 `invoke` 立刻返这一枚错误的适配器 → `call` 交回的那枚 `Err`（`Ok` 即 panic）。
///
/// **`panic!` 那一支不是装饰**：`ModelCallError` 不派生 `PartialEq`，
/// 故四臂的断言只能是 `match` + `panic!(other)`——若把 `Ok` 折进「随便返回一枚」，
/// 一个恒返 `Err` 的实现会全绿。
async fn a_call_failing_with(error: ProviderError) -> ModelCallError {
    let (plan, _handles) = a_plan_over(
        vec![("m-fail", FakeModel::new().with_invoke(InvokeOutcome::Fail(error)))],
        &RecordingPolicy::new(),
    )
    .await;
    let messages = a_dialog();
    let (candidate, adapter) = plan.selected();
    match call(
        candidate,
        adapter,
        CallInput {
            messages: &messages,
            max_tokens: None,
        },
        None,
    )
    .await
    {
        Ok(reply) => panic!("适配器返错，call 该回 Err，却拿到 {reply:?}"),
        Err(e) => e,
    }
}

/// 适配器收到的 `model` **正是该候选自己的 id**（设计 §2.2、§3.1 第 5 条）；
/// `messages` 与 `max_tokens` 逐项相符。
///
/// # 为什么这条同时是「`call` 收 `&ExecutionCandidate` 而不收 `ModelId`」的正面照片
///
/// **id 只有一处来源——候选自己。** 本用例用 [`ReversingPolicy`] 让被选中的那条
/// （`m-c`）**不是输入次序里的第一条**，故「取第一条候选的 id」「取一个固定字面量」
/// 这两枚变异体都落得到红。
///
/// **`messages` / `max_tokens` 只能来自 `CallInput`**：`ExecutionCandidate` 与
/// `Candidate` 都不带它们，故这三条断言一起把「请求是照 `CallInput` 装出来的」钉住。
///
/// **红的条件（档位：取反）**：把 `model` 取自别处（固定字面量、或输入次序的第一条）
/// → 第一条断言红。
#[tokio::test]
async fn a_successful_call_hands_the_candidate_s_model_id_to_the_adapter() {
    let (plan, handles) = a_plan_over(
        vec![
            ("m-a", FakeModel::new()),
            ("m-b", FakeModel::new()),
            ("m-c", FakeModel::new()),
        ],
        &ReversingPolicy,
    )
    .await;

    let (candidate, adapter) = plan.selected();
    assert_eq!(
        candidate.model().as_str(),
        "m-c",
        "反序策略下被选中的该是 m-c——它同时是输入次序里的第三条，\
         故「取第一条候选」那种实现会在这里露馅"
    );
    let observed = handle_of(&handles, "m-c");

    let messages = a_dialog();
    let reply = call(
        candidate,
        adapter,
        CallInput {
            messages: &messages,
            max_tokens: Some(37u32),
        },
        None,
    )
    .await
    .expect("适配器配的是默认成功态，该回 Ok");

    let received = observed.invoke_requests();
    assert_eq!(received.len(), 1, "该恰好调了适配器一次");
    let request = &received[0];
    assert_eq!(
        request.model,
        *candidate.model(),
        "适配器收到的 model 该是候选自己的 id"
    );
    assert_eq!(
        request.messages, messages,
        "messages 该逐项相符（原样搬运：不重排、不改写）"
    );
    assert_eq!(
        request.max_tokens,
        Some(37u32),
        "max_tokens 该是 CallInput 给的那一枚"
    );

    // 适配器的响应原样交回（不吞、不替换）。
    assert_eq!(reply.content, "pong", "适配器的响应该原样交回");
}

/// 四枚失败变体**各自的两半**：`class` 是哪一枚、`source` 仍是给进去的那一枚。
///
/// **四臂各写一次、不抽代表**：`into_call_error` 的类别取自 [`classify`] 的手写 `match`，
/// 每个臂能各自漂移——一条表驱动的循环会把「某一臂被改错」藏在别的臂后面
/// （与 `each_provider_error_variant_maps_to_its_class` 同一条判据）。
///
/// **每条只断它自己那一臂**：`ModelCallError` 不派生 `PartialEq`，故用 `match` + `panic!`；
/// 而错臂落到 `panic!` 而不是「断言失败」，正是本用例能区分四臂的原因。
///
/// **红的条件（档位：取反）**：把某一臂的 `class` 写反（`Protocol → Transient`）
/// → **只有那一条红**（另三臂照绿）。
#[tokio::test]
async fn each_provider_failure_keeps_its_class_and_its_source() {
    // 一、Transport → Transient。
    match a_call_failing_with(ProviderError::Transport("连接被重置".into())).await {
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

    // 二、Unavailable → Transient。
    match a_call_failing_with(ProviderError::Unavailable("上游 503".into())).await {
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

    // 三、Protocol → Permanent。
    match a_call_failing_with(ProviderError::Protocol("响应不合契约".into())).await {
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

    // 四、UnknownModel → Permanent。
    match a_call_failing_with(ProviderError::UnknownModel("登记表里的 id 对不上".into())).await {
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
}

/// 适配器报 `Cancelled` → `ModelCallError::Cancelled { source }`，**不是** `Provider`。
///
/// **它是「`call` 与 `call_stream` / `abort` 共用 [`into_call_error`]」这条设计要求的
/// 第一张照片**（设计 §6.1 的落点段）：`call` 这一侧真的走到了那张表，
/// 而不是自己映射了一遍——一处映射错（把取消记成一次 `Transient` 失败）会在这里红。
///
/// **「那一枚没有 `class` 字段」是本条里的编译期照片**（与
/// `the_error_type_carries_the_provider_error_verbatim` 里同一形态的那一行同源）：
/// 下面那枚字面量只给 `source`，`Cancelled` 一旦长出 `class` 字段就编不过。
///
/// **红的条件（档位：放宽）**：把 `Cancelled` 归进 `Provider { class: Transient, .. }`
/// → 本条的 `match` 落到 `panic!` 那一支，红。
#[tokio::test]
async fn a_cancelled_provider_error_is_not_a_failure() {
    match a_call_failing_with(ProviderError::Cancelled("调用方发起的取消".into())).await {
        ModelCallError::Cancelled { source } => assert_eq!(
            source,
            ProviderError::Cancelled("调用方发起的取消".into()),
            "source 该是给进去的那一枚，逐字回读"
        ),
        other => panic!("Cancelled 该走 Cancelled 那一枚（它不是失败），实际 {other:?}"),
    }

    // 编译期：`Cancelled` 上**没有** `class` 字段。
    let _cancelled_has_no_class_field = ModelCallError::Cancelled {
        source: ProviderError::Cancelled("调用方发起的取消".into()),
    };
}

/// 截止（设计 §3.5）**两侧对钉**：
///
/// - **向一（档位：移除）**：`invoke` **永不返回** → 带 `Some(短截止)` 的调用仍返回
///   `ModelCallError::Deadline { .. }`；
/// - **向二（档位：收紧）**：同一个适配器改成「**略早于截止返回**」→ **成功**。
///
/// **只钉向一时，一个「永远返回 `Deadline`」的实现全绿**——那正是向二要挡的。
/// 两向在同一条用例里，是为了让红归因到「截止」这**一样**上，而不是「换了一组夹具」。
///
/// **`Deadline { elapsed_ms }` 的数值不断言**：设计 §3.1 末段把口径写死为**实测耗时**
/// （且写明它今天没有消费方），断一个数值就是钉一次巧合——实测耗时在调度抖动下
/// 不等于截止值。
///
/// **向一必须配合 `timeout` 命令跑**：一个去掉 `timeout` 的实现会让这条用例**挂住**
/// ——那不是干净的红，故由外层的 `timeout` 把它变成一次有界的失败。
///
/// **不用 `tokio::time::pause()` / `advance()`**：那两个要 `tokio` 的 `test-util` feature，
/// 而多开一个 feature 只为让一条用例跑得快不值当；真时间 + 短截止已经够稳
/// （两向都不依赖真实时钟的精度，只依赖「短截止内不返回」与「短截止内返回」这两件事）。
/// **这是一处实现选择，不是判据。**
#[tokio::test]
async fn a_call_that_never_returns_hits_the_deadline() {
    // 向一：永不返回 → 截止触发。
    let (plan, _handles) = a_plan_over(
        vec![("m-slow", FakeModel::new().with_invoke(InvokeOutcome::Never))],
        &RecordingPolicy::new(),
    )
    .await;
    let messages = a_dialog();
    let (candidate, adapter) = plan.selected();
    let err = call(
        candidate,
        adapter,
        CallInput {
            messages: &messages,
            max_tokens: None,
        },
        Some(Duration::from_millis(100)),
    )
    .await
    .expect_err("永不返回的调用该以截止回，不该有响应");

    assert!(
        matches!(err, ModelCallError::Deadline { .. }),
        "永不返回 + 有截止 → Deadline，得到 {err:?}"
    );

    // 向二：略早于截止返回 → 成功（钉住截止不会无故触发）。
    let (plan2, handles2) = a_plan_over(
        vec![(
            "m-quick",
            FakeModel::new().with_invoke(InvokeOutcome::ReplyAfter(
                Duration::from_millis(20),
                a_reply(),
            )),
        )],
        &RecordingPolicy::new(),
    )
    .await;
    let messages2 = a_dialog();
    let (candidate2, adapter2) = plan2.selected();
    let reply = call(
        candidate2,
        adapter2,
        CallInput {
            messages: &messages2,
            max_tokens: None,
        },
        Some(Duration::from_millis(500)),
    )
    .await
    .expect("略早于截止返回的调用该成功——这一向挡住「无脑超时」的实现");
    assert_eq!(reply.content, "pong", "成功那一向该拿到适配器给的响应");
    assert_eq!(
        handle_of(&handles2, "m-quick").invoke_calls(),
        1,
        "适配器真的被发起过一次（成功来自适配器，不是别处编出来的）"
    );
}

/// 同一个 `CallInput` 下两次连续的 `call`，**各带同一个短截止**，两次**各自**都能完成。
///
/// **它钉的是「截止不是被消费一次的共享值」**（设计 §3.5 的「截止只包住一次调用」）：
/// 一次调用把截止用掉之后，第二次不该继承一个已经过期的截止。
///
/// **红的条件（档位：取反）**：把 `deadline` 做成一次性的（例如换算成一个共享的
/// 剩余时长、第一次调用之后第二次恒超时）→ 第二次红。
#[tokio::test]
async fn the_deadline_wraps_one_call_only() {
    let (plan, handles) = a_plan_over(
        vec![(
            "m-twice",
            FakeModel::new().with_invoke(InvokeOutcome::ReplyAfter(
                Duration::from_millis(10),
                a_reply(),
            )),
        )],
        &RecordingPolicy::new(),
    )
    .await;
    let messages = a_dialog();
    let (candidate, adapter) = plan.selected();

    // 两次各构一枚 `CallInput`：`CallInput` 不派生 `Copy`，而**要钉的正是「两次的截止
    // 一样、且各自生效」**——载荷逐字相同，差别只在「这是第二次调用」。
    let first = call(
        candidate,
        adapter,
        CallInput {
            messages: &messages,
            max_tokens: None,
        },
        Some(Duration::from_millis(500)),
    )
    .await;
    let second = call(
        candidate,
        adapter,
        CallInput {
            messages: &messages,
            max_tokens: None,
        },
        Some(Duration::from_millis(500)),
    )
    .await;

    assert!(first.is_ok(), "第一次调用该在截止内完成，得到 {:?}", first.err());
    assert!(
        second.is_ok(),
        "第二次调用该在**同一个**截止内完成（截止不是被第一次用掉的共享值），得到 {:?}",
        second.err()
    );
    assert_eq!(
        handle_of(&handles, "m-twice").invoke_calls(),
        2,
        "两次调用该各发起一次（不是被折成一次）"
    );
}

// ===== Task 9：`call_stream` 与中止入口（设计 §3.5、§8.3） =====

/// 播下**一条**候选、发起一次流式调用，交回那枚 [`ModelStream`] 与它的**两个面**：
/// 夹具的观测句柄（读计数与记录）与 `abort` 要的那枚 `Arc<dyn ModelProvider>`。
///
/// **两个面指同一个对象**：`Arc<FakeModel>` 与 `Arc<dyn ModelProvider>` 是同一枚适配器的
/// 两种写法，故「经 `abort` 发出去的那次取消」在夹具那一侧读得到。
/// **不在这里断言任何东西**：它是夹具装配，判据在各用例里。
///
/// **只播一条候选**（与 Task 8 的 `a_call_failing_with` 同取舍）：这些用例判的是
/// 「流的 `CallId` 是谁给的」「取消有没有发出去」，与「选中了哪一条候选」无关。
async fn a_stream_over(
    id: &str,
    adapter: FakeModel,
) -> (ModelStream, Arc<FakeModel>, Arc<dyn ModelProvider>) {
    let (plan, handles) = a_plan_over(vec![(id, adapter)], &RecordingPolicy::new()).await;
    let messages = a_dialog();
    let (candidate, provider) = plan.selected();
    let stream = call_stream(
        candidate,
        provider,
        CallInput {
            messages: &messages,
            max_tokens: None,
        },
        None,
    )
    .await
    .expect("夹具的默认态是「交出一枚单分片流」，故该回 Ok");
    (stream, handle_of(&handles, id), Arc::clone(provider))
}

/// 适配器收到的 `model` **正是该候选自己的 id**；`messages` 与 `max_tokens` 逐项相符。
/// **与 `call` 那条同形**（设计 §3.1 把 ④ 写成「与 ③ 同形」），故判据也同源。
///
/// # 为什么这条同时是「`call_stream` 收 `&ExecutionCandidate` 而不收 `ModelId`」的正面照片
///
/// **id 只有一处来源——候选自己。** 本用例用 [`ReversingPolicy`] 让被选中的那条
/// （`m-c`）**不是输入次序里的第一条**，故「取第一条候选的 id」「取一个固定字面量」
/// 这两枚变异体都落得到红。
///
/// **另加两条计数器断言**：`invoke_calls() == 0` 与 `stream_calls() == 1`——
/// 它们把「流式那一支走的是 `stream`」钉住。少了它们，一个**转调 `invoke`**、把响应
/// 包成单分片流的实现会全绿（那是一个真会发生的实现选择，而它绕开了 `stream`）。
///
/// **红的条件（档位：取反 / 放宽）两条，各自实测过**（2026-10-08）：
/// 1. **取反**：把 `model` 取自一枚新造的字面量 → 只红本条（`model` 那条断言）。
/// 2. **放宽**：在 `call_stream` 里多插一次 `adapter.invoke(..)` →
///    **只红本条**，落点是 `invoke_calls() == 0` 那条断言。
///
/// **一条据实记的边界**：把 `adapter.stream(..)` **整个换成** `adapter.invoke(..)`
/// 这一枚变异体**在本 task 里写不出来**——两者的返回类型不同（`InvokeResponse` /
/// `ModelStream`），要它成型得先把响应包成一枚 `Stream`，而 `Stream` 是
/// `futures_core` 的 trait、在本 crate 里**只挂了 dev 依赖**（lib 里实现不了它，
/// 而简报明写本 task 不动 `Cargo.toml`）。故上面第 2 条取的是它的**可写邻形**：
/// 只多插一次 `invoke`，不动返回类型。**这不是「写不出变异体」的层①或层②，
/// 是第三类的一半：要做到那一枚，得先动本 crate 的依赖表。**
#[tokio::test]
async fn a_stream_call_hands_the_candidate_s_model_id_to_the_adapter() {
    let (plan, handles) = a_plan_over(
        vec![
            ("m-a", FakeModel::new()),
            ("m-b", FakeModel::new()),
            ("m-c", FakeModel::new()),
        ],
        &ReversingPolicy,
    )
    .await;

    let (candidate, adapter) = plan.selected();
    assert_eq!(
        candidate.model().as_str(),
        "m-c",
        "反序策略下被选中的该是 m-c——它同时是输入次序里的第三条，\
         故「取第一条候选」那种实现会在这里露馅"
    );
    let observed = handle_of(&handles, "m-c");

    let messages = a_dialog();
    let stream = call_stream(
        candidate,
        adapter,
        CallInput {
            messages: &messages,
            max_tokens: Some(37u32),
        },
        None,
    )
    .await
    .expect("适配器配的是默认的成流态，该回 Ok");

    let received = observed.stream_requests();
    assert_eq!(received.len(), 1, "该恰好调了适配器的 `stream` 一次");
    let request = &received[0];
    assert_eq!(
        request.model,
        *candidate.model(),
        "适配器收到的 model 该是候选自己的 id"
    );
    assert_eq!(
        request.messages, messages,
        "messages 该逐项相符（原样搬运：不重排、不改写）"
    );
    assert_eq!(
        request.max_tokens,
        Some(37u32),
        "max_tokens 该是 CallInput 给的那一枚"
    );

    // 流式那一支走的是 `stream`，不是 `invoke`。
    assert_eq!(
        observed.invoke_calls(),
        0,
        "流式那一支不该走 `invoke`（一个「转调 invoke 再包成单分片流」的实现会在这里露馅）"
    );
    assert_eq!(observed.stream_calls(), 1, "该恰好调了 `stream` 一次");

    // 交回的那枚 `CallId` 是**适配器给的**那一枚（默认态是 `call-1`），不是 G 现造的一枚。
    assert_eq!(
        stream.call,
        CallId::new("call-1"),
        "默认态给的 `CallId` 该原样出现在交回的流上"
    );
}

/// 交回的 `ModelStream.call` **就是适配器在那一枚流里给的那个 `CallId`**
/// （G 不新造、不改写它）。
///
/// **夹具给一枚与默认态不同的 `CallId`**（`adapter-supplied-7`）：若实现自己
/// `CallId::new(...)` 造一枚，或照抄默认态那个字面量，两条路都在这里对不上。
///
/// **这一条是 `aborting_a_stream_calls_cancel_with_the_streams_own_call_id` 的前提**：
/// 若 `call_stream` 交回的 `CallId` 与适配器记的不是同一个，
/// 「`abort` 真的取消了那一条流」这件事就无从谈起——`abort` 会拿一枚**谁也没听说过**的
/// `CallId` 去问适配器，而适配器照样返 `Ok`（它不做校验）。**故那一条的红会晚一步、
/// 落到一个已完成的流上**，这里先把它挡住。
///
/// **红的条件（档位：取反）**：G 在 `call_stream` 交回之前把 `ModelStream.call` 改写成
/// 自己造的一枚 → **实测红的是两条**：本条与
/// `a_stream_call_hands_the_candidate_s_model_id_to_the_adapter`（后者钉的是默认态那枚
/// `call-1`）。**两条一起才钉得住「没被写死」**：写死成 `call-1` 的实现在本条上红、
/// 在后一条上绿；写死成 `adapter-supplied-7` 的实现在本条上绿、在后一条上红。
/// **单看本条，一枚写死 `adapter-supplied-7` 的实现是绿的**——这是本条的射程边界，据实写明。
///
/// **另一条实测**：把 `CallId` 改写成别的一枚时，**`aborting_a_stream_calls_cancel_...`
/// 那一条仍是绿的**（它拿 `stream.call` 当基准，两者一起被改写，于是自洽）——
/// 这正是本条被单列出来的原因：它是那一条的前提，而它自己的红**不会**传到那一条上。
#[tokio::test]
async fn the_stream_carries_the_call_id_the_adapter_produced() {
    let distinctive = CallId::new("adapter-supplied-7");
    let (stream, _observed, _provider) = a_stream_over(
        "m-stream",
        FakeModel::new().with_stream(StreamOutcome::Chunks {
            call: distinctive.clone(),
            chunk: Ok(StreamChunk {
                delta: "pong".into(),
                done: true,
            }),
        }),
    )
    .await;

    assert_eq!(
        stream.call, distinctive,
        "交回的 ModelStream.call 该就是适配器在那枚流里给的那一个（G 不新造、不改写）"
    );
}

/// **否定式照片的两侧之向一**（设计 §3.5）：发起一次流式调用、**丢弃** `ModelStream`，
/// 再断言假适配器记录的 `cancel` **为空**。
///
/// # 为什么这一向必须单独钉
///
/// 「取消经 `CallId` 走 `cancel()`，**不经流的 drop**」是 `ModelStream` 的文档已经写死的契约
/// （`crates/continuum-core/src/model.rs`）。一个「drop 即取消」的实现在**适配器侧什么也没做**
/// （drop 不产生任何远端动作），而它会让**向二**（经 `abort` 中止）照样绿——
/// **只钉向二等于没钉**。
///
/// **两条断言不是重复的**：`cancel_calls()` 与 `cancelled()` 是两个独立的观测端，
/// 故「调了 `cancel` 但没记下 `CallId`」这类夹具级错只在前一条上露馅。
///
/// **红的条件（档位：放宽——多做了不该做的事）**：**设计 §3.5 写的字面形态是「在
/// `ModelStream` 的 `Drop` 里调 `cancel`」，而那一枚在本 crate 里写不出来**（实测 2026-10-08）：
/// `ModelStream` 是 C 的类型，`impl Drop for ModelStream` 报
/// `E0117`（孤儿规则）＋ `E0120`（`Drop` 只能对本地类型实现）。要绕开它就得让 G 包一层
/// 自己的 `Stream` 实现，而 `Stream` 是 `futures_core` 的 trait、在本 crate 里只挂 dev 依赖
/// （lib 里实现不了；简报明写本 task 不动 `Cargo.toml`）。
/// **故本条的实测取的是它的可写邻形**：在 `call_stream` 里**建立流之后顺手取消一次**
/// → **实测红的是四条**：本条、`aborting_a_stream_calls_cancel_with_the_streams_own_call_id`、
/// `aborting_a_finished_stream_is_not_an_error`（这三条读的都是 `cancel` 的计数）与
/// `a_stream_call_hands_the_candidate_s_model_id_to_the_adapter`（那枚邻形多调了一次
/// `stream`，`stream_calls() == 1` 那条断言因此红）。
/// **本条是这四条里唯一直接读「丢弃之后 `cancel` 为空」的**，故它是那张否定式照片的落点。
#[tokio::test]
async fn dropping_a_stream_does_not_cancel() {
    let (stream, observed, _provider) = a_stream_over("m-drop", FakeModel::new()).await;

    // 丢弃它——若取消被塞进 `ModelStream` 的 `Drop`，下面两条会红。
    drop(stream);

    assert_eq!(
        observed.cancel_calls(),
        0,
        "丢弃一条流不该调 `cancel`（设计 §3.5：取消经 `CallId` 走 `cancel()`，不经流的 drop）"
    );
    assert_eq!(
        observed.cancelled(),
        Vec::<CallId>::new(),
        "`cancel` 收到过的 `CallId` 该为空——空 `Vec` 正是这张否定式照片的读数"
    );
}

/// **两侧之向二**（设计 §3.5）：经 G 的中止入口 [`abort`] 中止 → 断言
/// (a) 适配器的 `cancel` **被调用了一次**、(b) 收到的 `CallId` **== `stream.call`**。
///
/// **三个断言缺一不可**（「被调用」「恰好一次」「是那一个」）——
/// 只断「被调用」时，一个 `cancel(&CallId::new(""))` 的实现会绿。
///
/// **基准那枚 `CallId` 取自被测实现交回的值**（`stream.call`），不是夹具的常量：
/// 拿常量当基准的话，「`call_stream` 交回了一枚错的 `CallId`」会与本条一起绿。
/// 前者由 `the_stream_carries_the_call_id_the_adapter_produced` 单独挡住。
///
/// **红的条件（档位：取反）**：把 `abort` 里送出去的 `CallId` 换成一枚新造的
/// → 第三条断言红。**实测（2026-10-08）：红集只有本条一枚**——
/// `aborting_a_finished_stream_is_not_an_error` 与
/// `each_provider_failure_on_abort_keeps_its_class` 都照绿（它们不看那枚 `CallId`），
/// 故本条不是靠别的用例替它红的。
#[tokio::test]
async fn aborting_a_stream_calls_cancel_with_the_streams_own_call_id() {
    let (stream, observed, provider) = a_stream_over("m-abort", FakeModel::new()).await;
    let own = stream.call.clone();

    abort(&provider, &stream)
        .await
        .expect("夹具的 `cancel` 默认成功，该回 Ok");

    assert_eq!(observed.cancel_calls(), 1, "`abort` 该恰好调一次 `cancel`");
    let seen = observed.cancelled();
    assert_eq!(seen.len(), 1, "`cancel` 该恰好收到一枚 `CallId`");
    assert_eq!(
        seen[0], own,
        "`cancel` 收到的那枚该就是 `stream.call`——G 不新造、不改写它"
    );
}

/// 适配器对 `cancel` 返回 `Ok(())`（「这个 `CallId` 已完成」不算错）→ `abort` 返回 `Ok(())`。
///
/// 设计 §3.5：取消的语义是**幂等、尽力而为**（C §5.3 定案）——**一次取消与一次完成天然竞态**，
/// 故 G 的中止入口不把「这个 `CallId` 已完成」判成错误。
///
/// **这一条钉的是 G 侧的判定，不是适配器的行为**：假适配器返 `Ok`，
/// 而 G **不得**在它返回 `Ok` 之后自行合成一个 `Err`。
///
/// **据实记一处射程**：本夹具的 `cancel` 一律返它被配的那一枚结果——
/// 它**没有「已完成」这个可配面**，故本条实际断的是「适配器返 `Ok` 时，G 原样交回 `Ok`」。
/// 「一个**真**已完成流上的取消」在本夹具上不可表达（`StreamOutcome` 不带「这条流已经结束」）。
/// **这不是漏项，是夹具的可配面边界**：要它，得给 `FakeModel` 加一个「已完成」的态，
/// 而那要 G 侧先有一位能表达「流已完成」的输入（今天没有）。
///
/// **红的条件（档位：收紧）**：在 `abort` 里对「已完成」的 `CallId` 直接返回一个 `Err`。
/// **实测（2026-10-08）：这一枚只能写成「`Ok` 路径上也返 `Err`」**——G 手里没有任何
/// 「这条流已完成」的读数（`ModelStream` 只带 `call` 与 `chunks`，而 `chunks` 要
/// `Pin<&mut>` 才推得动，`abort` 收的是 `&ModelStream`；`cancel` 的返回值也不带状态）。
/// 实测红的是**两条**：本条与 `aborting_a_stream_calls_cancel_with_the_streams_own_call_id`。
///
/// **故本条的定位据实写清**：它**不是一条能把「已完成」与「未完成」分开的判别式**
/// （那一类判断在 G 侧今天不可表达），而是**「G 不在 `Ok` 路径上自行合成 `Err`」这条
/// 回归护栏**。**它挡得住的那一枚是实测过的**，故它不是空转的用例。
#[tokio::test]
async fn aborting_a_finished_stream_is_not_an_error() {
    let (stream, observed, provider) = a_stream_over("m-done", FakeModel::new()).await;

    let result = abort(&provider, &stream).await;
    assert!(
        result.is_ok(),
        "取消的语义是幂等、尽力而为——「已完成」不是错误，得到 {:?}",
        result.err()
    );
    assert_eq!(
        observed.cancel_calls(),
        1,
        "取消该真的发出去过一次（不是被 G 在调适配器之前就挡下了）"
    );
}

/// 一个 `cancel` 返这一枚错误的适配器 → `abort` 交回的那枚 `Err`（`Ok` 即 panic）。
///
/// 与 `a_call_failing_with` 同形、**不是它的复用**：那一个走 `call` 与 `invoke`，
/// 这一个走 `call_stream` 与 `cancel`——两处被测入口不同，故两套装配各写一次。
async fn an_abort_failing_with(error: ProviderError) -> ModelCallError {
    let (stream, _observed, provider) = a_stream_over(
        "m-abort-fail",
        FakeModel::new().with_cancel(Answer::Fail(error)),
    )
    .await;
    match abort(&provider, &stream).await {
        Ok(()) => panic!("适配器的 `cancel` 返错，`abort` 该回 Err，却拿到 Ok"),
        Err(e) => e,
    }
}

/// `abort` 的失败面**逐变体各写一次**：`class` 是哪一枚、`source` 仍是给进去的那一枚。
///
/// # 判据：**同一批失败，两个转换点**
///
/// `call` / `call_stream` / `abort` 都要经 [`into_call_error`]（设计 §6.1 的落点）。
/// **两个转换点各映射一套，正是本项目一贯判为缺陷的形状**（同一件事两个产生点）：
/// 一处改了、另一处没改，行为在两条路径上分叉，而两条路径各自看起来都「对」。
/// 故本用例把**与 `each_provider_failure_keeps_its_class_and_its_source` 同一批**的
/// 五枚 `ProviderError` 在 `abort` 这一侧再钉一遍。
///
/// # 五臂各写一次、不抽代表
///
/// 五枚**逐项**各断一次（四枚失败变体各得一个写死的类别，`Cancelled` 走**不带 `class`**
/// 的那一枚）——`into_call_error` 的类别取自 [`classify`] 的手写 `match`，每个臂能各自漂移，
/// 一条表驱动的循环会把「某一臂被改错」藏在别的臂后面。
/// **`Cancelled` 那一臂不是凑数**：少了它，一个「在 `abort` 里自己映射一遍、把取消记成
/// 一次 `Transient` 失败」的实现在本用例上全绿——而那正是本条要挡的形状。
///
/// **每条只断它自己那一臂**：`ModelCallError` 不派生 `PartialEq`，故用 `match` + `panic!`。
///
/// **红的条件（档位：放宽）**：在 `abort` 里另写一套转换（如一律 `Transient`）→
/// **实测（2026-10-08）：红集只有本用例一枚**。**另有一枚更细的（档位：替换）**：
/// 若 `abort` 只换了 `cancel` 送出去的那枚 `CallId`、而失败转换照旧，
/// **实测本用例全绿**——那一枚由 `aborting_a_stream_calls_cancel_with_the_streams_own_call_id` 挡。
#[tokio::test]
async fn each_provider_failure_on_abort_keeps_its_class() {
    // 一、Transport → Transient。
    match an_abort_failing_with(ProviderError::Transport("连接被重置".into())).await {
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

    // 二、Unavailable → Transient。
    match an_abort_failing_with(ProviderError::Unavailable("上游 503".into())).await {
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

    // 三、Protocol → Permanent。
    match an_abort_failing_with(ProviderError::Protocol("响应不合契约".into())).await {
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

    // 四、UnknownModel → Permanent。
    match an_abort_failing_with(ProviderError::UnknownModel("登记表里的 id 对不上".into())).await {
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

    // 五、`Cancelled` **不是失败**：它走不带 `class` 的那一枚（与 `call` 那一侧同判定）。
    match an_abort_failing_with(ProviderError::Cancelled("调用方发起的取消".into())).await {
        ModelCallError::Cancelled { source } => assert_eq!(
            source,
            ProviderError::Cancelled("调用方发起的取消".into()),
            "source 该是给进去的那一枚，逐字回读"
        ),
        other => panic!("Cancelled 该走 Cancelled 那一枚（它不是失败），实际 {other:?}"),
    }
}

/// 一个 `stream` 返这一枚错误的适配器 → `call_stream` 交回的那枚 `Err`（`Ok` 即 panic）。
///
/// 第三个同形装配（`a_call_failing_with` 走 `invoke`、`an_abort_failing_with` 走 `cancel`，
/// 本函数走 `stream`）：**三处被测入口不同，故三套装配各写一次**，不抽成一个收闭包的助手
/// ——抽了就得把「调哪一个方法」也参数化，那会让三条用例的红都从同一条装配路径上来。
async fn a_stream_call_failing_with(error: ProviderError) -> ModelCallError {
    let (plan, _handles) = a_plan_over(
        vec![(
            "m-stream-fail",
            FakeModel::new().with_stream(StreamOutcome::Fail(error)),
        )],
        &RecordingPolicy::new(),
    )
    .await;
    let messages = a_dialog();
    let (candidate, adapter) = plan.selected();
    match call_stream(
        candidate,
        adapter,
        CallInput {
            messages: &messages,
            max_tokens: None,
        },
        None,
    )
    .await
    {
        Ok(_stream) => {
            panic!("适配器报错、没有交出流，`call_stream` 该回 Err，却拿到 Ok")
        }
        Err(e) => e,
    }
}

/// `call_stream` 的失败面**逐变体各写一次**：`class` 是哪一枚、`source` 仍是给进去的那一枚。
///
/// # 为什么本用例不在简报列的用例表里，而仍然要写
///
/// 简报列的失败面用例只有 `abort` 那一侧（`each_provider_failure_on_abort_keeps_its_class`），
/// 判据是「**同一批失败两个转换点**」。**而转换点今天是三处**：`call`（Task 8 有照片）、
/// `call_stream`（本用例）、`abort`（Task 9 那一条）。
/// 少了本用例，`call_stream` 的失败路径**一条照片都没有**——而写在这里的
/// `src/model_call.rs` 文档会指着 `abort` 那条说「照片在那」，
/// 那是**指着一条走别的入口的用例**（本 task 的自审把它查了出来）。
/// **它同时是 `StreamOutcome::Fail` 这一可配面的第一个使用者**
/// （`tests/common/mod.rs` 那张表里这一项此前没有收件人被用上）。
///
/// # 五臂各写一次、不抽代表
///
/// 与另两条同源：`into_call_error` 的类别取自 [`classify`] 的手写 `match`，每个臂能各自漂移；
/// `Cancelled` 那一臂同样不是凑数（少了它，一个把取消记成一次 `Transient` 失败的实现会全绿）。
///
/// **红的条件（档位：放宽）**：在 `call_stream` 里另写一套转换（不用 [`into_call_error`]）
/// → **实测（2026-10-08）：红集只有本用例一枚**。
#[tokio::test]
async fn each_provider_failure_on_a_stream_call_keeps_its_class() {
    // 一、Transport → Transient。
    match a_stream_call_failing_with(ProviderError::Transport("连接被重置".into())).await {
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

    // 二、Unavailable → Transient。
    match a_stream_call_failing_with(ProviderError::Unavailable("上游 503".into())).await {
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

    // 三、Protocol → Permanent。
    match a_stream_call_failing_with(ProviderError::Protocol("响应不合契约".into())).await {
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

    // 四、UnknownModel → Permanent。
    match a_stream_call_failing_with(ProviderError::UnknownModel("登记表里的 id 对不上".into()))
        .await
    {
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

    // 五、`Cancelled` **不是失败**：它走不带 `class` 的那一枚（与另两个入口同判定）。
    match a_stream_call_failing_with(ProviderError::Cancelled("调用方发起的取消".into())).await {
        ModelCallError::Cancelled { source } => assert_eq!(
            source,
            ProviderError::Cancelled("调用方发起的取消".into()),
            "source 该是给进去的那一枚，逐字回读"
        ),
        other => panic!("Cancelled 该走 Cancelled 那一枚（它不是失败），实际 {other:?}"),
    }
}

// ===== Task 10：不调 `usage()` / `list_models()` / `describe_model()` 的否定式照片 =====
//
// 三条**否定式照片**（本节自己就是那三条判据的落点）加一条**正控制**。
// 判据是「G 的路径上这三个方法**一次都没被调过**」，读数取夹具的计数器。
//
// **本节与 Task 4 的模块面守卫**不**构成「同一件事的两个照片」，据实写明**：
// Task 10 的简报（`.superpowers/sdd/p3g-task-10-brief.md` 的第 12–14 行）写「本 task 的三条与
// Task 4 的模块面守卫互补：Task 4 钉**文本里不出现**」，而**实测**
// `crates/continuum-runtime/tests/model_call_discipline.rs` 的三张清单（21 枚 needle）
// **不含** `usage` / `list_models` / `describe_model` 任何一枚（2026-10-09 实测零命中），
// 设计 §11 的两格判据（「G 不调 `list_models()`」与「G 不调 `usage()` / `describe_model()`」）
// 也都只写「否定式照片」、没有文本那一层——那三张清单钉的是工具路径名 / 适配器名 / 能力凭据名。
// **故本节没有可互补的文本守卫**，简报那句话的来历照留在此，免得后来者照着它去找一条不存在的守卫。
//
// **本节的红条件逐条按「写不写得出来」分过层**（成例是 G 的 M-8-3 / M-9-1 / M-9-2 / M-9-3：
// 计划里写的红条件有几枚**在交付签名上写不出来**）。**逐条实测结论写在各自的用例上**。

/// G 的路径**不问适配器的用量口径**（设计 §5）：假适配器的 `usage()` 返 `Err`，正常路径照过。
///
/// # 判据（设计 §5）
///
/// `usage()` 的语义**两种读法都不成立**：
///
/// - 读作「**累计**」→ 它就成了同一事实的**第二个产生点**：累计量由逐次返回值求和可得，
///   而适配器自报的累计与客户端求和**可以不一致且无从核对**；
/// - 读作「**本会话**」→ **「会话」在 §315 里没有定义**（谁开、谁关、跨不跨进程重启，
///   规范一个字都没给）。
///
/// 故 **G 不定义它、也不用它**。G 实际依赖的是**逐次**的那一个——`InvokeResponse.usage`，
/// 与那次调用**同一物证**的量，不需要与任何「累计」对账。下面 `reply.usage` 那条断言
/// 就是这件事的正面照片：用量真的读出来了，而且是从这一次调用的返回值里读的。
///
/// # 为什么还要断 `health_calls() == 1`
///
/// 只断「计数为 0」时，一个**什么都没问**的实现也全绿（0 与「没问」在输出上不可区分）。
/// 故另断这条路径**确实问过适配器一次**，而那一次是可用性快照（`health()`），不是 `usage()`。
///
/// **红的条件（档位：移除）**：在 `call` 里加一句 `let _ = adapter.usage().await;` → 计数变 1，红。
/// **实测（2026-10-09）**：`cargo test -p continuum-runtime --test model_call --no-fail-fast`
/// 下红**两条**——本用例与共用适配器那条（后者也读 `usage_calls()`）。
/// 变异体写法与日志路径见 task-10 报告 §4（`M-A`）。
#[tokio::test]
async fn the_path_does_not_ask_for_usage() {
    let adapter = FakeModel::new().with_usage(Answer::Fail(ProviderError::Unavailable(
        "usage() 的语义未定义，G 不用它".into(),
    )));
    let (plan, handles) = a_plan_over(vec![("m-usage", adapter)], &RecordingPolicy::new()).await;
    let observed = handle_of(&handles, "m-usage");

    let messages = a_dialog();
    let (candidate, provider) = plan.selected();
    let reply = call(
        candidate,
        provider,
        CallInput {
            messages: &messages,
            max_tokens: None,
        },
        None,
    )
    .await
    .expect("G 的路径不调 usage()，故那一枚 Err 不该露头");

    assert_eq!(
        reply.usage,
        Usage {
            input_tokens: 1,
            output_tokens: 1,
        },
        "逐次用量该是这一次调用自己的返回值里那一份（设计 §5），不是问适配器要来的"
    );
    assert_eq!(
        observed.usage_calls(),
        0,
        "G 的路径上 usage() 一次都不该被调——计数非 0 说明某处把它当成了第二个用量产生点"
    );
    assert_eq!(
        observed.health_calls(),
        1,
        "这条路径确实问过适配器一次（可用性快照取的是 health()），故上面的 0 不是「什么都没问」"
    );
}

/// 候选集的来路是**登记表**，不是适配器自报的清单（设计 §3.2 末条）。
///
/// # 判据（设计 §3.2）
///
/// **登记是路由的权威、`list_*` 是描述的权威**，而 G 要的是**路由事实**。三个来源里
/// （D 的 `model_registry` 表 / C 的 `ProviderRegistry` / 适配器的 `list_models()`）
/// **第三个在这条路径上一次都不出现**。
///
/// **夹具与 Task 6 的 `the_candidate_set_comes_only_from_the_registry_table` 同一份**
/// （同一条登记行、同一个「自报表外 id」的适配器）：那条钉的是候选集的**结构**
/// （⊆ 登记表、无表外 id、两两不同），本条的独立增量是**适配器的计数**。
///
/// # 本条为什么跑到 ② 异步段（不止 ① `plan_candidates`）
///
/// 简报给本条的**红条件是「把候选集的 id 来源换成 `list_models()` → 计数变 1 且候选集多出那个 id」**，
/// 而**那一枚在交付签名上写不出来**（**层③：只能在改签名的前提下表达**）：
/// `plan_candidates` 是**同步**函数（它就是设计 §3.4 那两个同步边界之一，它持 `Tx`），
/// 而 `list_models()` 是 `async`——要把 id 来源换成它，得先把 `plan_candidates` 改成
/// `async fn`，那会连带改掉 7 处调用侧，**唯一可观察者是调用侧编译失败，而按纪律那不算红**。
/// **故实测取它的可写邻形**：在**异步段**（`snapshot` 或 `select` 里）插一次
/// `list_models().await` → 只动计数、不动类型。本条因此把 ② 也跑一遍，
/// 让那条计数断言有一个**能写出来的**红。**这不是把候选集的判据挪到 ②**：
/// `plan_candidates` 那三条断言照旧在 ① 上（同一份夹具、同一个入口）。
///
/// **红的条件（档位：放宽，可写邻形）**：在 `snapshot` 里插一句
/// `let _ = candidate.adapter.list_models().await;` → `list_models_calls()` 变 1，本条红。
///
/// **实测（2026-10-09）**：那一枚红**两条**——本用例与共用适配器那条（`M-B`）。
/// **强形（即简报给的那一枚）实测为层③**：把 `plan_candidates` 改成 `async fn` 并把 id 来源换成
/// `list_models()` 之后，`cargo test -p continuum-runtime --test model_call --no-run` 报
/// **28 个编译错误**（20 枚 `E0599`「`impl Future` 上没有 `expect`」＋ 8 枚 `E0308`），
/// **全部落在调用侧**（`tests/model_call.rs`），`src/` 一处不报。按纪律那不算红，
/// 故本条的红取上面那枚可写邻形（日志 `M-B-strong`）。
///
/// **本条跑 ② 这一半是可写红的来源**：只跑 ① 时，上面那枚邻形**只红共用适配器那一条**，
/// 本条的计数断言**一个能写出来的红都没有**——**实测（2026-10-09，隔离版）**：把 ② 那一段
/// 关掉（`if false`）后再跑 `M-B`，红集**只剩共用适配器那一条**，**本用例 `ok`**
/// （日志 `M-B-iso-sync-only`，`test result: FAILED. 38 passed; 1 failed`）。
#[tokio::test]
async fn the_path_does_not_ask_the_adapter_what_models_it_serves() {
    let (_dir, db) = a_model_db();
    let tx = db.begin().unwrap();
    let mut registry = ProviderRegistry::new();

    register_with_state(&tx, "m-served", LifecycleState::Active);
    insert_profile(&tx, "m-served");
    let adapter = Arc::new(FakeModel::new().with_list_models(Answer::Ok(vec![descriptor(
        "ghost-not-registered",
    )])));
    serve(&mut registry, &["m-served"], Arc::clone(&adapter));

    // ① 同步段：候选集只从登记表来。
    let candidates = plan_candidates(&tx, &registry).expect("登记表里有一个模型");
    drop(tx);

    let ids = ids_of(&candidates);
    // **非空锚**（与 Task 6 那条同源）：空候选集会让下面那条「不含 ghost」的断言空真。
    assert_eq!(
        ids,
        vec!["m-served".to_owned()],
        "候选集该恰是登记表里那一条；空集说明这个实现根本没在造候选"
    );
    assert!(
        !ids.iter().any(|id| id == "ghost-not-registered"),
        "适配器自报而登记表里没有的 id 不该进候选集（G 不调 `list_models()`）：{ids:?}"
    );

    // ② 异步段：同一份候选集走一次 `select`——上面那条计数断言的可写红落在这一段。
    let plan = select(
        candidates,
        a_route_input(no_constraints()),
        &RecordingPolicy::new(),
    )
    .await
    .expect("候选集非空，该有输出");
    assert_eq!(
        plan.selected().0.model().as_str(),
        "m-served",
        "被选中的该是登记表里那一条，不是适配器自报的表外 id"
    );

    assert_eq!(
        adapter.list_models_calls(),
        0,
        "G 的路径上 list_models() 一次都不该被调——计数非 0 说明候选集（或别的什么）改从描述那一侧取了"
    );
}

/// 可用性快照取自 `health()`，**不是**从 `describe_model()` 的返回里推（设计 §4.3、§3.3）。
///
/// # 判据（设计 §4.3）
///
/// **描述是 `ModelDescriptor`、可用性是 `ProviderHealth`，两个方法、两个类型、两件事**：
/// `list_models()` **不返回任何健康度**，取可用性的方法只有 `health()`。C 的设计初稿把
/// 「取描述」与「取可用性」混称成一步，**已拆成两步**；本设计据此照办——G 用 `health()`
/// 取快照，**不拿第 1 步的产物当可用性**。
///
/// `health_calls() == 1` 那一断言就是这条的正面锚：快照**确实**取了，且是从 `health()` 取的。
///
/// **红的条件（档位：放宽）**：把快照的来源换成 `describe_model()` 的返回 →
/// `describe_model_calls()` 变 1、`health_calls()` 变 0，本条两处红。
/// **据实记**：那一枚的字面写法在交付签名上也写不出来（`ModelDescriptor` 变不出
/// `ProviderHealth`，两个类型之间没有转换），**可写邻形**是「照旧用 `health()`，另加一次
/// `describe_model().await`」——只动计数、不动类型。
///
/// **实测（2026-10-09）**：那一枚邻形红**两条**——本用例与共用适配器那条（`M-C`）。
#[tokio::test]
async fn the_path_does_not_ask_for_model_descriptions() {
    let adapter =
        FakeModel::new().with_describe_model(Answer::Fail(ProviderError::Unavailable(
            "描述不是可用性，G 不拿它推健康度".into(),
        )));
    let (plan, handles) = a_plan_over(vec![("m-desc", adapter)], &RecordingPolicy::new()).await;
    let observed = handle_of(&handles, "m-desc");

    let messages = a_dialog();
    let (candidate, provider) = plan.selected();
    call(
        candidate,
        provider,
        CallInput {
            messages: &messages,
            max_tokens: None,
        },
        None,
    )
    .await
    .expect("G 的路径不调 describe_model()，故那一枚 Err 不该露头");

    assert_eq!(
        observed.describe_model_calls(),
        0,
        "G 的路径上 describe_model() 一次都不该被调——描述是 ModelDescriptor，可用性是 ProviderHealth"
    );
    assert_eq!(
        observed.health_calls(),
        1,
        "快照该恰好取一次，且取自 health()——两条一起才把「可用性不是从描述里推的」钉住"
    );
}

/// **本节四条的头一条正控制**：三条否定式照片钉在**同一个适配器**上，
/// 而它在**同一次运行里既跑通成功路径、又跑通失败路径**。
///
/// # 三个方法**同时**配成返 `Err`（设计 §11 的「三条共用一个假适配器」）
///
/// 三个 `Err` 内容各不相同，故若要归因，读得出是哪一处露的头。
///
/// # 它的独立增量：失败路径的类别是**它自己的那一枚**
///
/// 「计数为 0」这件事若被读成「这条路径整个坏掉了」，那三条否定式照片就都没意义了。
/// 上面 1、3 两条各自带 `expect(… 该回 Ok)`，本身已经挡掉「恒错实现」；
/// **本条 1、2、3 都拍不到的是**：一条**失败**路径的类别**不受那三个 `Err` 影响**
/// ——成功的那条 `Ok`、失败的那条走 `Provider { class: Permanent }`（`Protocol` 自己的类别），
/// 三个否定方法的 `Err` 一个都没渗进去。
///
/// **红的条件（档位：取反）**：让三个 `Err` 中的任何一个污染路径，例如把
/// `snapshot` 里的 `candidate.adapter.health().await` 换成「`describe_model` 成功即 `Healthy`、
/// 失败即 `Unavailable`」→ `describe_model` 那一枚 `Err` 把候选打成 `Unavailable`，
/// `select` 转而报 `NoEligibleCandidate`，`a_plan_over` 的 `expect` 先炸，红。
///
/// **实测（2026-10-09）**：那一枚红**七条**（逐条见日志 `M-D`）——
/// 本节的三条（本用例、`the_path_does_not_ask_for_usage`、`the_path_does_not_ask_for_model_descriptions`，
/// 它们都断 `health_calls()` / `describe_model_calls()`）＋
/// Task 3 的三条（`one_probe_per_candidate`、`the_snapshot_is_indexed_by_the_candidates_own_model_id`、
/// `a_unavailable_adapter_is_carried_through_verbatim`）＋
/// Task 7 的一条（`a_probed_health_actually_reaches_the_ranking`）。
/// **这一枚不隔离**——它改的是快照的来源，而快照是 Task 3 起五条用例的被测面。**据实报**：
/// 不换更窄的变异体，因为「三个 `Err` 里的哪一个污染路径」在实现上就落在这里。
#[tokio::test]
async fn the_three_negative_methods_share_one_adapter_and_the_positive_path_still_works() {
    let adapter = FakeModel::new()
        .with_usage(Answer::Fail(ProviderError::Unavailable(
            "usage() 无定义".into(),
        )))
        .with_list_models(Answer::Fail(ProviderError::Unavailable(
            "清单是描述那一侧的权威".into(),
        )))
        .with_describe_model(Answer::Fail(ProviderError::Unavailable(
            "描述不是可用性".into(),
        )))
        // 失败路径**不复用**那三个 `Err`：它是这条路径自己的失败面（`stream` 那一支），
        // 配置一个 G 没有理由去问的类别（`Protocol` → `Permanent`），
        // 好让「类别是它自己的那一枚」这句话在读数上分得开。
        .with_stream(StreamOutcome::Fail(ProviderError::Protocol(
            "响应不合契约".into(),
        )));

    let (plan, handles) = a_plan_over(vec![("m-shared", adapter)], &RecordingPolicy::new()).await;
    let observed = handle_of(&handles, "m-shared");
    let messages = a_dialog();

    // 成功路径：`call` → `Ok`。
    let (candidate, provider) = plan.selected();
    let reply = call(
        candidate,
        provider,
        CallInput {
            messages: &messages,
            max_tokens: None,
        },
        None,
    )
    .await
    .expect("三个否定方法都返 Err，而成功路径不该受它们影响");
    assert_eq!(
        reply.content, "pong",
        "成功路径该交出适配器配的那一枚响应，逐字回读"
    );

    // 失败路径：`call_stream` → 它自己那个类别。
    let (candidate, provider) = plan.selected();
    match call_stream(
        candidate,
        provider,
        CallInput {
            messages: &messages,
            max_tokens: None,
        },
        None,
    )
    .await
    {
        Ok(_) => panic!("`stream` 配的是 Fail，该回 Err"),
        Err(ModelCallError::Provider { class, source }) => {
            assert_eq!(
                class,
                FailureClass::Permanent,
                "`Protocol` 自己的类别是 Permanent，三个否定方法的 Err 一个都没渗进来"
            );
            assert_eq!(
                source,
                ProviderError::Protocol("响应不合契约".into()),
                "source 该是 `stream` 配的那一枚，逐字回读"
            );
        }
        Err(other) => panic!("该走 Provider 那一枚，实际 {other:?}"),
    }

    // 三个否定方法的计数：整条路径（成功与失败各一次）一次都没调过它们。
    assert_eq!(
        observed.usage_calls(),
        0,
        "usage() 一次都没被调——三个 Err 里任何一个污染路径都会先在这里露头"
    );
    assert_eq!(
        observed.list_models_calls(),
        0,
        "list_models() 一次都没被调"
    );
    assert_eq!(
        observed.describe_model_calls(),
        0,
        "describe_model() 一次都没被调"
    );
    // 两条路径**确实各做了一件事**，故上面那三个 0 不是「这条路径什么都没干」。
    assert_eq!(
        observed.health_calls(),
        1,
        "两条路径共用一次规划，故快照只取一次"
    );
    assert_eq!(observed.invoke_calls(), 1, "成功路径该恰好发起一次 invoke");
    assert_eq!(observed.stream_calls(), 1, "失败路径该恰好发起一次 stream");
}
