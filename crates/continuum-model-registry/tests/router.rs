//! Task 10：§250 的**请求面**——[`TaskSkillRequirement`] / [`FamilyPreference`] / [`RoutingRequest`]
//! （设计 `docs/superpowers/specs/2026-10-05-p3d-model-registry-router-design.md` §5.1）。
//!
//! 本文件只覆盖**请求**这一侧。§250 的另外三样（候选集、输出、排序）都不在本 task：
//! `CostPolicy` / `LatencyPolicy` / `FailureHistory` 三个输入**不定义形状**，落在排序策略接口之后
//! （规范只给名字，`FailureHistory` 连产生方都没有指定，设计 §5.1 的两处「不定义形状」）。
//!
//! # 本文件的两处判据重点
//!
//! 1. **空需求要拒**（`an_empty_requirement_is_rejected`）：`compatibility = matched / required`
//!    在 `required` 为空时是 `0/0`；而 `CandidateScore.compatibility: Ratio` **拒 NaN**、
//!    `evaluate` **不返回 `Result`**——实现者只剩 panic 或编一个值（如 1.0）两条路，
//!    **而两条都能过其余全套用例**。故做成**非空类型**，并把「是空」这件事挡在构造期，
//!    照片就是本文件那条断言**具体哪一枚 `Err`**（设计 §5.1 末段）。
//! 2. **`Degraded` 的撞名**（`a_request_carries_availability_as_values`）：这里的
//!    `ProviderHealth::Degraded` 与 `LifecycleState::Degraded` 是**两个轴上的两个东西**，
//!    只是名字撞了。用例注释里逐句写明，免得后来者把它们混用。
//!
//! # 类型层的三条通道住在 `tests/compile_fail/`，本文件不另开 trybuild 用例
//!
//! 「需求不能从裸 `Vec<SkillDimension>` 构造」（**两条**通道：字段私有 E0451、无 `From<Vec<_>>`
//! E0277，**各一份文件**）与「请求漏掉 `budget` 编译不过」（E0063，外加「`budget` 位收的是
//! `BudgetView` 而不是一个装得下它的 `Option`」那一侧）都是**签名层**命题，运行期用例无论怎么写
//! 都只能证明「我没这么构造过」，证明不了「构造不出来」。故各有一份 trybuild 样例，
//! **由 `tests/type_level.rs` 的 `tests/compile_fail/*.rs` 通配收走**——
//! 本文件不再写一份 `t.compile_fail(...)`：同一份样例跑两遍不多出任何证据，
//! 只会让「变红的是哪一处」变模糊。
//!
//! **两条通道各要一份文件，不是为了好看**：实测（rustc 1.95）把两条错写进同一个函数体时，
//! rustc 只报第一条、第二条根本不出现——合并成一份，那条没被报出来的通道**就没有照片**。
//!
//! # 「预算必填」为什么没有运行期用例，而只有一条 `const` 形态的正侧断言
//!
//! ENG-005 的预算是**必填参数**（不是 `Option`），这是**签名层**的事实：设计 §9 的对应行写的是
//! 「签名层面（无 `Option`）」，§10 第 2 条明写**没有运行期用例**——语义层未建，
//! [`BudgetView`] 的每个 `Some` 都只能由测试构造，「剩余额度算得对不对」在本阶段**不可观察**。
//! 故本文件那条请求夹具（`a_request_without_a_budget_constraint_is_built_by_the_driver`）
//! **不验任何数值语义**，只做两件事：其一，把 `budget` 位交给一个**返回类型写明是 `BudgetView`**
//! 的函数（`no_budget_constraint`），于是「把字段改成 `Option<BudgetView>`」在本文件里是 `E0308`；
//! 其二，把 [`BudgetView`] 的五个量纲一次写全，于是「少一个量纲」是 `E0560`。
//! **这两条都只是编译期的事实**，按本项目纪律不算「变红」（见报告里变异档位的说明）。

use continuum_core::model::{ModelId, ProviderHealth};
use continuum_model_registry::{
    BudgetView, CandidateScore, FamilyPreference, FamilyRelation, LifecycleState, ModelProfile,
    RankedExecutionCandidates, RankingPolicy, Ratio, RequirementError, RoutableModel, RoutableState,
    RoutingError, RoutingRequest, RoutingReason, SkillDimension, TaskSkillRequirement, load_profile,
    p3d_model_migrations, rank, register_model,
};
use continuum_persist::{Db, Migration, Tx, Value, builtin_migrations};

/// §19 的五项（`docs/spec/01-concepts.md:933-960` 的封闭清单）与**手写的**落库编码字面量。
///
/// 编码是手写的字面量，**不是**被测函数返回的（本项目纪律：手写的钉格式，函数返回的钉路径）。
/// 五项**逐项列出**，不抽代表。
const ALL_FAMILIES: [(FamilyPreference, &str); 5] = [
    (FamilyPreference::Auto, "auto"),
    (FamilyPreference::OpenAiPreferred, "openai_preferred"),
    (FamilyPreference::ClaudePreferred, "claude_preferred"),
    (FamilyPreference::LocalPreferred, "local_preferred"),
    (FamilyPreference::Custom, "custom"),
];

/// 一份**非空**的三维需求。**顺序是刻意的乱序**（不是 §248 的九维次序，也不是字典序）：
/// 它是下面那条「顺序逐一相同」的用例能否变红的全部依据——喂进去的若本来就是有序的，
/// 那么「构造期排一次序」这类改动照样绿。
fn three_dimensions() -> TaskSkillRequirement {
    TaskSkillRequirement::try_new(vec![
        SkillDimension::Media,
        SkillDimension::Coding,
        SkillDimension::Verification,
    ])
    .expect("三个维度是非空集合")
}

/// ENG-005 的预算视图：五个量纲**都不构成约束**（全 `None`）。
///
/// **返回类型 `BudgetView` 是承重的**：它把「`RoutingRequest.budget` 就是这个类型」钉在调用点上
/// ——若该字段被改成 `Option<BudgetView>`，本函数在 `budget: no_budget_constraint()` 那一行是 `E0308`。
///
/// 五个量纲一次写全：少一个即 `E0560`，多一个即 `E0063`。
/// **`None` 的取值本身不承载判据**（`None` ≠ `Some(0)`——前者是「不约束」，
/// 后者是「额度为零」，折叠发生在驱动侧的投影里，本 crate 里没有那个落点；见 `tests/budget.rs`）。
fn no_budget_constraint() -> BudgetView {
    BudgetView {
        money: None,
        wall_time: None,
        token: None,
        gpu_time: None,
        network_transfer: None,
    }
}

/// 一次自动路由的请求，四个字段按 §5.1 的签名装配（策略侧的三样不在请求里）。
fn request(family: FamilyPreference, availability: Vec<(ModelId, ProviderHealth)>) -> RoutingRequest {
    RoutingRequest {
        requirements: three_dimensions(),
        family,
        availability,
        budget: no_budget_constraint(),
    }
}

