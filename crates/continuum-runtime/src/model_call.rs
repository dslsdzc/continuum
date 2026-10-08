//! 模型调用路径（设计 `docs/superpowers/specs/2026-10-06-p3g-model-calling-path-design.md`）
//! 里**不依赖四段流程**的那部分：`ProviderError` 的分类表，以及它到
//! [`ModelCallError`] 的转换。
//!
//! **本模块最早落地的就是上面那两样**。**四段流程里已落地的是第①段（[`plan_candidates`]，
//! 同步段）、第②段（[`snapshot`] 与 [`select`]，异步段）与第③段（[`call`]）**；
//! `call_stream` 与中止入口由后续 task 落在这里。
//!
//! [`ModelCallError`] 的定义在 [`crate::error`]——那是设计 §10.2 指定的落点
//! （该文件由 F 创建，G 只增补），与把分类表放在这里并不冲突：**表是 G 的增量，
//! 而那个文件是两个子项目共写的**。

use std::sync::Arc;
use std::time::{Duration, Instant};

use continuum_core::model::{InvokeRequest, InvokeResponse, Message, ModelId, ProviderHealth};
use continuum_core::ProviderError;
use continuum_graph::failure::FailureClass;
use continuum_model_registry::{
    list_registered, load_profile, rank, BudgetView, ExecutionCandidate, FamilyPreference,
    RankedExecutionCandidates, RankingPolicy, RoutableModel, RoutingError, RoutingRequest,
    TaskSkillRequirement,
};
use continuum_persist::Tx;
use continuum_provider::model::ModelProvider;
use continuum_provider::ProviderRegistry;

use crate::error::ModelCallError;

/// 一个候选：**D 的闸门产物**与它对应的**适配器句柄**，同一次定下。
///
/// **解析只发生一次**（设计 §3.1 第 1 条）：句柄由 [`plan_candidates`] 里的
/// `registry.model_for(id)` 得到；若在发起调用时再解析一次，那就是同一件事的第二个产生点。
///
/// **字段私有、构造点唯一**（就在 [`plan_candidates`] 里）：本 crate 外**造不出一枚**
/// `Candidate`，故集成测试里的候选只能由 `plan_candidates` 产出。**这不是一条为测试方便的
/// 口子——它就是设计 §3.1 第 1 条的落点**——它同时挡掉了「给测试开一条 `#[cfg(test)]` 构造口」
/// 那条替代（那会在测试与生产之间开第二条构造通道）。
///
/// **不派生任何东西**：没有当场消费方（判据同 D 的 `BudgetView`，D §6.1）。
pub struct Candidate {
    model: RoutableModel,
    adapter: Arc<dyn ModelProvider>,
}

impl Candidate {
    /// 这份**已过闸门**的画像与状态。
    pub fn model(&self) -> &RoutableModel {
        &self.model
    }

    /// 服务这个模型的适配器句柄。它与 [`Candidate::model`] **同一次**定下（设计 §3.1 第 1 条）。
    pub fn adapter(&self) -> &Arc<dyn ModelProvider> {
        &self.adapter
    }
}

