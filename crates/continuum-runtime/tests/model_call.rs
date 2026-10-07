//! G 这一侧的失败面：分类表与 `ModelCallError` 的照片（设计 §6.1、§11），
//! 以及**假模型适配器夹具自身的用例**（Task 2）。
//!
//! **本文件到本 task 为止只有这两部分**：四段流程（取快照、调排序、`call`、`call_stream`）
//! 的用例由后续 task 追补。

mod common;

use common::{descriptor, Answer, FakeModel};
use continuum_core::model::{CallId, InvokeRequest, Message, ModelId, ProviderHealth, Role};
use continuum_core::ProviderError;
use continuum_graph::failure::FailureClass;
use continuum_graph::p1_graph_migrations;
use continuum_model_registry::{
    list_registered, p3d_model_migrations, BudgetView, CandidateScore, FamilyPreference,
    FamilyRelation, LifecycleState, RankingPolicy, Ratio, RoutableModel, RoutingError,
    RoutingReason, RoutingRequest, SkillDimension, TaskSkillRequirement,
};
use continuum_persist::{builtin_migrations, Db, PersistError, Tx, Value};
use continuum_provider::model::ModelProvider;
use continuum_provider::ProviderRegistry;
use continuum_runtime::model_call::{classify, into_call_error, plan_candidates, snapshot, Candidate};
use continuum_runtime::ModelCallError;
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

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
/// **故本条的两条结构断言没有变异体照片**，成因是构造性的两条：
/// (i) 任何候选都得是一枚 `RoutableModel`，它唯一的构造点 `try_new` 要一枚 `ModelProfile`，
/// 而画像只能从 `model_profile` 表读出、那张表的 `id` 又是 **`model_registry(id)` 的外键**；
/// (ii) `ProviderRegistry` 的公开面只有按 id 查（`model_for`），**没有枚举**，故「从注册表造
/// 候选」这条路走不到。即「候选 ⊆ 登记表」与「id 两两不同」在本仓是 **D 的表结构 ＋ crate
/// 边界**兜住的，不是 G 这段代码兜住的。**本条的价值是回归护栏**：将来若有人给候选集另开一条
/// 来源（或 D 撤掉那条外键），它会红。
///
/// **本条唯一有变异体的一侧是 `.expect("登记表里有一个模型")`**：候选集恒空那一枚（`M4`）
/// 让它红。
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
/// (a) 一个模型都没登记；(b) 登记了、也有适配器，但**全部**被闸门挡下。
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
/// **恒真**的断言（外加一条 `dead_code` 警告），故不写。**该半的照片归 Task 7**（`select` 落地处）。
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

    // 来路 (b)：登记了、也有适配器，但全部被闸门挡下（都没有画像）。
    let (_dir2, db2) = a_model_db();
    let tx2 = db2.begin().unwrap();
    let mut registry2 = ProviderRegistry::new();
    register_with_state(&tx2, "m-bare", LifecycleState::Active);
    serve(&mut registry2, &["m-bare"], Arc::new(FakeModel::new()));
    let err2 = match plan_candidates(&tx2, &registry2) {
        Ok(candidates) => panic!(
            "全部被闸门挡下，该以 NoEligibleCandidate 回，却得到 {} 条候选",
            candidates.len()
        ),
        Err(e) => e,
    };
    assert!(
        matches!(
            err2,
            ModelCallError::Routing(RoutingError::NoEligibleCandidate)
        ),
        "全部被闸门挡下 → 同一枚 NoEligibleCandidate（两种情形走同一条失败路径），得到 {err2:?}"
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
/// **红的条件（档位：放宽）**：在快照里就把 `Unavailable` 滤掉（丢掉该候选）
/// → 本条向一（快照里找不到 m-u）与 `the_snapshot_is_as_long_as_the_candidate_set` **同时**红。
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