/// **空需求被构造期拒绝，且断言是哪一枚 `Err`。**
///
/// 红的条件：`try_new` 对空集合返回 `Ok`（或不返回 `Err`）即红——**这是 fail-open 的那一侧**：
/// 放行一个空需求，`compatibility` 就是 `0/0`，而它的下游 `Ratio` 拒 NaN、`evaluate` 不返回
/// `Result`，于是这个 NaN 一路走到策略里才炸（或更糟：被编成某个数）。
///
/// **断言到具体哪一枚**（不是「返回了 Err」）：只断言 `is_err()` 的话，把失败换成另一枚
/// （比如将来误删了 `RequirementEmpty` 而返回别的变体）不会有任何用例变红。
///
/// 为什么是「构造期拒绝」而不是「规定空需求时 `compatibility = 1.0`」：后者要为一个规范
/// **没有定义**的状态发明一个语义值，而「一次不需要任何能力的任务」是不是一个合法任务，
/// 规范没有说——类型层拒掉它比替它选一个数诚实（设计 §5.1 末段）。
#[test]
fn an_empty_requirement_is_rejected() {
    let err = match TaskSkillRequirement::try_new(Vec::new()) {
        Ok(_) => panic!("空需求应被拒，`try_new` 却返回了 `Ok`"),
        Err(e) => e,
    };
    assert_eq!(err, RequirementError::RequirementEmpty);
}

/// **非空需求的内容与顺序逐一相同**，`dimensions()` 给出切片。
///
/// 两侧各有一张照片，因为「非空」这条守卫的两侧各能单独漂移：
///
/// - **单元素**：边界那一格。非空的最小形态**必须被接受**——只写「三个维度能过」的话，
///   把守卫改成 `dims.len() < 2` 即把单元素需求也拒掉，而**没有任何用例会红**
///   （这与「只钉一侧就等于没钉」同一条；缺的这侧正是 fail-closed 漂移的那侧）。
/// - **三元素乱序**：钉顺序**不是**被重排过的——构造期排一次序、去一次重、或按 §248 的九维
///   下标重排，都会让这张照片红。喂进去的顺序刻意与九维次序、字典序都不同。
///
/// 红的条件：`dimensions()` 返回的不是入参那一片（改了顺序、改了内容、或返回了别的向量）。
#[test]
fn a_non_empty_requirement_keeps_its_dimensions_in_order() {
    let single = TaskSkillRequirement::try_new(vec![SkillDimension::ToolUse])
        .expect("一个维度也是非空集合");
    assert_eq!(single.dimensions(), &[SkillDimension::ToolUse]);

    let kept = three_dimensions();
    assert_eq!(
        kept.dimensions(),
        &[
            SkillDimension::Media,
            SkillDimension::Coding,
            SkillDimension::Verification,
        ],
    );
}

/// §19 的五项**逐项**：类型里的取值与它的落库编码。
///
/// 编码的规则与 `SkillDimension` / `LifecycleState` 同一张表：小写，多词以 `_` 连接
/// （全局约束「编码挂在其类型上」，本 crate 不依赖 serde、不用 `Debug` 表示落库）。
/// **本阶段这个值不落库**（三张表里没有 family 这一列，Task 5 的迁移 80 可查），
/// 但那份编码与类型**同址**：将来给它建列的那个 task 直接用这一对函数，不另建一张表。
///
/// 红的条件：任一枚编码被改成大写或驼峰（`"Auto"` / `"autoPreferred"`）即红——
/// 五个字面量是手写的，逐项比对。
///
/// # 本用例守不住的那一处，据实写明
///
/// 上面那张表是**手写的**：**给 [`FamilyPreference`] 加第六个成员时，本用例不会红**
/// （表里没有它，循环也遍历不到它）。这点与 `tests/lifecycle.rs` 的 `ALL_STATES`、
/// `tests/profile.rs` 的 `ALL_DIMENSIONS` 是同一处不设防的点，登记在此而不是假装覆盖到了。
/// §19 是**封闭清单**（设计 §5.1），故「加第六项」本身要先改规范；本用例能保证的是
/// **已有的五项各自的编码逐字正确**。
#[test]
fn the_five_family_preferences_are_recorded_verbatim() {
    assert_eq!(ALL_FAMILIES.len(), 5, "§19 的清单是五项，本表须逐项列全");

    for (index, (family, name)) in ALL_FAMILIES.iter().enumerate() {
        assert_eq!(
            family.as_str(),
            *name,
            "§19 第 {index} 项的落库编码应是 {name:?}（小写、多词以 `_` 连接）",
        );
    }

    // `Custom` 在 §19 里**不带载荷**（规范只写了 "Custom" 这一个词），本类型也不给它加：
    // 上面那行 `FamilyPreference::Custom` 是个**裸构造**——若给它加了载荷，这一行即编译不过。
    let custom_without_payload = FamilyPreference::Custom;
    assert_eq!(custom_without_payload.as_str(), "custom");
}

/// §250 的「当前可用性」：`Vec<(ModelId, ProviderHealth)>`，带上 §315 的三种取值**逐项**。
///
/// # 撞名：`ProviderHealth::Degraded` **不是** `LifecycleState::Degraded`
///
/// 这里的 `Degraded` 是**供应商侧的可用性**（`crates/continuum-core/src/model.rs:83-87`：
/// `Healthy` / `Degraded` / `Unavailable`，说的是**这一家供应商现在还好不好用**）；
/// 而 `LifecycleState::Degraded` 是 **§249 的模型生命周期异常态**（`docs/spec/05-normative.md:868-871`：
/// 这个**模型本体**已进入异常态）。**两个轴上的两个东西，只是名字撞了**（设计 §4.2 末段）。
/// 后来者若把它们当一个，会得出「可用性降级 = 生命周期降级」这种规范里没有的等式，
/// 而两者在本层是**分别**进 [`RoutingRequest`] 与 `RoutableModel` 的。
/// （本 crate 里两处都只有「装得下」的照片：`LifecycleState::Degraded` 过闸门那侧在
/// `src/lifecycle.rs` 的 `#[cfg(test)] mod tests`，`ProviderHealth::Degraded` 在这条用例。）
///
/// # 这两种取值**怎么用**不在本 task
///
/// 过滤与失误路径都在 Task 11 的候选集构造里（设计 §5.3：只过滤 `Unavailable`，
/// `Healthy` 与 `Degraded` 都留下且**排序不变**）。本 task 只保证**它们装得进请求、且彼此可分辨**
/// ——把它们揉成同一个值（例如换成 `Vec<(ModelId, bool)>`）就没有任何东西能表达这个区别了。
///
/// 红的条件：字段的类型换了、或三个取值里任意两个被折叠（本条是**编译期**的判据，
/// 见文件头：本 crate 里没有能把这个请求重新解释一遍的实现体，故不存在运行期变异体）。
#[test]
fn a_request_carries_availability_as_values() {
    let healthy = ModelId::new("model-healthy");
    let degraded = ModelId::new("model-degraded");
    let unavailable = ModelId::new("model-unavailable");

    let carried = request(
        FamilyPreference::Auto,
        vec![
            (healthy.clone(), ProviderHealth::Healthy),
            (degraded.clone(), ProviderHealth::Degraded),
            (unavailable.clone(), ProviderHealth::Unavailable),
        ],
    );

    assert_eq!(carried.availability.len(), 3, "三个取值逐项各一条");
    assert_eq!(carried.availability[0], (healthy, ProviderHealth::Healthy));
    assert_eq!(carried.availability[1], (degraded, ProviderHealth::Degraded));
    assert_eq!(
        carried.availability[2],
        (unavailable, ProviderHealth::Unavailable),
    );
}