/// ① 同步段：读 D 的登记表、过 D 的闸门、解析适配器。**收 `Tx`**——
/// 它是 G 全模块**唯一**收 `Tx` 的入口（设计 §3.4）。
///
/// 候选集**只从 D 的 `model_registry` 出发**（设计 §3.2）：登记的每一行，三件事的合取——
/// (a) 在表里（由 `list_registered` 给）；(b) `load_profile` 有画像且
/// [`RoutableModel::try_new`] 过闸门；(c) `registry.model_for(id)` 命中。
/// **任一不成立即丢弃这一条，不报错。**
///
/// **G 不自己再判一次闸门**（(b) 由 `try_new` 判）、**G 不调 `list_models()`**
/// （登记是路由的权威，`list_*` 是描述的权威）。
///
/// # 丢弃是 fail-closed 的方向
///
/// 一个没有适配器的模型**定义上不可执行**，它比 `Unavailable` 更硬（设计 §3.2 的判据与
/// D 对 `availability` 缺席的处置同源：不确定就不放行，差别只在形式——D 拒绝，G 丢弃）。
/// **丢弃是显式的、有照片的**：`tests/model_call.rs` 的两侧对钉
/// （`a_model_kept_out_for_want_of_an_adapter_comes_back_once_it_is_registered`）。
///
/// # 三条判定的次序没有判据
///
/// 合取的三条谁先谁后，**设计未给判据**，本实现取 (a) → (b) → (c) 并写在这里。
/// **没有一条用例依赖它**：所有用例**只让一个条件为假**，故不构造两个条件同时成立的输入，
/// 换一个次序不会让哪条用例变绿或变红（G 的计划在 Task 6 的「刻意不写的用例」里写死了这条）。
/// 次序在一处**是可观察的**（一个画像列有表外取值的登记项，先查哪一条决定它被丢弃还是报
/// `Storage`）——正因为可观察而判据不存在，用例才不该踩上去。
///
/// # 候选集为空
///
/// → `Err(ModelCallError::Routing(RoutingError::NoEligibleCandidate))`，**不调 `rank`**
/// （设计 §3.1 第 9 条）。**「不调 `rank`」在这一层是类型上的事实**：本函数的参数表里
/// 没有 `&dyn RankingPolicy`（G 这一侧调 `rank` 的地方只有 `select`），故它拿不出一次
/// `rank` 调用——这半的照片归端到端那一处（Task 11 的可达性用例，`select` 在它之前落地），
/// **不在本函数**（订正 2026-10-08：原写「归 `select` 落地处」，实测 Task 7 那一节不含这条用例）。
/// **空的判定不返回空列表**：本函数对空集返回 `Err` 而不是 `Ok(vec![])`，
/// 故「G 交出去的候选集非空」在下游是一条构造性事实（与 D 的 `rank` 不产出空列表同形）。
pub fn plan_candidates(
    tx: &Tx<'_>,
    registry: &ProviderRegistry,
) -> Result<Vec<Candidate>, ModelCallError> {
    let mut candidates = Vec::new();
    for (id, state) in list_registered(tx).map_err(ModelCallError::Storage)? {
        let Some(profile) = load_profile(tx, &id).map_err(ModelCallError::Storage)? else {
            continue;
        };
        // 闸门由 D 判，G 不自己再判一次：`try_new` 失败（`NotRoutable`）就是「不过闸门」，
        // 与「没有画像」同样丢弃这一条。
        let Ok(model) = RoutableModel::try_new(profile, state) else {
            continue;
        };
        // `model_for` 今天唯一的失败是 `NotFound`（`registry.rs` 的该函数只产生它），
        // 而它就是「这个 id 没有适配器」——丢弃这一条，不报错。**不用通配的 `Err(_)`**：
        // 通配会把将来新增的失败变体也静默吃进「丢弃」里，而「丢弃」只对「没有适配器」
        // 这一件事有判据。
        let Ok(adapter) = registry.model_for(&id) else {
            continue;
        };
        candidates.push(Candidate { model, adapter });
    }

    if candidates.is_empty() {
        return Err(ModelCallError::Routing(RoutingError::NoEligibleCandidate));
    }
    Ok(candidates)
}

