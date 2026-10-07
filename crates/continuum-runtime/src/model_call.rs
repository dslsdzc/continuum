//! 模型调用路径（设计 `docs/superpowers/specs/2026-10-06-p3g-model-calling-path-design.md`）
//! 里**不依赖四段流程**的那部分：`ProviderError` 的分类表，以及它到
//! [`ModelCallError`] 的转换。
//!
//! **本模块最早落地的就是上面那两样**。**四段流程里已落地的是第①段（[`plan_candidates`]，
//! 同步段）与第②段的前半（[`snapshot`]）**；调排序（`select`）与 `call` / `call_stream`
//! 由后续 task 落在这里。
//!
//! [`ModelCallError`] 的定义在 [`crate::error`]——那是设计 §10.2 指定的落点
//! （该文件由 F 创建，G 只增补），与把分类表放在这里并不冲突：**表是 G 的增量，
//! 而那个文件是两个子项目共写的**。

use std::sync::Arc;

use continuum_core::model::{ModelId, ProviderHealth};
use continuum_core::ProviderError;
use continuum_graph::failure::FailureClass;
use continuum_model_registry::{list_registered, load_profile, RoutableModel, RoutingError};
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
/// **「共用」是设计要求，今天没有照片**：那三处由 Task 11 落地，本 task 里它们还不存在，
/// 故现在钉不了「别处不再自己映射」——**本 crate 今天的生产代码里本函数没有调用方**。
/// 等那三处成形，这张照片应当是一条源码文本判据（`ModelCallError::Provider` 的构造点
/// 在本 crate 的 `src/` 里只有一处），本 task 不代写。
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