/// 四个字段**逐项**读得回来：请求是**纯数据、无 I/O**，故它是可装配、可表驱动的输入
/// （设计 §5.1 的「请求与策略的分界是刻意的」）。
///
/// **「没有策略字段」这一条也有照片**：本用例只装配 §5.1 表里落在**请求**那一侧的三样
/// （`TaskSkillRequirement` / `FamilyPreference` / 可用性）加 ENG-005 的 `budget`，
/// 结构体字面量**没有** `cost_policy` / `latency_policy` / `failure_history` 三格可填——
/// 给 `RoutingRequest` 加这三个字段中任何一个，本用例即 `E0063`（字面量少字段）。
/// 三样落在**策略**那一侧的理由是「它是『怎么算』而不是『算什么』」，规范未给形状。
///
/// 红的条件：任一字段改名、改类型、或增删字段即编译不过（同文件头：签名层的判据）。
#[test]
fn a_request_without_a_budget_constraint_is_built_by_the_driver() {
    let carried = request(FamilyPreference::ClaudePreferred, Vec::new());

    assert_eq!(carried.family.as_str(), "claude_preferred");
    assert_eq!(
        carried.requirements.dimensions(),
        &[
            SkillDimension::Media,
            SkillDimension::Coding,
            SkillDimension::Verification,
        ],
    );
    assert!(
        carried.availability.is_empty(),
        "没有可用性快照时请求照样装得下（`rank` 对缺席的处置在 Task 11）",
    );
    // 预算五个量纲都装得下 `None`（该量纲当前不构成约束）。**不读回、不比较数值**：
    // 「剩余额度算得对不对」在本阶段不可观察（设计 §10 第 2 条）。
    let _: &BudgetView = &carried.budget;
}

/// §5.1 的「请求 vs 策略」分界：请求侧纯数据、可表驱动——**同一条策略换一份请求重跑**。
///
/// 本用例是上面那条分界的**行为**照片：把同一组可用性配给五个 family，**逐项**得到一个请求，
/// 每个请求的 `family` 就是配进去的那一个。若 `FamilyPreference` 被折叠成一个布尔
/// （「要不要跨 family」），本用例的五个配置就无法两两分辨。
#[test]
fn the_same_availability_can_be_paired_with_each_family() {
    let snapshot = vec![(ModelId::new("model-alpha"), ProviderHealth::Degraded)];

    for (family, name) in ALL_FAMILIES {
        let carried = request(family, snapshot.clone());
        assert_eq!(
            carried.family.as_str(),
            name,
            "{name} 这一档的请求应带上它自己那一档",
        );
        assert_eq!(carried.availability, snapshot);
    }
}

// ===== Task 11：`rank` 的候选集构造、全序与输出面（设计 §5.2、§5.3、§5.4） =====
//
// # 夹具链：裸 SQL 播两行 → `load_profile` → `RoutableModel::try_new`
//
// [`RoutableModel`] 只能由 [`ModelProfile`] 产出，而画像在 crate 外构造不出来（Task 3），
// `save_profile` 又收一枚画像做入参——链条不能自举，故**第一枚画像由裸 SQL 播下**
// （与 Task 7 的夹具同一处置：`tests/persist.rs` 的 `insert_profile_row`）。
//
// **每一行画像必须先登记**：`model_profile.id REFERENCES model_registry(id)`，没登记就播画像，
// 外键会拒（`Db::open_with` 开了 `PRAGMA foreign_keys=ON`）。
//
// # 本文件的用例需要 0 维观测
//
// `load_profile` 会组合 `load_skill_vector` 填画像的第十二个字段（Task 8），而**本文件的策略
// 不读 `skill_vector`**（`rank` 交付的是机制，具体打分是 Task 12 的 `BaselineRankingPolicy`）。
// 故这里唯一需要的是「画像在」——观测一维都不播。需要 1 / N 维的用例（Task 12）在播下画像
// 之后再调 `save_skill_observation`。
//
// # 每个候选都**必须**有 `availability` 条目
//
// `rank` 对「候选集里有、`availability` 里没有」的模型返回 `Err(UnknownAvailability)`（设计 §5.3，
// 不当作可用）。漏给一条条目会以 `Err` 的形式失败、且报错位置指向被测函数，
// 故 `availability(...)` 是唯一入口——需要别的健康度时显式写出来，不靠默认值。

/// 建一个装了三张表的临时库。
fn db() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let mut migrations: Vec<Migration> = builtin_migrations();
    migrations.extend(p3d_model_migrations());
    let db = Db::open_with(&path, migrations).unwrap();
    db.migrate().unwrap();
    (dir, db)
}