/// 逐候选取一次可用性快照（设计 §3.3）：对**每个候选**调一次它所对应的适配器的 `health()`，
/// 把结果按该候选自己的 `ModelId` 记下。条目与 `candidates` **同序、逐位对应**。
///
/// # 粒度是逐候选，不是逐适配器
///
/// 同一个适配器服务多个候选时，它会被问**多次**。**这不是一条性能选择**，是设计 §4.4
/// 第 1 条「`availability` 必须覆盖候选集」的构造性保证（逐候选取 ⇒ 天然全覆盖）。
/// 照片：`tests/model_call.rs` 的 `one_probe_per_candidate`——三个候选**共用一个适配器**，
/// 计数该是 3（按句柄去重的实现给出 1）。
///
/// **是否并发探活不在本函数的约定里**：设计 §3.3 明写它「不影响可观察结果」
/// （`rank` 只按 id 查条目，对次序无判据），故它是实现选择、**没有照片**——
/// 本函数**不写一条「必须是顺序」的断言**来钉死它。
///
/// # 把适配器的健康度原样摊给它的每个模型
///
/// **同一个适配器名下某个模型单独不可用这件事，在 §315 的类型上不可表达**：
/// `health()` 挂在适配器上、无参，而 `ProviderHealth` 的三个变体没有一个是「按模型」的。
/// G 只能把一个适配器的健康度**原样摊给它服务的每个模型**（设计 §3.3，已记在 §14 第 2 条）。
///
/// # 本层不裁剪，也不发明截止
///
/// `Healthy` / `Degraded` / `Unavailable` **一律原样带出**：只过滤 `Unavailable` 是 `rank`
/// 的事，而 `Degraded` 与 `Healthy` 之间**没有判据**（设计 §4.4 第 2 条）——
/// 在这里做二次裁剪会是在 D 已写死的地方加第二个判据。
/// 照片：`a_unavailable_adapter_is_carried_through_verbatim`（两侧：`Unavailable` 不被丢、
/// `Degraded` 不被升格）。
///
/// **`health()` 没有截止，本层不发明一条**（设计 §3.3）：§315 的签名里没有超时参数，
/// 故一次挂住的探活会把整条路由挂住。要不要给探活一个截止，**规范未给判据**，
/// 本设计不发明（记在 §14 第 3 条）。本函数**不加** `Option<Duration>` 参数。
pub async fn snapshot(candidates: &[Candidate]) -> Vec<(ModelId, ProviderHealth)> {
    let mut snapshot = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        let health = candidate.adapter.health().await;
        snapshot.push((candidate.model.profile().id().clone(), health));
    }
    snapshot
}

/// ② 的三个输入值**来自别处，G 不产它们**（设计 §3.1）：`requirements` 与 `family`
/// 来自规划侧（§250 的第 1、5 项），`budget` 来自驱动从语义层 `Budget` 的投影（D §6.1）。
/// **这三个值今天的生产方都不存在**（设计 §12 第 1 条）——故所有用例的输入都由测试直接构造，
/// **钉不了「驱动真的这么传」**；本设计也不向规划侧／语义层提这条请求（它们尚未建）。
///
/// **按值收，无生命周期参数**（设计 §3.1 第 7 条）：`RoutingRequest` 要按值持有这三者，
/// 而 `TaskSkillRequirement` **没有 `Clone`**（D 计划 Task 10）——**收引用会逼出一次不可得的克隆**。
/// **不取「给 `TaskSkillRequirement` 加 `Clone`」这条替代**：那要改 D 的类型，
/// 而收益只是省一次移动（设计 §3.1 第 7 条的原话）。
///
/// **字段 `pub`、没有 accessor**：它是一份纯数据投影，唯一的生产方（驱动）在别处，
/// 加一对 `fn requirements(&self)` 只会让同一件事有两个落点（判据同 D 的 `BudgetView`）。
/// **不派生任何东西**：与 D 的请求面同一条判据——从构造到读回之间没有本 crate 的代码，
/// 故「读回来还是原值吗」这类断言必然恒真（D §5.1）。
pub struct RouteInput {
    /// §250 #1 的能力需求。来自规划侧。
    pub requirements: TaskSkillRequirement,

    /// §250 #5 的 family 偏好（§19 的封闭清单）。来自规划侧。
    pub family: FamilyPreference,

    /// ENG-005 的预算视图。来自驱动从语义层 `Budget` 的投影（D §6.1）。**G 只搬运它**
    /// （设计 §8.1）：不解释单位、不换算、不做减法——那要用一份没有单位的余量做算术。
    pub budget: BudgetView,
}

