//! §250 的**请求面**（设计 `docs/superpowers/specs/2026-10-05-p3d-model-registry-router-design.md` §5.1）。
//!
//! 本模块只有**纯数据**：一次自动路由的请求，以及它装的三样东西。
//! **一次 I/O 都没有，一个 `Tx` 都不接**——候选排序（`rank`）是同步纯函数，
//! 它的输入全在这里与 [`crate::lifecycle::RoutableModel`]（设计 §1.2、§5.3）。
//!
//! # 请求与策略的分界是刻意的
//!
//! §250 的六个输入落在**两侧**（设计 §5.1 的那张表）：
//!
//! | §250 的输入 | 落在 | 理由 |
//! |---|---|---|
//! | `TaskSkillRequirement` | **请求** | 每次路由都不同，且是排序的输入 |
//! | `ModelProfile` | 请求的**间接**成分（经 `&[RoutableModel]` 传入，**不在** [`RoutingRequest`] 里） | 它是**候选集**，不是单次请求的参数 |
//! | `FamilyPreference` | **请求** | §19 的封闭清单，纯数据 |
//! | `CostPolicy` / `LatencyPolicy` / `FailureHistory` | **策略**（今天不定义形状） | 规范只给名字，且它们是「怎么算」而不是「算什么」；`FailureHistory` 连产生方都没有指定 |
//!
//! **请求进得去表驱动用例，策略可以换一份重跑同一组用例。** 故本模块**不出现**那三个策略输入
//! 的任何字段或类型——给 [`RoutingRequest`] 加上它们，等于把一个未定义的形状钉在请求面上。
//!
//! # 本模块的类型**不派生任何东西**，判据与 [`crate::budget::BudgetView`] 同
//!
//! 它们是**纯数据**：字段 `pub`（或只有一条构造闸门），从构造到读回之间**没有本 crate 的代码**，
//! 故「读回来还是原值吗」这类运行期断言必然恒真。派生的判据是**有没有当场消费方**：
//! 本模块的**三个**类型（[`TaskSkillRequirement`] / [`FamilyPreference`] / [`RoutingRequest`]）
//! 今天一个消费者都没有（`rank` 在 Task 11），故 `Debug` / `PartialEq` 等
//! 一律不加。**要加的那一天，连同一个真用得上的用例一起加。**

use continuum_core::model::{ModelId, ProviderHealth};

use crate::budget::BudgetView;
use crate::error::RequirementError;
use crate::profile::SkillDimension;

/// **非空**的 §248 维度集合：一次路由请求需要这个任务具备哪几个维度。
///
/// §250 只给了这个名字，形状是本设计定的（依据是 §250 的「能力匹配」以 §248 的向量为基准）。
///
/// # 字段私有，空的集合在构造期就被拒
///
/// **crate 外**唯一的构造入口是 [`TaskSkillRequirement::try_new`]，**空集合返回
/// [`RequirementError::RequirementEmpty`]**。射程要说准：字段的私有性是**模块级**的，
/// 故本模块自己的代码仍能直接写字面量——被挡住的是**crate 外**（含 `tests/`，那是独立的 crate），
/// 而两处照片（`tests/router.rs` 与 `tests/compile_fail/a_requirement_cannot_be_built_*.rs`）
/// 的观察点也都在 crate 外。
/// 这不是一条风格约定，是**堵一个 `0/0`**：
/// `compatibility = matched / required`（§84）在 `required` 为空时是 `0/0`，
/// 而 `CandidateScore.compatibility: Ratio` **拒 NaN**、`evaluate` **不返回 `Result`**——
/// 实现者只剩 panic 或编一个值两条路，**两条都能过 §9 原有的全套用例**（设计 §5.1 末段）。
/// 类型层拒掉它比替它选一个数诚实。照片：`tests/router.rs` 的
/// `an_empty_requirement_is_rejected`（断言是哪一枚 `Err`），以及 `tests/compile_fail/` 里
/// **两份**样例——`a_requirement_cannot_be_built_from_a_bare_vec.rs`（字段私有，E0451）与
/// `a_requirement_cannot_be_built_by_conversion.rs`（无 `From<Vec<_>>`，E0277）。
/// **两条通道分开成两份文件**：实测 rustc 只报同一函数体里的第一条错，
/// 合并成一份会让其中一条通道没有照片。
///
/// # 不带权重、不带阈值
///
/// 两者都是**规范没有的数**，加了就是发明（设计 §5.1）：§250 只说「能力匹配」，
/// 没给任何一个维度的权重，也没给「命中几个算够」的阈值。故本类型只是**维度的一个集合**，
/// 匹配的算法（谁命中、缺哪几个）归排序策略，`missing` 也**不折算成扣分**
/// ——那同样需要一个规范没有的权重（设计 §5.2）。
pub struct TaskSkillRequirement {
    dimensions: Vec<SkillDimension>,
}