/// 播一行登记项 ＋ 一行画像。**每个 id 只调一次**——两条写都是裸 `INSERT`，第二次即主键拒。
///
/// 画像的十一列里只有 `confidence` 由调用方给（本文件的策略读它，见 `StubPolicy::evaluate`），
/// 其余取固定字面量：本 task 不读它们，而它们必须是**合法取值**，否则 `load_profile` 的解码
/// 会以 `Err` 的形式失败（报错位置同样指向被测函数）。三个列表列是**手写** JSON 文本。
fn seed_candidate(tx: &Tx<'_>, model: &str, confidence: &str) {
    register_model(tx, &ModelId::new(model)).unwrap();
    tx.execute(
        "INSERT INTO model_profile
           (id, version, provider, model_revision, modalities, tools, failure_modes,
            cost_profile, latency_profile, evidence_count, confidence)
         VALUES (?1, 'version-beta', 'provider-gamma', 'revision-delta',
                 ?2, ?3, '[]', NULL, NULL, 42, ?4)",
        &[
            Value::text(model),
            Value::text(r#"["text"]"#),
            Value::text(r#"["tool-epsilon"]"#),
            Value::text(confidence),
        ],
    )
    .unwrap();
}

/// 从已播下的那一行读回画像（**crate 外唯一的画像来源**）。
fn profile_of(tx: &Tx<'_>, model: &str) -> ModelProfile {
    load_profile(tx, &ModelId::new(model))
        .unwrap()
        .unwrap_or_else(|| panic!("画像已由裸 SQL 播下，{model} 应读得到"))
}

/// 一个候选：已播下的画像 ＋ 过闸门的状态。
fn routable(tx: &Tx<'_>, model: &str, state: LifecycleState) -> RoutableModel {
    RoutableModel::try_new(profile_of(tx, model), state)
        .expect("用例给的状态应在可路由六态里")
}

/// `RoutingRequest::availability` 的条目，按 §315 的三种取值显式写出。
fn availability(entries: &[(&str, ProviderHealth)]) -> Vec<(ModelId, ProviderHealth)> {
    entries
        .iter()
        .map(|(model, health)| (ModelId::new(*model), *health))
        .collect()
}

/// 一个候选的**全部可观察面**（五个访问器 ＋ `reason` 的四个字段），用于「逐项相同」的断言。
///
/// 用它而**不给这些类型派生 `PartialEq`**：本模块的类型按设计不派生东西（`router.rs` 的模块头），
/// 而「相同」的判据要走**访问器**——那是消费者真正拿到的东西。
type Fingerprint = (
    String,
    RoutableState,
    f64,
    f64,
    FamilyRelation,
    Vec<SkillDimension>,
    Vec<SkillDimension>,
    Vec<String>,
);

fn fingerprint(ranked: &RankedExecutionCandidates) -> Vec<Fingerprint> {
    ranked
        .candidates()
        .iter()
        .map(|candidate| {
            (
                candidate.model().as_str().to_string(),
                candidate.state(),
                candidate.compatibility().get(),
                candidate.confidence().get(),
                candidate.reason().family(),
                candidate.reason().matched().to_vec(),
                candidate.reason().missing().to_vec(),
                candidate.reason().notes().to_vec(),
            )
        })
        .collect()
}

/// 表驱动的测试策略：`ModelId` → `(compatibility, family)`，`confidence` 取**画像上的**那一个
/// （§247，与设计 §5.3 的基线同源）。`matched` / `missing` / `notes` 三个字段全表共用一份。
///
/// **本 task 交付的是机制**（接口、全序、候选集构造），具体打分函数是 Task 12 的
/// `BaselineRankingPolicy`（设计 §5.3 的「第二步」）。故这里的策略是一张**手写的**表，
/// 让用例能把 compatibility / family 与排序结果对上。
struct StubPolicy {
    scores: Vec<(&'static str, f64, FamilyRelation)>,
    matched: Vec<SkillDimension>,
    missing: Vec<SkillDimension>,
    notes: Vec<String>,
}

impl StubPolicy {
    fn new(scores: Vec<(&'static str, f64, FamilyRelation)>) -> Self {
        Self {
            scores,
            matched: vec![SkillDimension::Coding],
            missing: vec![SkillDimension::Media, SkillDimension::Verification],
            notes: vec![String::from("策略写下的一条事实陈述")],
        }
    }
}

impl RankingPolicy for StubPolicy {
    fn evaluate(&self, _request: &RoutingRequest, model: &RoutableModel) -> CandidateScore {
        let id = model.profile().id().as_str();
        let (_, compatibility, family) = self
            .scores
            .iter()
            .find(|(name, ..)| *name == id)
            .unwrap_or_else(|| panic!("策略表里没有 {id}"));

        CandidateScore {
            compatibility: Ratio::try_new(*compatibility)
                .expect("用例给的 compatibility 应是合法 Ratio"),
            confidence: model.profile().confidence(),
            reason: RoutingReason::new(
                *family,
                self.matched.clone(),
                self.missing.clone(),
                self.notes.clone(),
            ),
        }
    }
}

/// **输出有序，且 `selected()` 是表头**（§84 的 `selected_model`，设计 §5.2）。
///
/// 三个候选的 `compatibility` 递减，故表头就是最高的那个；`confidence` 的取值**不参与**
/// 这个次序（它只在 `compatibility` 并列时起作用）。
///
/// 红的条件：`selected()` 不返回表头（例如取表尾），或候选的兼容度读错字段、使期望次序不成立。
///
/// **本处不承担「输入顺序无关」这条照片**：夹具按期望次序喂入，故删掉 `rank` 的 `sort_by`
/// 本用例照样绿。那条照片归 `shuffling_the_input_does_not_change_the_output`（三种输入顺序）
/// 与 `tied_candidates_are_ordered_by_model_id`（按与 id 升序相反的顺序喂入）。
#[test]
fn the_result_is_ordered_and_selected_is_its_head() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    seed_candidate(&tx, "model-alpha", "0.1");
    seed_candidate(&tx, "model-bravo", "0.2");
    seed_candidate(&tx, "model-charlie", "0.3");

    let models = vec![
        routable(&tx, "model-alpha", LifecycleState::Active),
        routable(&tx, "model-bravo", LifecycleState::Active),
        routable(&tx, "model-charlie", LifecycleState::Active),
    ];
    let policy = StubPolicy::new(vec![
        ("model-alpha", 0.9, FamilyRelation::SameFamily),
        ("model-bravo", 0.5, FamilyRelation::SameFamily),
        ("model-charlie", 0.3, FamilyRelation::SameFamily),
    ]);
    let request = request(
        FamilyPreference::Auto,
        availability(&[
            ("model-alpha", ProviderHealth::Healthy),
            ("model-bravo", ProviderHealth::Healthy),
            ("model-charlie", ProviderHealth::Healthy),
        ]),
    );

    let ranked = rank(&request, &models, &policy).expect("三个候选都有可用性条目，应排出序来");

    assert_eq!(
        ranked.selected().model().as_str(),
        "model-alpha",
        "`compatibility` 最高的那一个应是表头（§84 的 selected_model）"
    );
    let ordered: Vec<&str> = ranked
        .candidates()
        .iter()
        .map(|candidate| candidate.model().as_str())
        .collect();
    assert_eq!(ordered, vec!["model-alpha", "model-bravo", "model-charlie"]);

    tx.commit().unwrap();
}

/// §84 的 `alternatives` **不是第二份数据**，是同一份有序列表的表尾（设计 §5.2）。
///
/// 判据是**指针相同**（`as_ptr()`）而不是「内容相等」：内容相等在「另拷一份表尾」的实现下
/// 照样成立，而设计要的正是「不另设 `alternatives` 字段」那条——同一份数据只有一个落点。
///
/// 红的条件：`alternatives()` 返回一份拷贝（指针不同）或返回的不是表尾（长度或首元素不对）。
#[test]
fn alternatives_is_the_tail_of_the_same_list() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    seed_candidate(&tx, "model-alpha", "0.1");
    seed_candidate(&tx, "model-bravo", "0.2");
    seed_candidate(&tx, "model-charlie", "0.3");

    let models = vec![
        routable(&tx, "model-alpha", LifecycleState::Active),
        routable(&tx, "model-bravo", LifecycleState::Active),
        routable(&tx, "model-charlie", LifecycleState::Active),
    ];
    let policy = StubPolicy::new(vec![
        ("model-alpha", 0.9, FamilyRelation::SameFamily),
        ("model-bravo", 0.5, FamilyRelation::SameFamily),
        ("model-charlie", 0.3, FamilyRelation::SameFamily),
    ]);
    let request = request(
        FamilyPreference::Auto,
        availability(&[
            ("model-alpha", ProviderHealth::Healthy),
            ("model-bravo", ProviderHealth::Healthy),
            ("model-charlie", ProviderHealth::Healthy),
        ]),
    );

    let ranked = rank(&request, &models, &policy).expect("三个候选都应过");

    let candidates = ranked.candidates();
    let alternatives = ranked.alternatives();

    assert_eq!(alternatives.len(), candidates.len() - 1, "表尾比整表少表头一个");
    assert_eq!(
        alternatives.as_ptr(),
        candidates[1..].as_ptr(),
        "`alternatives()` 必须是同一份列表的表尾，不是另拷的一份数据"
    );
    let tail: Vec<&str> = alternatives
        .iter()
        .map(|candidate| candidate.model().as_str())
        .collect();
    assert_eq!(tail, vec!["model-bravo", "model-charlie"]);

    tx.commit().unwrap();
}

/// `ExecutionCandidate` 的**五个访问器逐项各一条**（不抽代表）。
///
/// 五个都是接口冻结处的必需品：`selected()` 交给消费者，消费者下一步拿着 `model()` 去
/// 调 provider（子项目 G）；`state()` 让消费者决定是否降权／重试；两个 `Ratio` 是 §84 的示例值；
/// `reason()` 是 §84 要输出的那件事（设计 §5.2）。
///
/// 取值刻意**偏离默认**：`state` 取 `Active`（不是六态里的第一个 `Discovered`）、
/// `compatibility` 取 0.75、`confidence` 取画像上的 0.4、`family` 取 `SameFamily`——
/// 每一项都能与「访问器读错了另一个字段」区分开。
///
/// 红的条件：五个访问器里任一返回别的字段（`state()` 返回 `RoutableState::Discovered`、
/// `compatibility()` 返回 `confidence`、`reason()` 不是策略给的那一枚，等等）。
#[test]
fn every_candidate_accessor_has_a_photo() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    seed_candidate(&tx, "model-alpha", "0.4");
    let models = vec![routable(&tx, "model-alpha", LifecycleState::Active)];
    let policy = StubPolicy::new(vec![("model-alpha", 0.75, FamilyRelation::SameFamily)]);
    let request = request(
        FamilyPreference::Auto,
        availability(&[("model-alpha", ProviderHealth::Healthy)]),
    );

    let ranked = rank(&request, &models, &policy).expect("唯一候选有可用性条目，应过");
    let candidate = ranked.selected();

    assert_eq!(candidate.model(), &ModelId::new("model-alpha"), "访问器 model()");
    assert_eq!(
        candidate.state(),
        RoutableState::Active,
        "访问器 state()：它是过闸门后的六态值，取自输入那个模型（不是默认态）"
    );
    assert_eq!(
        candidate.compatibility().get(),
        0.75,
        "访问器 compatibility()：策略给的那个数"
    );
    assert_eq!(
        candidate.confidence().get(),
        0.4,
        "访问器 confidence()：§247 画像上的 confidence，不是策略另算的一个数"
    );
    assert_eq!(
        candidate.reason().family(),
        FamilyRelation::SameFamily,
        "访问器 reason()：策略给的那一枚 reason（本条与下一条用例两侧对钉 family）"
    );

    tx.commit().unwrap();
}

/// `RoutingReason` 的**四个字段逐项**（`family()` / `matched()` / `missing()` / `notes()`）。
///
/// `missing` 是 `Vec<SkillDimension>`，**不是一个数值**——本设计不把「缺一个维度」折算成任何
/// 扣分，因为那需要一个规范没有的权重（设计 §5.2）。
///
/// 本条的 `family` 取 `CrossFamily`，与上一条的 `SameFamily` 构成两侧：一个把 `family()`
/// 写死成任一取值的实现，必在其中一条上红。
///
/// 红的条件：任一访问器返回别的字段或换了形状（`missing()` 折成一个 `Ratio` 即编译不过，
/// 转而返回 `matched()` 的内容即红）。
#[test]
fn the_reason_carries_family_matched_missing_and_notes() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    seed_candidate(&tx, "model-alpha", "0.5");
    let models = vec![routable(&tx, "model-alpha", LifecycleState::Active)];
    let policy = StubPolicy::new(vec![("model-alpha", 0.5, FamilyRelation::CrossFamily)]);
    let request = request(
        FamilyPreference::Auto,
        availability(&[("model-alpha", ProviderHealth::Healthy)]),
    );

    let ranked = rank(&request, &models, &policy).expect("唯一候选有可用性条目，应过");
    let reason = ranked.selected().reason();

    assert_eq!(reason.family(), FamilyRelation::CrossFamily, "reason.family()");
    assert_eq!(
        reason.matched(),
        &[SkillDimension::Coding],
        "reason.matched()：命中的需求维度"
    );
    assert_eq!(
        reason.missing(),
        &[SkillDimension::Media, SkillDimension::Verification],
        "reason.missing()：未命中的维度 —— 是**一个向量**，不是一个折算过的数"
    );
    assert_eq!(
        reason.notes(),
        &[String::from("策略写下的一条事实陈述")],
        "reason.notes()：策略写入的事实陈述"
    );

    tx.commit().unwrap();
}