/// ② 的产物：D 的排序结果与**每个候选自己的适配器句柄**成对带出（设计 §3.1 第 1 条
/// 与第 6 条）。
///
/// # 为什么需要它
///
/// `rank` 交回的 [`ExecutionCandidate`] **只带 `ModelId`**（D §5.2 的五个访问器里没有句柄），
/// 而 `call` / `call_stream` / `abort` 都要句柄（设计 §3.1 第 4 条：`ModelStream` 自己不带句柄，
/// 故「中止一条流」除了 `CallId` 还必须知道问哪个适配器）。句柄于是必须在 ② 与 ③ 之间有地方安放。
///
/// # 配对键是 `ModelId`，**不是位置**
///
/// 按位置配（让 `adapters` 与 `ranked.candidates()` 逐位对应）要求「句柄表的次序恰好等于
/// D 排序之后的次序」，**那是 D 的排序结果的一个未经声明的假设**——而按 id 复原用的是
/// 设计 §3.1 第 6 条写死的那个键。故 `adapters` **保持构造时的次序**，查的时候按 id 找。
///
/// **唯一性的两个来源**（缺一不可，设计 §3.1 第 6 条）：(a) 候选集**逐行来自
/// `model_registry`**，`id` 是该表的 `PRIMARY KEY`，故 G 交出去的候选 id 两两不同；
/// (b) D 在 `rank` 内**另有一道判重**（`DuplicateModelCandidate`）。
/// **若少了 (a)**，按 id 配对就可能把两条候选配到同一个句柄上，而那是**静默的错误配对**。
///
/// **字段私有、构造点唯一**（就在 [`select`] 里）：本 crate 外造不出一枚 `CallPlan`。
/// **不派生任何东西**：`RankedExecutionCandidates` 与 `Arc<dyn ModelProvider>` 都不好比对，
/// 而没有当场消费方的派生一律不加。
pub struct CallPlan {
    /// D 的排序结果，原样带出。**G 不再排一次序、不做第二份判断**（设计 §4.6）。
    ranked: RankedExecutionCandidates,

    /// 候选 id → 它自己的句柄，次序是**构造时的候选集次序**（查的时候按 id，见类型文档）。
    adapters: Vec<(ModelId, Arc<dyn ModelProvider>)>,
}

impl CallPlan {
    /// 取一条候选自己的句柄。
    ///
    /// **`expect` 不会触发，理由是构造性的**：[`select`] 把候选集**同一份**解构成
    /// `models`（交给 `rank`）与 `adapters`（留在这里），而 `rank` 的输出**只从它的入参里选**，
    /// 不会凭空造出一个 id；`adapters` 覆盖解构前的每一个候选，故每条候选都查得到。
    /// 查不到即「`rank` 输出了一条不在入参里的候选」——那是 D 的缺陷，不是这里能兜的。
    fn adapter_for(&self, id: &ModelId) -> &Arc<dyn ModelProvider> {
        self.adapters
            .iter()
            .find(|(known, _)| known == id)
            .map(|(_, adapter)| adapter)
            .expect("rank 的每一条候选都出自交进去的那个候选集，故 id 必在 adapters 里")
    }

    /// §84 的 `selected_model` 与**它对应的**句柄。
    ///
    /// **两枚返回值的对应关系是配过对的、不是各取各的表头**：一起返回正是为了让调用方
    /// 拿不到「候选是这一条、句柄是另一条」的那种组合。照片是
    /// `tests/model_call.rs` 的 `the_result_pairs_every_candidate_with_its_own_adapter`。
    pub fn selected(&self) -> (&ExecutionCandidate, &Arc<dyn ModelProvider>) {
        let candidate = self.ranked.selected();
        (candidate, self.adapter_for(candidate.model()))
    }

    /// §84 的 `alternatives` 与各自的句柄，**次序与 D 的输出相同**。
    ///
    /// **不是第二份数据**：它就是 [`CallPlan::selected`] 那一份有序列表的表尾（D §5.2
    /// 对 `RankedExecutionCandidates::alternatives` 的同一处置），故这里**不重新排序、
    /// 不重新解析**——G 在排序这件事上不做第二份判断（设计 §4.6）。
    /// 照片：`tests/model_call.rs` 的 `alternatives_is_the_tail_of_the_same_list`
    /// （输入次序与 D 的输出次序不同的构造下，再排一次序就会红）。
    pub fn alternatives(&self) -> Vec<(&ExecutionCandidate, &Arc<dyn ModelProvider>)> {
        self.ranked
            .alternatives()
            .iter()
            .map(|candidate| (candidate, self.adapter_for(candidate.model())))
            .collect()
    }
}