impl TaskSkillRequirement {
    /// 从一个维度集合构造。**空集合 → `Err(RequirementError::RequirementEmpty)`**。
    ///
    /// **顺序原样保留**：本函数不排序、不去重。需求里维度的次序不是本层的信息
    /// （匹配是一个集合运算），但「原样保留」是这里唯一不需要解释的选择——
    /// 排一次序就要为「凭什么按这个序」找出处，而规范没给。
    ///
    /// **收 `Vec` 而不是切片**：它要把这批维度**存下来**（`RoutingRequest` 里带着它走），
    /// 故此处必然发生一次所有权转移，收切片只会把那次拷贝推到调用方看不见的地方。
    pub fn try_new(dimensions: Vec<SkillDimension>) -> Result<Self, RequirementError> {
        if dimensions.is_empty() {
            return Err(RequirementError::RequirementEmpty);
        }
        Ok(Self { dimensions })
    }

    /// 这批维度，**顺序与内容与入参逐一相同**（`try_new` 不排序、不去重）。
    ///
    /// 返回切片而不是 `&Vec<_>`：调用方要的是「读这批维度」（策略对它们做集合运算），
    /// 不需要本类型容器的那几个方法。
    pub fn dimensions(&self) -> &[SkillDimension] {
        &self.dimensions
    }
}

/// §19 的模型 family 偏好（`docs/spec/01-concepts.md:933-960` 的**封闭清单**，照录五项）。
///
/// §19 的原文是五项：`Auto` / `OpenAI preferred` / `Claude preferred` / `Local preferred` / `Custom`，
/// 默认优先使用同一 model family，「只有明显收益足够高时才跨 family」。
///
/// # `Custom` 不带载荷
///
/// §19 里 `Custom` 就是这一个词，本设计**也不给它加**载荷（设计 §5.1）：
/// 「自定义什么」规范没有说，猜一个形状就等于发明。
///
/// # §19 的「明显收益足够高」**没有阈值**
///
/// 那一句是 §19 的正文，但它没给任何数。本层因此**不实现**这条判断：
/// 偏好原样进 [`RoutingRequest`]，阈值留给策略，见设计的 §11 第 10 条（收件人：规范维护者）。
///
/// # 落库编码与类型同址，**但本阶段不落库**
///
/// [`FamilyPreference::as_str`] 给出的编码与 `SkillDimension` / `LifecycleState` 同一张表
/// （小写、多词以 `_` 连接）。本 crate 的三张表里**没有 family 这一列**（`crate::persist` 的迁移 80），
/// 本阶段这个值只作路由的输入。编码函数仍然写在这里，是因为全局约束要求
/// **编码挂在类型上**、不在 `persist.rs` 里另建一张表：将来给它建列的那个 task 直接用这一对函数。
pub enum FamilyPreference {
    /// 不指定 family，由 Router 自行决定（§19 的默认档）。
    Auto,
    /// 优先 OpenAI 一族的模型。
    OpenAiPreferred,
    /// 优先 Claude 一族的模型。
    ClaudePreferred,
    /// 优先本地部署的模型。
    LocalPreferred,
    /// 用户自定义。**§19 里它不带载荷**，本类型也不加。
    Custom,
}