/// 打乱输入顺序，输出**逐项**相同（全序且确定的照片，设计 §5.3）。
///
/// 夹具里 `model-bravo` 与 `model-charlie` 的 `compatibility` 与 `confidence` **全同**：
/// 这一档并列，次序只能由兜底档（`ModelId` 升序）定。**若没有并列，本用例在删掉兜底档后
/// 照样绿**——`sort_by` 是稳定排序，无并列时次序与输入顺序无关。
///
/// 红的条件：`compare` 的兜底一档不按 `ModelId` 升序即红（两种输入顺序会给出两种输出）。
#[test]
fn shuffling_the_input_does_not_change_the_output() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    seed_candidate(&tx, "model-alpha", "0.5");
    seed_candidate(&tx, "model-bravo", "0.5");
    seed_candidate(&tx, "model-charlie", "0.5");

    let policy = StubPolicy::new(vec![
        ("model-alpha", 0.9, FamilyRelation::SameFamily),
        ("model-bravo", 0.5, FamilyRelation::SameFamily),
        ("model-charlie", 0.5, FamilyRelation::SameFamily),
    ]);
    let request = request(
        FamilyPreference::Auto,
        availability(&[
            ("model-alpha", ProviderHealth::Healthy),
            ("model-bravo", ProviderHealth::Healthy),
            ("model-charlie", ProviderHealth::Healthy),
        ]),
    );

    let fingerprints: Vec<Vec<Fingerprint>> = [
        ["model-alpha", "model-bravo", "model-charlie"],
        ["model-charlie", "model-bravo", "model-alpha"],
        ["model-bravo", "model-charlie", "model-alpha"],
    ]
    .iter()
    .map(|order| {
        let models: Vec<RoutableModel> = order
            .iter()
            .map(|model| routable(&tx, model, LifecycleState::Active))
            .collect();
        fingerprint(&rank(&request, &models, &policy).expect("三种输入顺序都应排出序来"))
    })
    .collect();

    let ordered: Vec<&str> = fingerprints
        .iter()
        .flat_map(|items| items.iter().map(|item| item.0.as_str()))
        .collect();
    assert_eq!(
        ordered,
        vec![
            "model-alpha",
            "model-bravo",
            "model-charlie",
            "model-alpha",
            "model-bravo",
            "model-charlie",
            "model-alpha",
            "model-bravo",
            "model-charlie",
        ],
        "三种输入顺序的输出应逐项相同，且并列的两条按 ModelId 升序"
    );
    assert_eq!(fingerprints[0], fingerprints[1], "打乱输入不改变输出");
    assert_eq!(fingerprints[1], fingerprints[2], "打乱输入不改变输出");

    tx.commit().unwrap();
}

/// 两条 `compatibility` / `confidence` 全同、`ModelId` 不同 → 输出按 `ModelId` **升序**。
///
/// **夹具把两条同分候选按与 id 升序相反的顺序喂入**（`model-bravo` 在前）：Rust 的 `sort_by`
/// 是**稳定**排序，若按 id 升序喂入，则删掉兜底档后输出照样是 id 升序——**假绿**。
///
/// 红的条件：去掉 `compare` 的兜底那一档即红（输出会是 `["model-bravo", "model-alpha"]`）。
#[test]
fn tied_candidates_are_ordered_by_model_id() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    seed_candidate(&tx, "model-alpha", "0.5");
    seed_candidate(&tx, "model-bravo", "0.5");

    let models = vec![
        routable(&tx, "model-bravo", LifecycleState::Active),
        routable(&tx, "model-alpha", LifecycleState::Active),
    ];
    let policy = StubPolicy::new(vec![
        ("model-alpha", 0.5, FamilyRelation::SameFamily),
        ("model-bravo", 0.5, FamilyRelation::SameFamily),
    ]);
    let request = request(
        FamilyPreference::Auto,
        availability(&[
            ("model-alpha", ProviderHealth::Healthy),
            ("model-bravo", ProviderHealth::Healthy),
        ]),
    );

    let ranked = rank(&request, &models, &policy).expect("两个候选都应过");
    let ordered: Vec<&str> = ranked
        .candidates()
        .iter()
        .map(|candidate| candidate.model().as_str())
        .collect();

    assert_eq!(
        ordered,
        vec!["model-alpha", "model-bravo"],
        "两条候选全同分时，兜底档按 ModelId 升序——输入顺序是反的，故这不是稳定排序的副作用"
    );

    tx.commit().unwrap();
}