/// ② 异步段：取可用性快照 → 组装 [`RoutingRequest`] → 调 `rank`，并把句柄与排序结果
/// **成对带出**（[`CallPlan`]）。
///
/// # 不收 `Tx`
///
/// 设计 §3.4：`Tx` 内部持一枚 `MutexGuard`，故持着它跨 `await` 的 future 不是 `Send`；
/// 且整个库是单连接 + 单 `Mutex`，跨 `await` 持有它会**把模型调用期间的整仓库访问全部挡住**。
/// 故本函数**不收 `Tx`**——这条不是风格，是那两条事实的直接后果。
///
/// # 按值收候选集，不是 `&[Candidate]`
///
/// 设计 §3.1 第 3 条：`rank` 的第二个入参是 `&[RoutableModel]`，而 `RoutableModel`
/// **字段私有、不可克隆**，`Candidate` 按值持有它——**`&[Candidate]` 变不出 `&[RoutableModel]`**。
/// 故本函数按值收，内部把每个候选**解构成**它的模型（进 `Vec<RoutableModel>`，正是 `rank`
/// 要的那个）与它的句柄（进 `CallPlan` 的配对表）。
///
/// # G 在这一段里不做任何二次裁剪
///
/// 设计 §4.4 第 2 条：`Degraded` / `Healthy` **都照原样送进 `availability`**，
/// **不因健康度做任何判断**——只过滤 `Unavailable` 是 `rank` 的事，而 `Healthy` 与 `Degraded`
/// 之间**没有判据**。在这里做二次裁剪会是在 D 已写死的地方加第二个判据。
/// **照片：Task 3 的 `a_unavailable_adapter_is_carried_through_verbatim`**——快照这一层
/// （本函数唯一产生 `availability` 的地方）原样带过 `Unavailable` 与 `Degraded`；
/// 本函数只是把快照原样装进请求，不在这之上再加一层。
///
/// # 唯一的 `Err` 来源是 D 的 `rank`
///
/// `select` 自己不产生任何新的失败：它不读库（不接 `Tx`）、不解析适配器（解析在
/// [`plan_candidates`] 里只发生一次，设计 §3.1 第 1 条）。故本函数的失败面**就是**
/// `ModelCallError::Routing`——带出 D 那一枚，不压平成一枚同名的变体。
///
/// # 候选集为空时不在这里判
///
/// 设计 §3.1 第 9 条把「空候选集 → `NoEligibleCandidate`、不调 `rank`」落在
/// [`plan_candidates`] 里（它才是候选集的产生点，且它的 `Err` 在 `rank` 之前）。
/// 本函数因此**收不到空 `Vec`**；若真收到，`rank` 会以同一枚 `NoEligibleCandidate` 返回，
/// 形状一致。
///
/// # 策略参数上为什么有 `Send + Sync`（订正 2026-10-08，Task 8 实测）
///
/// 设计 §3.1 的代码块把这个参数写成 `policy: &dyn RankingPolicy`，而**设计 §3.4 又要求
/// 异步段交出的 future 是 `Send`**（那一条的落点是 `tests/model_call_face.rs` 的
/// `the_async_segments_future_is_send`）。**两件事在 `&dyn RankingPolicy` 这个写法下不能同时成立**：
/// 本函数在 `snapshot(..).await` **之后**才用 `policy`，故那枚 `&dyn RankingPolicy` 要跨 `await`
/// 活着；而 `&T: Send` 要求 `T: Sync`，D 的 [`RankingPolicy`] **没有 `Send + Sync` 超界**
/// ——实测报 `E0277: dyn RankingPolicy cannot be shared between threads safely`。
///
/// **取「在设计要求的判据上补界」这一条**，不取另外两条：(a) 改 D 的 trait 加超界——
/// 那是别人的类型，而本条只是 G 这一侧的一个参数；(b) 不钉 `Send`——
/// 那会撤掉设计 §3.4 明写的一条判据，而驱动要 `tokio::spawn` 这些 future。
/// **代码块是示意、正文才是约束**（设计 §3.1 自己写死的口径），而 §3.4 是正文。
///
/// 具体类型（`RecordingPolicy` 之类）自动 coerce 进来，调用方**一个字都不用改**。
pub async fn select(
    candidates: Vec<Candidate>,
    input: RouteInput,
    policy: &(dyn RankingPolicy + Send + Sync),
) -> Result<CallPlan, ModelCallError> {
    // 快照是逐候选取的（设计 §3.3），故它与候选集**同一次构造**：条目集天然覆盖候选集，
    // 这正是 `UnknownAvailability` 在 G 这条路径上不可达的那条构造性事实（设计 §4.5）。
    let availability = snapshot(&candidates).await;

    // 解构：每个候选按值拆成「模型」（`rank` 要的）与「句柄」（要与 D 的输出配对带出的）。
    // 两件事在同一次遍历里做，句柄因此与模型同源——不是第二次解析。
    let mut models: Vec<RoutableModel> = Vec::with_capacity(candidates.len());
    let mut adapters: Vec<(ModelId, Arc<dyn ModelProvider>)> = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        let Candidate { model, adapter } = candidate;
        adapters.push((model.profile().id().clone(), adapter));
        models.push(model);
    }

    // 三个值原样搬进请求：本函数不解释它们、不换算、不做减法（设计 §8.1）。
    let request = RoutingRequest {
        requirements: input.requirements,
        family: input.family,
        availability,
        budget: input.budget,
    };

    let ranked = rank(&request, &models, policy).map_err(ModelCallError::Routing)?;
    Ok(CallPlan { ranked, adapters })
}