impl FamilyPreference {
    /// §19 五项的落库编码：**小写，多词以 `_` 连接**。**本函数是该编码唯一的产生点。**
    ///
    /// match 穷尽且无通配臂：加第六项时本函数编译不过，编码不会漏分支。
    /// （手写的那张编码表在 `tests/router.rs` 的 `ALL_FAMILIES`，五项逐项钉住格式。）
    ///
    /// **`OpenAiPreferred` 的编码是 `openai_preferred`，不是 `open_ai_preferred`**：
    /// 编码的基准是 **§19 的短语**（"OpenAI preferred"），不是 Rust 变体名的驼峰切分——
    /// `OpenAI` 在那份封闭清单里是**一个词**（与 `ToolUse` ← "tool use"、
    /// `ConstraintFollowing` ← "constraint following" 同一条规则）。这条区分写在这里，
    /// 免得后来者把它读成打错了字。
    ///
    /// **不提供 `parse`**：本阶段没有任何读取方（这一列不存在），
    /// 一份没有消费方的逆向表就是「提前铺开的 API」——它的每一条臂都不会被跑到，
    /// 而本仓对「声明了没有产生方（或消费方）的东西」一贯的处置是删或写明理由（设计 §6.1）。
    /// 与它同族的 `SkillDimension::parse` 有消费方（`crate::persist` 的读取路径），故那边有。
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::OpenAiPreferred => "openai_preferred",
            Self::ClaudePreferred => "claude_preferred",
            Self::LocalPreferred => "local_preferred",
            Self::Custom => "custom",
        }
    }
}

/// 一次自动路由的请求。**纯数据，无 I/O**——四个字段都是 `pub`，
/// 由驱动装配（它本来就同时依赖语义层与本层，见 [`BudgetView`] 的模块头）。
///
/// 四个字段逐项：§250 #1 的能力需求、§250 #5 的 family 偏好、§250 的「当前可用性」，
/// 以及 ENG-005 的预算视图。**没有 accessor**：字段是 `pub` 的纯数据投影，
/// 加一对 `fn requirements(&self)` 只会让同一件事有两个落点（与 `BudgetView` 同一判据）。
pub struct RoutingRequest {
    /// §250 #1 的能力需求（非空，见 [`TaskSkillRequirement`]）。
    pub requirements: TaskSkillRequirement,

    /// §250 #5 的 family 偏好（§19 的封闭清单）。
    pub family: FamilyPreference,

    /// §250 的「当前可用性」：模型 → §315 的 [`ProviderHealth`] 快照。
    ///
    /// **它不是「这个模型的生命周期状态」**：这里的 `ProviderHealth::Degraded` 是
    /// **供应商侧的可用性**，与 [`crate::lifecycle::LifecycleState::Degraded`]
    /// （§249 的模型生命周期异常态）是**两个轴上的两个东西**，只是名字撞了（设计 §4.2 末段）。
    /// 生命周期状态走的是 [`crate::lifecycle::RoutableModel`]，不在这条列表里。
    ///
    /// **这两个取值怎么用不在本 task**：设计 §5.3 写死「只过滤 `Unavailable`」，
    /// 那条过滤与「列表里没有条目的候选」的失败路径都在 Task 11 的候选集构造里。
    /// 本层只保证它们**装得进请求、且彼此可分辨**。
    pub availability: Vec<(ModelId, ProviderHealth)>,

    /// ENG-005 的预算视图。**必填，不是 `Option`。**
    ///
    /// 这一条是「§250 的 MUST 考虑成本」在本设计里的**全部兑现**：结构性地不可省略，
    /// 故一个策略想忽略预算，是一次**看得见的选择**，而不是一次遗漏（设计 §5.3 末段）。
    /// **它还不是「已实现」**——量值算不出来（§6：量纲无单位），语义层未建，
    /// 每个 `Some` 今天都只能由测试构造（设计 §10 第 2 条）。
    /// 照片形态因此是**签名层**的：`tests/compile_fail/a_routing_request_without_a_budget.rs`。
    pub budget: BudgetView,
}