/// `compatibility` 并列时，`confidence` **降序**决定次序（设计 §5.3 的缺省 `compare`：
/// 「按 (compatibility, confidence) 降序」）。
///
/// **这条用例是补上来的，不在计划 Step 2 的清单里**（来历记在此处，免得被当成漏项或越权）：
/// 实跑变异「去掉 confidence 那一档」时**全绿**——上面两条排序用例里并列的候选在
/// `compatibility` 与 `confidence` 上是**同时**并列的，故那一档从来没有被判据碰到。
/// 而 `compare` 的文档注释与设计 §5.3 都**写死了**它是第二档：注释里的绝对措辞要有照片
/// （本仓已立过这条），故在此补一条，不推给 Task 12——Task 12 的用例清单里也没有它。
///
/// 夹具的形态是承重的：两条候选 `compatibility` 全同（0.5），`confidence` 不同
/// （0.4 对 0.8），且按 `ModelId` **升序**喂入（`model-alpha` 在前），而期望的表头是
/// `model-bravo`。故本条在三种实现下都红：**去掉第二档**（稳定排序保序 → alpha 在前）、
/// **把第二档写反**（升序 → alpha 在前）、**只剩兜底档**（`ModelId` 升序 → alpha 在前）。
#[test]
fn confidence_breaks_a_compatibility_tie() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    seed_candidate(&tx, "model-alpha", "0.4");
    seed_candidate(&tx, "model-bravo", "0.8");

    let models = vec![
        routable(&tx, "model-alpha", LifecycleState::Active),
        routable(&tx, "model-bravo", LifecycleState::Active),
    ];
    let policy = StubPolicy::new(vec![
        ("model-alpha", 0.5, FamilyRelation::SameFamily),
        ("model-bravo", 0.5, FamilyRelation::SameFamily),
    ]);
    let request = request(
        FamilyPreference::Auto,
        availability(&[
            ("model-alpha", ProviderHealth::Healthy),
            ("model-bravo", ProviderHealth::Healthy),
        ]),
    );

    let ranked = rank(&request, &models, &policy).expect("两个候选都应过");
    let ordered: Vec<&str> = ranked
        .candidates()
        .iter()
        .map(|candidate| candidate.model().as_str())
        .collect();

    assert_eq!(
        ordered,
        vec!["model-bravo", "model-alpha"],
        "`compatibility` 并列时按 `confidence` 降序：0.8 的那个在前，尽管它是 id 更大的那个"
    );

    tx.commit().unwrap();
}

/// **`compatibility` 与 `confidence` 都并列时，`family` 决定次序**——`compare` 的第三档
/// （设计 §5.3：§19 的「优先同一 model family」）。
///
/// **这条用例是补上来的，不在计划 Step 2 的清单里**（来历记在此处，免得被当成漏项或越权）：
/// 计划把这一档的照片派给了 Task 12 的 `same_family_candidates_rank_before_cross_family_ones`，
/// **这一半派单已撤回**——这一档长在本 task 的代码里（`compare` 的缺省实现里 `family_rank`
/// 那一行），它的两个操作数都是本 task 的候选，输入也在本 task 的夹具射程内。
/// 实跑变异「删掉第三档」（M7）时本 task 原有用例**全绿**：那些用例里并列的候选 `family`
/// 全是 `SameFamily`，这一档从未决定过任何次序。故在此补一条照片。
///
/// **Task 12 那条用例不删、只换夹具**：它独有「**族由请求的偏好算出**」那半边
/// （`OpenAiPreferred` 时同族优先、`Auto` 时全部视为 `SameFamily`），本用例钉的只是
/// 「`reason.family` 不同的两条候选怎么排」。删了它，那半边就没有照片。
///
/// （此处的互引按**名字**写：这里早先引的是计划的行号 `:1247`，那是一处 bash 围栏
/// ——该用例在计划里记作 `:1283`；本文件另一处引的 `src/router.rs:458` 也在本次改动
/// 加了模块头行之后失效。行号式互引会漂，名字不会；两个错引的行号留在这里作来历。）
///
/// 夹具的形态是承重的：两条候选的 `compatibility`（0.5）与 `confidence`（画像列的 0.5）
/// **全同**，前两档都判不了；按 `ModelId` **升序**喂入（`model-alpha` 在前），而期望的表头是
/// **同族的 `model-bravo`**——它是 id 更大的那个。故删掉第三档（落到兜底档的 id 升序）即红。
///
/// **两侧对钉**：把 `model-bravo` 也改成 `CrossFamily`（两族关系相同）→ 第三档同样判不了，
/// 次序回到兜底档的 id 升序，表头是 `model-alpha`。这一侧钉的是「**两族关系相同时第三档交出
/// `Equal`**、次序交还给兜底档」，且**在本用例内**必须按两种输入顺序各来一遍（2a 升序 / 2b 倒序）
/// ——只留一种，本用例就漏掉下面那两类里的一类：
///
/// - **2a**（按 id 升序喂入）：一个「并列时返回 `Less`」的第三档在这里红（实测 M7d）——
///   `is_less` 恒真 → 交换 → 输出 `["model-bravo", "model-alpha"]`；
/// - **2b**（按与 id 升序的相反顺序喂入 `["model-bravo", "model-alpha"]`）：一个「**只在并列时**
///   返回 `Greater`」的第三档（记作 T）在这里红（实测 M7c）——`is_less` 恒 false → 稳定排序
///   保序 → 输出仍是 `["model-bravo", "model-alpha"]`。正确实现在 2b 上探的是
///   `is_less(alpha, bravo)`：并列时第三档交出 `Equal`、第四档判出 `Less` → 交换 → 得到期望。
///
/// **这两类在别处也被抓到，别把本用例读成它们的唯一照片**（实测）：T 同时在 `shuffling_…`
/// 与 `tied_…` 上红，「并列时 `Less`」也在 `shuffling_…` 上红——那两条的候选两两同族，
/// 并列档一旦不交 `Equal`，`is_less` 就恒真／恒假，它们的期望同样保不住。
/// 故补 2a / 2b 的价值是**上面那句措辞在本用例自己这里有照片**（此前要靠别处的用例替它担保），
/// 不是「这两类此前无人抓到」。
///
/// 措辞说准：**只在并列时不返回 `Equal`** 的第三档只会在这一侧（2a / 2b）暴露；一个**恒**返回
/// `Greater` 的第三档不在此列，它连方向一都过不去（机制与实测见下）。
///
/// **恒 `Greater`（M7b）的机制与实测**：恒 `Greater` ⇒ `is_less` 恒 `false` ⇒ 稳定排序保序，
/// 且第四档被遮蔽。故它对**任何**依赖排序的期望都保不住——实测 M7b 红在方向一
/// （`["model-alpha","model-bravo"]`，保序的结果）、`shuffling_…` 与 `tied_…`。
/// （它在本用例里停在方向一，2b 那句不会跑到；「2b 能抓 T」由 M7c 单独实测。）
///
/// 红的条件：删掉第三档（M7）、或把它写反（`CrossFamily` 在前）、或让它在并列时不交出 `Equal`。
#[test]
fn family_breaks_a_compatibility_and_confidence_tie() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    seed_candidate(&tx, "model-alpha", "0.5");
    seed_candidate(&tx, "model-bravo", "0.5");

    let models = vec![
        routable(&tx, "model-alpha", LifecycleState::Active),
        routable(&tx, "model-bravo", LifecycleState::Active),
    ];
    let request = request(
        FamilyPreference::Auto,
        availability(&[
            ("model-alpha", ProviderHealth::Healthy),
            ("model-bravo", ProviderHealth::Healthy),
        ]),
    );

    // 一、两族关系不同：同族的 `model-bravo` 在前，尽管它是 id 更大的那个。
    let split = StubPolicy::new(vec![
        ("model-alpha", 0.5, FamilyRelation::CrossFamily),
        ("model-bravo", 0.5, FamilyRelation::SameFamily),
    ]);
    let ranked = rank(&request, &models, &split).expect("两个候选都应过");
    let ordered: Vec<&str> = ranked
        .candidates()
        .iter()
        .map(|candidate| candidate.model().as_str())
        .collect();
    assert_eq!(
        ordered,
        vec!["model-bravo", "model-alpha"],
        "前两档全同时按 family 排：同族的在前，尽管它是 id 更大的那个"
    );

    // 二、把 `model-bravo` 也改成跨族：两族关系相同，第三档判不了，次序交还给兜底档的 id 升序。
    //     **两种输入顺序各来一遍**（同 `shuffling_the_input_does_not_change_the_output` 的形态）：
    //     2a 按 id 升序、2b 按与 id 升序相反的次序，期望**不变**。两类「并列时不交 `Equal`」的
    //     第三档分别落在两处：恒 `Less` 类在 2a 红，**只在并列时**返回 `Greater` 的 T 在 2b 红
    //     （它在 2a 上是绿的）——只留一种就漏掉一类。
    let both_cross = StubPolicy::new(vec![
        ("model-alpha", 0.5, FamilyRelation::CrossFamily),
        ("model-bravo", 0.5, FamilyRelation::CrossFamily),
    ]);
    for (label, order) in [
        ("2a 按 id 升序喂入", ["model-alpha", "model-bravo"]),
        ("2b 按与 id 升序相反喂入", ["model-bravo", "model-alpha"]),
    ] {
        let models: Vec<RoutableModel> = order
            .iter()
            .map(|model| routable(&tx, model, LifecycleState::Active))
            .collect();
        let ranked = rank(&request, &models, &both_cross).expect("两个候选都应过");
        let ordered: Vec<&str> = ranked
            .candidates()
            .iter()
            .map(|candidate| candidate.model().as_str())
            .collect();
        assert_eq!(
            ordered,
            vec!["model-alpha", "model-bravo"],
            "{label}：两族关系相同时第三档不决定次序，交付兜底档的 ModelId 升序（与输入顺序无关）"
        );
    }

    tx.commit().unwrap();
}