/// ③ 发起一次调用要装的东西。`messages` / `max_tokens` 就是 [`InvokeRequest`] 的另两个字段
/// （`model` **不在这里**，它来自候选，设计 §2.2）。
///
/// **字段 `pub`、没有 accessor**：它是一份纯数据投影，唯一的生产方（驱动）在别处，
/// 加一对 `fn messages(&self)` 只会让同一件事有两个落点（判据同 D 的 `BudgetView`，
/// 与 [`RouteInput`] 同形）。
///
/// **不派生任何东西**：从构造到读回之间没有本 crate 的代码，故「读回来还是原值吗」
/// 这类断言必然恒真（D §5.1）。**它也不派生 `Copy`**：两个字段都是 `Copy` 的，
/// 但**没有当场消费方**——`the_deadline_wraps_one_call_only` 两次各构一枚，
/// 而那正是要钉的形态（见 [`call`] 的「截止只包住一次调用」一节）。
pub struct CallInput<'a> {
    /// 这一次调用要送去的对话。**原样搬运**：不重排、不改写、不补一条系统消息。
    pub messages: &'a [Message],

    /// 输出上限。**`None` 就是「不设上限」**，G 不替它编一个默认值。
    pub max_tokens: Option<u32>,
}

/// ③ 异步段：对**一个候选**发起一次调用（设计 §3.1 的第三个函数）。
///
/// # 第二个参数是那枚值本身，不是它的类型名
///
/// 句柄在 ②③ 之间会被丢：`ExecutionCandidate` **只带 `ModelId`**（D §5.2 的五个访问器里
/// 没有句柄），而 `ModelStream` 也**不带句柄**（`crates/continuum-core/src/model.rs`
/// 里 `ModelStream` 只有 `call: CallId` 与 `chunks`），故发起与中止两侧都必须**另收一枚**。
/// 它是 [`CallPlan::selected`] 交出来的那一枚，**不是在这里第二次解析出来的**
/// （解析只发生在 [`plan_candidates`] 里，设计 §3.1 第 1 条）。
///
/// # 不收 `ModelId`（设计 §2.2 第 5 条）
///
/// **id 只有一处来源——[`ExecutionCandidate::model`]。** 另收一枚 `ModelId` 会开出让
/// 「请求里的模型」与「候选」各说各话的第二条通道，而那一类错（调用打给了错的模型、
/// 返回值一切正常）是本路径上最坏的一种。照片：
/// `tests/model_call.rs` 的 `a_successful_call_hands_the_candidate_s_model_id_to_the_adapter`。
///
/// # 不收 `Tx`
///
/// 设计 §3.4：`Tx` 持一枚 `MutexGuard`，跨 `await` 持有它会把整仓库的访问挡住
/// （整个库是单连接 + 单 `Mutex`）。**发起一次调用正是本路径上最长的 `await`**，
/// 故这条约束在这里最硬。`plan_candidates` 是 G 全模块唯一的 `Tx` 收口。
///
/// # 截止的来源是值，不是 `ExecutionProfile` 本体（设计 §3.5）
///
/// 承载处是 `ExecutionProfile.timeout_ms`（`crates/continuum-graph/src/execution.rs`），
/// 但它**今天零生产构造点**，故 G 收一个 `Option<Duration>`、由驱动从那个字段投影过来
/// ——**与 D 收 `ProviderHealth`、收 `BudgetView` 的形状相同**。
/// **本函数不读 `ExecutionProfile`**，也不解释 `timeout_ms` 的单位。
///
/// # 截止只包住一次调用
///
/// `deadline` 是 `Copy` 的 `Option<Duration>`，故「用一次就没了」这件事在签名上不成立
/// ——**这是它的自然语义，不是本实现额外做的一件事**。照片：
/// `the_deadline_wraps_one_call_only`（两次连续调用各带同一个短截止，两次都完成）。
///
/// **截止不包住流的消费**（设计 §3.5）：一个长回答本来就要跑很久，把截止套在分片消费上
/// 会把「回答长」误判成「调用失败」。中止一条流走 `cancel(&stream.call)`，不走截止。
///
/// # `Deadline { elapsed_ms }` 是实测耗时
///
/// 设计 §3.1 末段：取值是**从发起到截止触发那一段**，不是截止值（字段名与语义同宽）。
/// **它今天没有消费方**，故用例只断言是哪一枚 `Err`、不断言数值——断一个数值就是钉一次巧合
/// （实测耗时在调度抖动下不等于截止值）。
///
/// # 失败面
///
/// **只有两条 `Err` 途径**：适配器失败（经 [`into_call_error`]，**本函数不自己映射一遍**
/// ——设计 §6.1 的落点）与截止。本函数**不产生** `Routing` / `Storage`。
pub async fn call(
    candidate: &ExecutionCandidate,
    adapter: &Arc<dyn ModelProvider>,
    input: CallInput<'_>,
    deadline: Option<Duration>,
) -> Result<InvokeResponse, ModelCallError> {
    let request = InvokeRequest {
        // id 取自候选本人——本函数拿不到第二个来源（参数表里没有别的 id）。
        model: candidate.model().clone(),
        messages: input.messages.to_vec(),
        max_tokens: input.max_tokens,
    };

    // 起算点在 `invoke` **之前**：`elapsed_ms` 的口径是「从发起到截止触发那一段」
    // （设计 §3.1 末段），而 `tokio::time::timeout` 交回的 `Elapsed` 是「超过截止之后
    // 又过了多久」，不是那一段。故本实现自己起算，不用 `Elapsed`。
    let started = Instant::now();
    match deadline {
        // 截止是 `Copy` 的值，每一次调用各用各的一份——不存在「被上一次用掉」的状态。
        Some(limit) => match tokio::time::timeout(limit, adapter.invoke(request)).await {
            Ok(result) => result.map_err(into_call_error),
            Err(_past_deadline) => Err(ModelCallError::Deadline {
                elapsed_ms: started.elapsed().as_millis() as u64,
            }),
        },
        None => adapter.invoke(request).await.map_err(into_call_error),
    }
}

