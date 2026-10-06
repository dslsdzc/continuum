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
    BudgetView, FamilyPreference, RequirementError, RoutingRequest, SkillDimension,
    TaskSkillRequirement,
};

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