/// 同一个 `ModelId` 的两个候选 → `Err(RoutingError::DuplicateModelCandidate { id })`，
/// **断言是哪一枚、id 是哪一个**。
///
/// 它是 `compare` 的「全序」这条断言的守门人：两条 `ModelId` 相同的候选无从定序，
/// 兜底档也兜不住（设计 §5.3、§5.4）。两条候选由**同一行画像读两次**得到——它们的 `ModelId`
/// 相同而身份不同，这正是「同一个模型的两次打分」那条排除对象的形态。
///
/// 红的条件：去掉判重即红（`rank` 会给出一份「两条同名候选」的排序）。
#[test]
fn the_same_model_twice_is_rejected_with_the_id() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    seed_candidate(&tx, "model-alpha", "0.5");

    let models = vec![
        routable(&tx, "model-alpha", LifecycleState::Active),
        routable(&tx, "model-alpha", LifecycleState::Active),
    ];
    let policy = StubPolicy::new(vec![("model-alpha", 0.5, FamilyRelation::SameFamily)]);
    let request = request(
        FamilyPreference::Auto,
        availability(&[("model-alpha", ProviderHealth::Healthy)]),
    );

    assert_eq!(
        rank(&request, &models, &policy).err(),
        Some(RoutingError::DuplicateModelCandidate {
            id: ModelId::new("model-alpha")
        }),
        "同一个 ModelId 出现两次即无从定序，带出的是那个 id"
    );

    tx.commit().unwrap();
}

/// **一个候选都没有 → `Err(NoEligibleCandidate)`**（设计 §5.3、§5.4）。
///
/// 两份**互不相同**的 `rank` 输入汇到同一枚 `Err`：
/// 1. **空输入**（一个模型都没登记）：`(&empty, &[])`；
/// 2. **唯一候选不可用**——过闸门了，但被可用性过滤清空。
///
/// 中间夹着一条**闸门侧**的配对断言（`RoutableModel::try_new` 对 `stale` 失败）：它证明
/// 「被挡下的模型进不了 `&[RoutableModel]`」，故「全部被闸门挡下」这一路径**没有自己的
/// `rank` 输入**——候选切片只能是空的，它落到 `rank` 上的就是第 1 条那次调用（**逐字节相同**）。
/// 本用例不把它算作第三份输入。
///
/// `NoEligibleCandidate` **不合并进 `NotRoutable`**：前者是「没有可用的」，后者是
/// 「有一枚被点名挡下了」，调用方（§110 的流程）对两者的处置不同。
///
/// 红的条件：任一条路径返回别的 `Err`（或 `Ok`）即红。
#[test]
fn no_eligible_candidate_is_its_own_error() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    seed_candidate(&tx, "model-alpha", "0.5");
    let policy = StubPolicy::new(vec![("model-alpha", 0.9, FamilyRelation::SameFamily)]);

    // 一、空输入。
    let empty = request(FamilyPreference::Auto, Vec::new());
    assert_eq!(
        rank(&empty, &[], &policy).err(),
        Some(RoutingError::NoEligibleCandidate),
        "一个候选都没有时返回 NoEligibleCandidate，不是空列表"
    );

    // 二、闸门侧：`stale` 构造不出 `RoutableModel`，故「全部被闸门挡下」时候选集**只能是空的**
    //     ——它落回上面那次 `rank(&empty, &[])`，不构成另一份 `rank` 输入。
    assert_eq!(
        RoutableModel::try_new(profile_of(&tx, "model-alpha"), LifecycleState::Stale).err(),
        Some(RoutingError::NotRoutable {
            state: LifecycleState::Stale
        }),
        "闸门这一侧：漂移中的模型进不了候选集"
    );
    assert_eq!(
        rank(&empty, &[], &policy).err(),
        Some(RoutingError::NoEligibleCandidate),
        "候选切片为空（挡下的模型进不来），与「一个模型都没登记」是同一份输入、同一枚 Err"
    );

    // 三、唯一候选不可用：过闸门了，但被可用性过滤清空。
    let models = vec![routable(&tx, "model-alpha", LifecycleState::Active)];
    let filtered = request(
        FamilyPreference::Auto,
        availability(&[("model-alpha", ProviderHealth::Unavailable)]),
    );
    assert_eq!(
        rank(&filtered, &models, &policy).err(),
        Some(RoutingError::NoEligibleCandidate),
        "唯一候选被可用性过滤掉之后，也与前两条汇到同一枚 Err"
    );

    tx.commit().unwrap();
}