/// 设计 §6.1 的分类表：**G 这一侧的默认口径，不是对适配器的断言**。
///
/// 适配器要对某次失败改口径，应当在 `ProviderError` 的**取值上**表达
/// （明知永久就报 `Protocol`），而不是指望这里为它开一个口子。
///
/// # 这张表不写成边界上的函数
///
/// C §5.1 拒掉过那件事，理由在本路径上**同样成立**：`Transport` 在一个适配器上可能是
/// 瞬时的、在另一个上可能是永久的，**故这张表是否为真，本子项目答不出**（设计 §6.2）。
/// 它是 G 产出类别时的默认读法；`ProviderError` 的取值域不因它而收窄，本表也不是
/// 适配器必须服从的契约。
///
/// # 一条已知的射程边界（据实记，不是漏项）
///
/// 既有夹具把**永久性配置缺陷**（「这个 provider 没有这个工具」）报成 `Unavailable`：
/// `crates/continuum-provider/tests/common/mod.rs:177` 的 `FakeTool::describe_tool`
/// ——`ok_or_else(|| ProviderError::Unavailable(id.as_str().to_owned()))`。
/// **若模型侧的真实适配器也这样用 `Unavailable`，本表会把一次配置缺陷分成 `Transient`。**
/// **修它在适配器，不在这里。**
///
/// **坐标订正（2026-10-07，原话照留）**：设计 §6.2 与本计划的这一条引的是
/// `crates/continuum-provider/tests/fake_provider.rs:106`，**那个坐标在本树上已失效**——
/// 实读 `wc -l crates/continuum-provider/tests/fake_provider.rs` 得 **104 行**（即 `:106`
/// 越界），且该文件里的**夹具**今天只剩 `FakeConnector`（`fake_provider.rs:13`，另有
/// `mod common;` 与两条用例）；工具侧夹具由 C 的 Task 2 整块搬进了 `tests/common/mod.rs`
/// （搬家的记录就在被搬去的那份文档注释里）。上文那个行号是本条**实读后**的坐标。
///
/// # `FailureClass::Resource` 这一格无输入
///
/// `ProviderError` 没有「限流 / 配额耗尽」的变体（C §12 第 7 条），故下面**不写
/// `Resource` 的臂**——一个没有输入的臂是假接口。**G 不擅自扩 `continuum-core`
/// 的取值域**：那是 §315 的接口类型。
///
/// 这句话的照片是 `tests/model_call.rs` 的 `each_provider_error_variant_maps_to_its_class`：
/// 五个 `ProviderError` 变体的像被**逐项钉死**（四枚失败变体各得一个写死的类别），
/// 故往这张表的任何一格塞 `Resource` 都会让那五条断言里的某一条红。
///
/// # 返回 `Option` 的那一格
///
/// **`None` 只对 `ProviderError::Cancelled` 返回**：**它是调用方自己发的取消，不是失败**
/// （设计 §6.1）。`None` 在本函数里**不是**「不知道」——「不知道」有
/// [`FailureClass::Unknown`] 这一格，而本表不给任何变体用它：**照片与上面那张
/// `Resource` 的照片是同一条**（`each_provider_error_variant_maps_to_its_class` 把
/// 五个变体的像逐项写死，其中没有 `Unknown`）。
/// 「`None` 只对 `Cancelled` 返回」的两侧照片在 `a_cancelled_call_is_not_a_failure`。
pub fn classify(e: &ProviderError) -> Option<FailureClass> {
    match e {
        ProviderError::Transport(_) => Some(FailureClass::Transient),
        ProviderError::Unavailable(_) => Some(FailureClass::Transient),
        ProviderError::Protocol(_) => Some(FailureClass::Permanent),
        ProviderError::UnknownModel(_) => Some(FailureClass::Permanent),
        ProviderError::Cancelled(_) => None,
    }
}

/// `ProviderError` → [`ModelCallError`] 的转换点：`call` / `call_stream` / `abort`
/// 共用它，**各自不再自己映射一遍**（设计 §6.1 的落点）。
///
/// **「共用」今天有一半的照片**：[`call`] 已落地并真的走这里
/// （`tests/model_call.rs` 的 `each_provider_failure_keeps_its_class_and_its_source`
/// 与 `a_cancelled_provider_error_is_not_a_failure` 断的是它**经本函数**得到的类别与变体）。
/// **另一半仍没有**：`call_stream` / `abort` 由后续 task 落地，故现在还钉不了
/// 「它们也不再自己映射一遍」——那要等三处成形，且那张照片应当是一条源码文本判据
/// （`ModelCallError::Provider` 的构造点在本 crate 的 `src/` 里只有一处），本 task 不代写。
///
/// **它只问 [`classify`] 一次**：设计 §6.1 的落点段写明「**`Cancelled` 不落在『失败』
/// 那一枚里**」正是**分类表那一行**（`Cancelled(_)` → 不进入分类）落到本类型上的结果，
/// 故这两件事是同一个决定的两种投影，分两处写会让它们漂移——一处说「不是失败」，
/// 另一处却给它一个 `class`。**代价据实记**：表一改，本函数的产出形状跟着改
/// （把 `Cancelled` 归成某一类，取消就会被记成一次该类的失败），这正是两条用例
/// 要在表的**两处**钉死它的原因。
pub fn into_call_error(e: ProviderError) -> ModelCallError {
    match classify(&e) {
        Some(class) => ModelCallError::Provider { class, source: e },
        None => ModelCallError::Cancelled { source: e },
    }
}