/// **`ProviderHealth::Unavailable` 的模型不进候选集**，且**两侧对钉**（设计 §5.3）。
///
/// 夹具让**不可用的那一个分数更高**（`model-bravo` 0.9 对 `model-alpha` 0.5）：若过滤不存在，
/// `selected()` 会是 `model-bravo`——故「表头是另一个」这条断言是承重的，不是碰巧成立。
///
/// 另一侧：把同一个候选由 `Unavailable` 改回 `Healthy` → 它**回到**输出里、并成为表头。
/// **缺了这一侧，一个「把所有候选都丢掉」的实现也全绿**（它同样让 `selected()` 是另一个？
/// 不——它会连 `model-alpha` 一起丢，返回 `NoEligibleCandidate`，见另一条用例；但只钉一侧
/// 就无法区分「正确地滤掉了一条」与「恰好只剩它」）。
///
/// 红的条件：去掉 `Unavailable` 那一档过滤即红（表头变成 `model-bravo`）。
#[test]
fn an_unavailable_model_is_not_a_candidate() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    seed_candidate(&tx, "model-alpha", "0.5");
    seed_candidate(&tx, "model-bravo", "0.5");

    let models = vec![
        routable(&tx, "model-alpha", LifecycleState::Active),
        routable(&tx, "model-bravo", LifecycleState::Active),
    ];
    let policy = StubPolicy::new(vec![
        ("model-alpha", 0.5, FamilyRelation::SameFamily),
        ("model-bravo", 0.9, FamilyRelation::SameFamily),
    ]);

    // 不可用的那个分数更高：过滤掉它之后表头才是另一个。
    let with_bravo_down = request(
        FamilyPreference::Auto,
        availability(&[
            ("model-alpha", ProviderHealth::Healthy),
            ("model-bravo", ProviderHealth::Unavailable),
        ]),
    );
    let ranked = rank(&with_bravo_down, &models, &policy).expect("还有一个可用的候选");
    assert_eq!(
        ranked.candidates().len(),
        1,
        "`Unavailable` 的那一个不在候选集里（它是「不可用即不是候选」，不是「降权」）"
    );
    assert_eq!(
        ranked.selected().model().as_str(),
        "model-alpha",
        "分数更高的那一个不可用，故表头是另一个"
    );

    // 另一侧：改回 `Healthy`，它回到输出里。
    let with_bravo_up = request(
        FamilyPreference::Auto,
        availability(&[
            ("model-alpha", ProviderHealth::Healthy),
            ("model-bravo", ProviderHealth::Healthy),
        ]),
    );
    let ranked = rank(&with_bravo_up, &models, &policy).expect("两个候选都应过");
    let ordered: Vec<&str> = ranked
        .candidates()
        .iter()
        .map(|candidate| candidate.model().as_str())
        .collect();
    assert_eq!(
        ordered,
        vec!["model-bravo", "model-alpha"],
        "改回 `Healthy` 之后它回到候选集里，并按分数成为表头"
    );

    tx.commit().unwrap();
}

/// **`Healthy` 与 `Degraded` 都留在候选集里，且两者之间排序不变**（设计 §5.3）。
///
/// 这一条是「**不发明降权判据**」的照片：设计写死只过滤 `Unavailable`，`Degraded` 该不该降权、
/// 降到什么程度，**规范未给判据**（§250 只说「考虑」、§84 没有给这一维的算法），
/// 故基线不为它改排序——健康度原样带进 `reason`，让策略自己决定（§11 第 24 条）。
///
/// 注意撞名：这里的 `ProviderHealth::Degraded` 是**供应商侧的可用性**，与
/// `LifecycleState::Degraded`（§249 的生命周期异常态）是两个轴上的两个东西。
///
/// 红的条件：给 `Degraded` 加一档降权（表头变化）或把它一并滤掉（少一条候选）即红。
#[test]
fn healthy_and_degraded_both_stay_and_the_order_does_not_change() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    seed_candidate(&tx, "model-alpha", "0.5");
    seed_candidate(&tx, "model-bravo", "0.5");

    let models = vec![
        routable(&tx, "model-alpha", LifecycleState::Active),
        routable(&tx, "model-bravo", LifecycleState::Active),
    ];
    let policy = StubPolicy::new(vec![
        ("model-alpha", 0.5, FamilyRelation::SameFamily),
        ("model-bravo", 0.9, FamilyRelation::SameFamily),
    ]);

    let fingerprints: Vec<Vec<Fingerprint>> = [ProviderHealth::Healthy, ProviderHealth::Degraded]
        .iter()
        .map(|health| {
            let request = request(
                FamilyPreference::Auto,
                availability(&[("model-alpha", ProviderHealth::Healthy), ("model-bravo", *health)]),
            );
            fingerprint(&rank(&request, &models, &policy).expect("两个候选都应过"))
        })
        .collect();

    assert_eq!(fingerprints[0].len(), 2, "两条候选都留着（`Degraded` 不被滤掉）");
    assert_eq!(
        fingerprints[0], fingerprints[1],
        "健康度在 Healthy 与 Degraded 之间来回改，两次输出逐项相同"
    );

    tx.commit().unwrap();
}

/// 候选集里的某个 `ModelId` 在 `RoutingRequest::availability` 里**没有条目** →
/// `Err(RoutingError::UnknownAvailability { id })`，**断言是哪一枚、id 是哪一个**。
///
/// **不当作可用**——按未知放行是 fail-open 的形状（设计 §5.3）。
///
/// 红的条件：把「列表里没有」当成「可用」而放行即红。
#[test]
fn a_candidate_missing_from_availability_is_rejected() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    seed_candidate(&tx, "model-alpha", "0.5");
    seed_candidate(&tx, "model-bravo", "0.5");

    let models = vec![
        routable(&tx, "model-alpha", LifecycleState::Active),
        routable(&tx, "model-bravo", LifecycleState::Active),
    ];
    let policy = StubPolicy::new(vec![
        ("model-alpha", 0.5, FamilyRelation::SameFamily),
        ("model-bravo", 0.9, FamilyRelation::SameFamily),
    ]);
    // `model-bravo` 这一条**刻意不给**。
    let request = request(
        FamilyPreference::Auto,
        availability(&[("model-alpha", ProviderHealth::Healthy)]),
    );

    assert_eq!(
        rank(&request, &models, &policy).err(),
        Some(RoutingError::UnknownAvailability {
            id: ModelId::new("model-bravo")
        }),
        "列表里没有条目即「未知」，带出的是缺条目的那个 id"
    );

    tx.commit().unwrap();
}

/// **与闸门那一侧配对的正面照片**：一个 `Active` 的模型经 `rank` **确实成为 `selected()`**。
///
/// 只钉「`stale` 被挡下」而不钉这一侧，整条路由路径可以在「永远返回 `NoEligibleCandidate`」
/// 的情况下全绿——而那正是 fail-open 的反面（设计 §5.3 的可用性过滤、`## 遗留` 的同一判据）。
///
/// 红的条件：`rank` 对合法输入返回 `Err`，或表头不是那个 `Active` 的模型。
#[test]
fn a_routable_model_does_come_out_as_the_selected_candidate() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();

    seed_candidate(&tx, "model-alpha", "0.5");
    let models = vec![routable(&tx, "model-alpha", LifecycleState::Active)];
    let policy = StubPolicy::new(vec![("model-alpha", 0.5, FamilyRelation::SameFamily)]);
    let request = request(
        FamilyPreference::Auto,
        availability(&[("model-alpha", ProviderHealth::Healthy)]),
    );

    let ranked = rank(&request, &models, &policy).expect("一个 Active 的模型应真的被选中");
    assert_eq!(ranked.selected().model().as_str(), "model-alpha");
    assert_eq!(ranked.selected().state(), RoutableState::Active);

    tx.commit().unwrap();
}
