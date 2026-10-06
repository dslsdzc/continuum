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
//! # 请求面的三个类型**不派生任何东西**，判据与 [`crate::budget::BudgetView`] 同
//!
//! 它们是**纯数据**：字段 `pub`（或只有一条构造闸门），从构造到读回之间**没有本 crate 的代码**，
//! 故「读回来还是原值吗」这类运行期断言必然恒真。派生的判据是**有没有当场消费方**：
//! Task 10 落地时本模块的**三个**类型（[`TaskSkillRequirement`] / [`FamilyPreference`] /
//! [`RoutingRequest`]）一个消费者都没有，故 `Debug` / `PartialEq` 等一律不加；
//! **Task 11 给 [`RoutableModel`] 这一侧接上了消费者，而下面那三个新类型仍然不派生**
//! ——`rank` 的用例走的是访问器，不是整值比对。**要加的那一天，连同一个真用得上的用例一起加。**
//!
//! **唯一的例外是 [`FamilyRelation`]**：`#[derive(...)]` 那张清单写了**五项**，
//! 逐项按「有没有当天用得上的消费方」判，**留四项、去掉一项**：
//!
//! | 派生 | 当天消费方 |
//! |---|---|
//! | `Debug` | `tests/router.rs` 的 `assert_eq!`——断言「是哪一枚」时要打印两边的值 |
//! | `PartialEq` | 同上，它就是 `assert_eq!` 的比较本身 |
//! | `Copy` | [`RoutingReason::family`] **按值**返回它（与 [`crate::lifecycle::RoutableState`] 的 `state()` 同形；不 `Copy` 则那一行是 E0507） |
//! | `Clone` | **没有直接消费方**；留着的真因是它是 `Copy` 的编译期前提（`Copy: Clone`），不是「将来可能有人用」 |
//! | `Eq` | **没有**——`assert_eq!` 只要 `Debug` ＋ `PartialEq`，故**不派生**（判据与 [`BudgetView`] / [`RoutableModel`] 同：没有消费方就不加） |
//!
//! 这是本模块（`router.rs`）**唯一**带派生的类型，不是一次口味上的放宽。
//! **这句是声明性的，没有编译期照片**：Rust 没有反射，「本模块只有这一个类型带派生」拍不成
//! 用例。能拍的是它的逐类型对偶——请求面那三个类型各自不带派生，由它们各自的用例钉住
//! （走访问器而不是整值比对）；可机械复核的是
//! `grep -n '^#\[derive' crates/continuum-model-registry/src/router.rs` 在本次改动后**只有一行**。
//!
//! # Task 11 的落点（设计 §5.2、§5.3、§5.4）
//!
//! [`rank`] 是**同步纯函数**：候选集构造（判重 → 查可用性 → 只滤掉 `Unavailable`）、
//! 逐个 [`RankingPolicy::evaluate`]、`sort_by` [`RankingPolicy::compare`]、取头。
//! 输出面是 [`RankedExecutionCandidates`]（字段私有，唯一的构造点在 `rank` 内），
//! 排序的**机制**在本层，**具体打分函数的数值明确推迟**（设计 §5.3 的三条依据），
//! 具名基线是 Task 12 的 [`BaselineRankingPolicy`]。
//!
//! # Task 12 的落点（设计 §5.3 的「第二步」）
//!
//! [`BaselineRankingPolicy`] 是本层交付的**具名基线**，**不是对规范的声称**：§84 未给打分函数，
//! 故它是一个**可替换的实现**（设计 §5.3）。它只读**已定义**的输入——§248 维度的**有无**、
//! §247 的 `confidence`、§19 的家族偏好、§249 的状态——并**用缺省的 [`RankingPolicy::compare`]**，
//! 故「全序」这件事仍由本层的那四档负责。**两处选择写在该类型的文档里**：家族由 `provider` 判
//! （§19 未给判据），以及被否掉的两个打分法。

use std::cmp::Ordering;

use continuum_core::model::{ModelId, ProviderHealth};

use crate::budget::BudgetView;
use crate::error::{RequirementError, RoutingError};
use crate::lifecycle::{RoutableModel, RoutableState};
use crate::profile::{Ratio, SkillDimension};

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

// ===== Task 11：输出面与 `rank`（设计 §5.2、§5.3、§5.4） =====

/// §19 的两族关系，落在候选的 `reason` 里（设计 §5.2、§5.3）。
///
/// # 它的定义归本 task（协调者裁定，2026-10-06）
///
/// 原稿把 `pub enum FamilyRelation { SameFamily, CrossFamily }` 写在 **Task 12** 的代码块里，
/// 而本模块的 [`RoutingReason`] 已经用它——**一个类型不能在它存在之前就被引用**，照原稿执行
/// 本 task 编不过（E0412／E0432）。判据是「[`FamilyRelation`] 是 [`RoutingReason`] 的字段类型，
/// 定义随宿主走」，且本 task 的 Interfaces 一栏已认领 [`RoutingReason`]。
/// 两枚变体的名字取自设计 `:793` / 计划 `:1310`，**不另取**；Task 12 **不得再定义一次**。
///
/// # 它是比较的第三档
///
/// [`RankingPolicy::compare`] 的缺省实现里，`SameFamily` 排在 `CrossFamily` 前面
/// （§19「优先同一 model family」）。**顺序的判据写在一个显式的 rank 函数里**，
/// 不靠变体的声明序——声明序是一种「没说出口的规格」。
///
/// # 派生集（逐项判据见模块头的那张表）
///
/// 五项里**留四项**：`Debug` / `PartialEq` 供用例断言「是哪一枚」（`assert_eq!` 两者都要）；
/// `Copy` 供 [`RoutingReason::family`] 按值返回；`Clone` **没有直接消费方**，留着的真因是
/// 它是 `Copy` 的编译期前提（`Copy: Clone`）；`Eq` **没有消费方，故不派生**——`assert_eq!`
/// 只要 `Debug` ＋ `PartialEq`。「两个变体之外没有第三个取值」只是**能派生 `Eq` 的前提**，
/// 不是**有消费方**的证据，两者别混。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FamilyRelation {
    /// 候选与请求偏好同族（§19 的 `Auto` 下全部视为这一档，那一条落在 Task 12）。
    SameFamily,
    /// 候选跨族。
    CrossFamily,
}

/// [`FamilyRelation`] 在 [`RankingPolicy::compare`] 里的序：`SameFamily` 在前，故它的秩最小。
///
/// **不派生 `Ord`**：那会让次序由变体的声明序给出，而声明序是一种没说出口的规格。
/// match 穷尽且无通配臂：加第三枚时本函数编译不过，排序不会漏分支。
fn family_rank(family: FamilyRelation) -> u8 {
    match family {
        FamilyRelation::SameFamily => 0,
        FamilyRelation::CrossFamily => 1,
    }
}

/// §84 的 reason。**结构化而非一行文本**：字段要供后续对账取用（设计 §5.2）。
///
/// # 字段私有，但有一个公开的构造入口
///
/// 四个字段私有，读口是四个访问器——它们**今天没有具名消费方**，只是 §84 要输出的内容的读口
/// （设计 §5.2 刻意把这句话写得比 [`ExecutionCandidate`] 那五个访问器弱）。
///
/// **然而 [`CandidateScore`] 的字段是 `pub` 的、[`RankingPolicy::evaluate`] 是公开 trait 的方法**，
/// 故 crate 外的策略实现者（以及本 crate 的测试策略）**必须**能造出一枚 [`RoutingReason`]：
/// 没有构造入口，`RankingPolicy` 这个「可替换的基线」就只是一个实现不出来的 trait。
/// 故有 [`RoutingReason::new`]——**这是接口冻结处的必需品，不是提前铺开的 API**。
///
/// 它不做任何校验：四个字段都是纯数据（`FamilyRelation` 是枚举、两个 `Vec` 的元素类型各自
/// 在构造期已把关），没有可再失败的判据，故不发明一个永不出现的 `Err`
/// （同 [`crate::profile::ModelProfile::try_new`] 的处置）。
pub struct RoutingReason {
    family: FamilyRelation,
    matched: Vec<SkillDimension>,
    missing: Vec<SkillDimension>,
    notes: Vec<String>,
}

impl RoutingReason {
    /// 四个字段全量传入，**顺序原样保留**（两个 `Vec` 不排序、不去重）：匹配是一个集合运算，
    /// 但「原样保留」是这里唯一不需要解释的选择——排一次序就要为「凭什么按这个序」找出处，
    /// 而规范没给（同 [`TaskSkillRequirement::try_new`] 的理由）。
    pub fn new(
        family: FamilyRelation,
        matched: Vec<SkillDimension>,
        missing: Vec<SkillDimension>,
        notes: Vec<String>,
    ) -> Self {
        Self {
            family,
            matched,
            missing,
            notes,
        }
    }

    /// §19 的两族关系。按值返回（它是 `Copy` 的两变体枚举，与 [`RoutableState`] 同形）。
    pub fn family(&self) -> FamilyRelation {
        self.family
    }

    /// 命中的需求维度。返回切片：调用方要的是「读这批维度」，不是本类型容器的那几个方法。
    pub fn matched(&self) -> &[SkillDimension] {
        &self.matched
    }

    /// **未命中**的需求维度。**是 `Vec<SkillDimension>`，不是一个数值**——本设计**不把
    /// 「缺一个维度」折算成任何扣分**，因为那需要一个规范没有的权重（设计 §5.2）。
    pub fn missing(&self) -> &[SkillDimension] {
        &self.missing
    }

    /// 策略写入的事实陈述。**来源是策略**：本层不替它写一句（那会是一条本层没有判据的断言）。
    pub fn notes(&self) -> &[String] {
        &self.notes
    }
}

/// 一个候选的打分。**没有总分字段**——§248 的「不依赖单一总分」在接口上也看得见（设计 §5.3）。
///
/// 字段 `pub`：它是 [`RankingPolicy::evaluate`] 的返回值，由**策略**（crate 外也可以是）构造，
/// 故 `pub` 是接口的一部分，不是「懒得写访问器」。
pub struct CandidateScore {
    /// §84 的 compatibility。含义规范未定义（§84 只说「A 更合理」），本层只保证它是 `[0,1]` 的
    /// 已构造 [`Ratio`]——故 [`RankingPolicy::compare`] 的前两档**可以假定**操作数合法。
    pub compatibility: Ratio,
    /// §84 的 confidence。取值与含义同上。
    pub confidence: Ratio,
    /// §84 的 reason。
    pub reason: RoutingReason,
}

/// 一个**可执行**的候选：模型、已过闸门的状态、两个 §84 的读数与 reason。
///
/// 字段私有且**只有 `rank` 构造它**——故「候选」这个词在本层与「过了闸门且可用」是同一件事。
///
/// # 五个访问器是接口冻结处的必需品
///
/// [`Self::model`] 是给**消费者**的（子项目 G）：消费者下一步就是拿着这个 `ModelId` 去
/// 调 provider，拿不到这一步就走不下去。其余四个同理（设计 §5.2）。**不是「将来可能有人用」。**
pub struct ExecutionCandidate {
    model: ModelId,
    state: RoutableState,
    compatibility: Ratio,
    confidence: Ratio,
    reason: RoutingReason,
}

impl ExecutionCandidate {
    /// §247／§315 的模型 id。消费方据此发起调用。
    pub fn model(&self) -> &ModelId {
        &self.model
    }

    /// **过闸门之后**的六态值（不是十态：被挡下的四态进不到这里，见
    /// [`crate::lifecycle::RoutableModel`]）。消费者据此决定是否降权／重试（§4.2 末段）。
    pub fn state(&self) -> RoutableState {
        self.state
    }

    /// §84 的 compatibility。
    pub fn compatibility(&self) -> Ratio {
        self.compatibility
    }

    /// §84 的 confidence。
    pub fn confidence(&self) -> Ratio {
        self.confidence
    }

    /// §84 的 reason。
    pub fn reason(&self) -> &RoutingReason {
        &self.reason
    }
}

/// §250 的输出：一份**有序**的候选列表（设计 §5.2）。
///
/// # 字段私有，唯一的构造点在 [`rank`] 内，且构造前先判空
///
/// 这条保证撑起 [`Self::selected`] 的签名：无候选时 `rank` 返回
/// `Err(RoutingError::NoEligibleCandidate)` 而**不返回空列表**，故 `selected()` 不必返回
/// `Option`——§84 的 `selected_model` 总是存在。不可表达性命题的照片是
/// `tests/compile_fail/ranked_candidates_cannot_be_built.rs`（字段私有，E0451）。
///
/// # 不另设 `alternatives` 字段
///
/// §84 的 `alternatives` **不是第二份数据**，是**同一份有序列表的表尾**：它是
/// [`Self::alternatives`] 这个访问器算出来的，不是存下来的（同一件事两个落点，设计 §5.2）。
pub struct RankedExecutionCandidates {
    candidates: Vec<ExecutionCandidate>,
}

impl RankedExecutionCandidates {
    /// §84 的 `selected_model`：表头。**不返回 `Option`**——列表非空是构造期的保证（见类型文档）。
    ///
    /// 索引 `[0]` 而不是 `.first()`：`.first()` 会给出一个编译期就消除了的 `None` 分支，
    /// 而那个分支不可能被走到；这里的下标由「构造前先判空」兜住。
    pub fn selected(&self) -> &ExecutionCandidate {
        &self.candidates[0]
    }

    /// §84 的 `alternatives`：**同一份列表**的表尾（可能为空——只有一个候选时就是空切片）。
    pub fn alternatives(&self) -> &[ExecutionCandidate] {
        &self.candidates[1..]
    }

    /// 整份有序列表。调用方要「按序看全部候选」（表头在那一段用例里是单独读的）时用它。
    pub fn candidates(&self) -> &[ExecutionCandidate] {
        &self.candidates
    }
}

/// 排序的**机制与接口**在本层；**具体打分函数的数值明确推迟**（设计 §5.3 的三条依据：
/// §84 未给两个读数的算法与含义、§25 的 Population Feedback 是 OPEN-008、
/// §247／§87／§333 的量纲没有单位）。
///
/// 具名基线（`BaselineRankingPolicy`）是 Task 12；本 trait 的存在让「换一份策略重跑同一组用例」
/// 成为可能（设计 §5.1 的请求／策略分界）。
pub trait RankingPolicy {
    /// 给一个**已过闸门**的候选打分。收 `&RoutableModel`（要读画像）与 `&RoutingRequest`
    /// （要读需求与偏好）。
    ///
    /// **不返回 `Result`**：这条签名是 [`TaskSkillRequirement`] 非空那条保证的理由之一——
    /// 空需求会让 `compatibility = matched / required` 成为 `0/0`，而 `Ratio` 拒 NaN，
    /// 实现者只剩 panic 或编一个值两条路（设计 §5.1 末段）。空需求在**构造期**就被拒，到不了这里。
    fn evaluate(&self, request: &RoutingRequest, model: &RoutableModel) -> CandidateScore;

    /// **全序**比较：`Ordering::Less` 表示 `a` 排在 `b` 前面。
    ///
    /// 缺省实现按 `(compatibility, confidence)` 降序，再按 family、model id 兜底：
    ///
    /// 1. `compatibility` 降序；
    /// 2. `confidence` 降序；
    /// 3. [`FamilyRelation`]：`SameFamily` 在前（§19 的「优先同一 model family」）；
    /// 4. [`ModelId`] **升序**——**全序的最后兜底**。
    ///
    /// 操作数就是 [`ExecutionCandidate`]——**不另立一个 `Scored` 类型**（同一件事的第二个落点），
    /// 缺省实现经 §5.2 的访问器读数，不动私有字段。
    ///
    /// # 「全序」由三件事合起来成立（逐条落实）
    ///
    /// - 前两档可比：[`Ratio`] 在**构造期**就拒 NaN 与 ±∞，故两个已构造的取值必有确定的大小关系
    ///   （`tests/profile.rs` 的 `ratio_rejects_non_finite_and_out_of_range`）；
    /// - 第三档：`FamilyRelation` 只有两枚，[`family_rank`] 给出确定的秩；
    /// - 第四档是**最后兜底**：两个 `ModelId` 两两可比且**无相等**，故比较不会以 `Equal` 收尾
    ///   （除非两条候选本就是同一个模型）。而**同一个模型的两次打分由建候选集时判重排除**——
    ///   `rank` 对重复的 id 返回 `Err(RoutingError::DuplicateModelCandidate { id })`。
    ///
    /// # 为什么本处**没有** NaN 的排序用例（写成「为什么没有」，免得被读成漏项）
    ///
    /// [`crate::profile::Ratio`] 的文档里写着「NaN 会让 `sort_by` 不是全序」——那句理由是对的，
    /// 但**它在 [`Ratio`] 上拍不到**：NaN 在构造期就被拒，**进不到 `compare` 的入参里**
    /// （两个操作数都是已构造的 `Ratio`）。故本处造不出这条用例，也**不该**造——
    /// 它真正的落点是**构造期**，照片就是上面那条 `ratio_rejects_non_finite_and_out_of_range`。
    /// 前两档因此**可以假定**操作数是合法 `Ratio`。
    ///
    /// # 比较用的是 `total_cmp` 而不是 `partial_cmp`
    ///
    /// 两个取值都是有限实数（构造期的保证），`partial_cmp` 与 `total_cmp` 在它们上给出相同的序，
    /// 唯一的分歧格是 `-0.0` 与 `0.0`（`PartialEq` 判它们相等，`total_cmp` 判前者小）。
    /// 取 `total_cmp` 是为了让这一档**自身**也是全序、不出现「按 `Eq` 相等而按 `cmp` 不等」的
    /// 半序收尾——排序要的是确定的结果，不是与 `Eq` 对齐。
    fn compare(&self, a: &ExecutionCandidate, b: &ExecutionCandidate) -> Ordering {
        b.compatibility()
            .get()
            .total_cmp(&a.compatibility().get())
            .then_with(|| b.confidence().get().total_cmp(&a.confidence().get()))
            .then_with(|| family_rank(a.reason().family()).cmp(&family_rank(b.reason().family())))
            .then_with(|| a.model().as_str().cmp(b.model().as_str()))
    }
}

/// §250 的路由：**建候选集 → 逐个打分 → 排序 → 取头**（设计 §5.3 的骨架）。
///
/// 纯函数：不接 `Tx`、不做 I/O（设计 §1.2）。拿到的是**已过闸门**的模型——被挡下的四态
/// （`unprofiled` / `stale` / `quarantined` / `disabled`）在类型上就进不到这个签名里
/// （[`RoutableModel::try_new`]），故 `NotRoutable` 到不了这里。
///
/// # 三条 `Err` 的判定次序
///
/// 本实现取「**判重 → 可用性 → 空判定**」。**次序没有判据**（设计未定），故本 crate 的用例
/// 都不构造两条 `Err` 同时可能的输入——换一个次序不会让哪条用例变绿或变红。这是刻意的：
/// 次序没有判据，用例就不该依赖它。
///
/// # 建候选集这一步的三件事，逐条写死（设计 §5.3）
///
/// 1. **判重**：同一个 `ModelId` 出现两次 → [`RoutingError::DuplicateModelCandidate`]。
///    它不只是去重：两条同 id 的候选**无从定序**（`compare` 的兜底档也兜不住），
///    故这一条是「全序」那条断言的守门人。
/// 2. **可用性**：候选在 [`RoutingRequest::availability`] 里**没有条目** →
///    [`RoutingError::UnknownAvailability`]，**不当作可用**——按未知放行是 fail-open 的形状。
/// 3. **只滤掉 `ProviderHealth::Unavailable`**，且这一条是「**不可用即不是候选**」而不是
///    「可用性低就降权」：本函数的输出是 `RankedExecutionCandidates`，即**可执行的**候选，
///    排进去一个供应商侧已不可用的模型，「拿它跑」那一步必然失败。
///    **`Healthy` 与 `Degraded` 都进候选集，两者之间没有判据**——`Degraded` 该不该降权、
///    降到什么程度规范未给判据（§250 只说「考虑」、§84 没有给这一维的算法），故本层不发明：
///    状态由 [`ExecutionCandidate::state`] 与候选的 `reason` 原样带出，让策略自己决定
///    （§11 第 24 条）。
///
/// # 空判定在最后
///
/// 筛完之后一个候选都没有（输入为空、或全被可用性过滤掉）→
/// [`RoutingError::NoEligibleCandidate`]，**不返回空列表**——那是
/// [`RankedExecutionCandidates::selected`] 不必返回 `Option` 的前提。
/// 它**不合并进** [`RoutingError::NotRoutable`]：前者是「没有可用的」，后者是
/// 「有一枚被点名挡下了」，调用方（§110 的流程）对两者的处置不同（设计 §5.4）。
pub fn rank(
    request: &RoutingRequest,
    models: &[RoutableModel],
    policy: &dyn RankingPolicy,
) -> Result<RankedExecutionCandidates, RoutingError> {
    // 一、判重：两条同 id 的候选无从定序（见函数文档）。
    let mut seen: Vec<&ModelId> = Vec::with_capacity(models.len());
    for model in models {
        let id = model.profile().id();
        if seen.contains(&id) {
            return Err(RoutingError::DuplicateModelCandidate { id: id.clone() });
        }
        seen.push(id);
    }

    // 二、可用性：查条目（缺席即 `Err`）→ 只滤掉 `Unavailable`。两件事同一步做，
    //     故「不可用」与「未知」的差别就是「列表里有没有这一条」。
    let mut candidates: Vec<ExecutionCandidate> = Vec::with_capacity(models.len());
    for model in models {
        let id = model.profile().id();
        let Some((_, health)) = request.availability.iter().find(|(known, _)| known == id) else {
            return Err(RoutingError::UnknownAvailability { id: id.clone() });
        };
        if *health == ProviderHealth::Unavailable {
            continue;
        }

        let score = policy.evaluate(request, model);
        candidates.push(ExecutionCandidate {
            model: id.clone(),
            state: model.state(),
            compatibility: score.compatibility,
            confidence: score.confidence,
            reason: score.reason,
        });
    }

    // 三、空判定：无候选时不返回空列表（见函数文档）。
    if candidates.is_empty() {
        return Err(RoutingError::NoEligibleCandidate);
    }

    // 排序是本层唯一改变次序的地方，且它必须是全序（`compare` 的文档里逐条落实了）。
    candidates.sort_by(|a, b| policy.compare(a, b));

    Ok(RankedExecutionCandidates { candidates })
}

// ===== Task 12：具名基线策略（设计 §5.3 的「第二步」） =====

/// §19 的家族偏好 → 该偏好所指 provider 字面量的映射。**这是基线的选择，不是规范的定义。**
///
/// 设计 §5.3 只写死「同族优先、`Auto` 时全部视为 `SameFamily`」，**没有**说怎么从一个模型身上
/// 看出它属于哪一族；§247 的十二个字段里唯一可能承载这件事的是 `provider`，而它的词表
/// **规范未定义**（`crate::profile::ModelProfile::provider` 的文档：自由文本）。故基线取一条明确的规则：
/// **逐字相等即同族**，字面量按 §19 的短语取小写（`OpenAI preferred` → `openai`，
/// 与 [`FamilyPreference::as_str`] 取 `openai_preferred` 是同一条「按短语切词」的口径）。
/// `Auto` 与 `Custom` 返回 `None`（**不产生区别**）：前者 §19 明说「全部视为 `SameFamily`」，
/// 后者不带载荷、无从比对。
///
/// **这一处是命名基线的一次选择**：换一份策略可以换一套映射，规范一个字都没说
/// （设计 §11 第 10 条，收件人是规范维护者）。故本函数**不是** `FamilyPreference` 的方法——
/// 它不构成对该类型的一次规范声明。
fn preferred_provider(family: &FamilyPreference) -> Option<&'static str> {
    match family {
        FamilyPreference::Auto | FamilyPreference::Custom => None,
        FamilyPreference::OpenAiPreferred => Some("openai"),
        FamilyPreference::ClaudePreferred => Some("claude"),
        FamilyPreference::LocalPreferred => Some("local"),
    }
}

/// 一个候选与请求偏好的 §19 族关系。**判据全在 [`preferred_provider`] 里**（含它为什么是基线的选择）。
///
/// [`preferred_provider`] 把 `Auto` 与 `Custom` 合到同一个 `None` 上，本函数于是也只有一条 `None` 臂：
/// **偏好不指向任何 provider 时不产生跨族差别**。这一条对 `Auto` 是 §19 的正文，
/// 对 `Custom` 是「无载荷可比」的后果——两者在代码里是同一件事，故不写成两臂。
fn family_relation(family: &FamilyPreference, provider: &str) -> FamilyRelation {
    match preferred_provider(family) {
        Some(preferred) if provider == preferred => FamilyRelation::SameFamily,
        Some(_) => FamilyRelation::CrossFamily,
        None => FamilyRelation::SameFamily,
    }
}

/// **具名基线策略**（设计 §5.3 的「第二步」）。名字里的「基线」是字面意思：
/// 它是本层交付的一个**可替换**实现，**不是对规范的声称**——§84 未给打分函数，
/// 故谁都可以换一份 [`RankingPolicy`] 重跑同一组用例（设计 §5.1 的请求／策略分界）。
///
/// # 它只读**已定义**的输入
///
/// §248 维度的**有无**（不读分数）、§247 的 `confidence`、§19 的家族偏好、§249 的状态：
///
/// - `compatibility` = 需求维度中**有观测**的那几个的占比（`matched / required`）。
///   **只判「有没有观测」，`SkillScore` 的数值一概不读**——这正是 §2.4「只用序、不用量」的落点：
///   占比是一个**有界的集合运算结果**，不是任何跨维度求和。
/// - `confidence` = 画像上的 `confidence`（§247），原样带出。
/// - `reason` 记 `matched` / `missing` / family（[`FamilyRelation`] 字段）与 state（写进 `notes`）。
///
/// # `compare` 用缺省实现，**本类型不重写**
///
/// 这是刻意的：四档（compatibility → confidence → family → `ModelId`）是设计 §5.3 写死的全序，
/// 基线不引入第二份排序口径。**故删掉缺省实现里的 `family` 那一档会红在本层的用例上**
/// （`tests/router.rs` 的 `same_family_candidates_rank_before_cross_family_ones`）——
/// 那是「基线确实经缺省的 `compare` 定序」这件事的照片。
///
/// # 被否掉的两个候选打分法（写进文档，不是可选说明）
///
/// **(a) 分数加权求和**——需要一个规范没有的**尺度与方向**：`SkillScore` 的取值域与单位未定义
/// （§23 的示例是 9.2，§24 未给范围），加权求和要先知道每一维怎么归一、往哪边算「更好」。
/// **(b) 阈值匹配**——需要一个规范没有的**阈值**：「命中几个算够」§250 没有给。
/// 两者都撞在 §2.4 的同一条判据上：**量纲未知时只做集合运算，不做算术**。
///
/// # 家族关系的判据
///
/// [`preferred_provider`] 与 [`family_relation`]：同族 = 模型的 `provider` 逐字等于该偏好所指的
/// 字面量；`Auto` / `Custom` 不产生跨族差别。**这是一次基线的选择**，理由与出处写在
/// [`preferred_provider`] 上。
///
/// # 它不读预算
///
/// [`RoutingRequest::budget`] 是**必填**的（不是 `Option`），故忽略它是一次**看得见的选择**，
/// 不是一次遗漏——§333 的五个量纲没有单位，量值算不出来（设计 §5.3 末段、§10 第 2 条）。
/// 用例 `the_baseline_does_not_read_the_budget` 的注释里逐句写明**这是「已写明未实现」而不是「忘了读」**。
pub struct BaselineRankingPolicy;

impl RankingPolicy for BaselineRankingPolicy {
    /// 按 §5.3 的三条算：`compatibility` 是**有无**的占比、`confidence` 取画像、
    /// `reason` 记四样（family 走字段，state 走 `notes`）。
    ///
    /// **不读 `SkillScore` 的数值、不读 `budget`、不读 `ProviderHealth`**——后两者本层拿不到，
    /// 前者是 §2.4 的禁令。`missing` 是 `Vec<SkillDimension>`，**不折算成扣分**（那要权重）。
    fn evaluate(&self, request: &RoutingRequest, model: &RoutableModel) -> CandidateScore {
        let skill_vector = model.profile().skill_vector();

        // 逐维判「有没有观测」——**只看 `is_some()`，不读 `score()`**。两个列表各收各的，
        // 顺序原样保留（与 `RoutingReason::new` 的契约一致：不排序、不去重）。
        let mut matched: Vec<SkillDimension> = Vec::new();
        let mut missing: Vec<SkillDimension> = Vec::new();
        for dimension in request.requirements.dimensions() {
            if skill_vector.get(*dimension).is_some() {
                matched.push(*dimension);
            } else {
                missing.push(*dimension);
            }
        }

        // `required` 非空由 `TaskSkillRequirement::try_new` 在**构造期**保证，故此处不可能 `0/0`；
        // `matched` 是 `required` 的子集，故商必落在 `[0,1]`——`Ratio` 的两条守卫都不会被踩到。
        // 这个 `expect` 因此不是「随手写的一句」：它是上面那两条保证的收口。
        let required = request.requirements.dimensions().len();
        let compatibility = Ratio::try_new(matched.len() as f64 / required as f64)
            .expect("matched / required ∈ [0,1]：matched ⊆ required，且 required 由构造期保证非空");

        CandidateScore {
            compatibility,
            confidence: model.profile().confidence(),
            reason: RoutingReason::new(
                family_relation(&request.family, model.profile().provider()),
                matched,
                missing,
                vec![state_note(model.state())],
            ),
        }
    }
}

/// 策略写进 [`RoutingReason::notes`] 的那一条状态陈述（`reason` 记 state 的落点，设计 §5.3）。
///
/// **`notes` 是自由文本，不是落库编码**：本仓「不用 `Debug` 表示落库」那条纪律管的是列值，
/// 而这一句不进任何列（`RoutingReason` 没有落库编码，设计 `:586`）。故这里用 `{:?}` 打出变体名，
/// 与 `RoutableState` **不另建一张编码表**——`RoutableState` 是内存里的收窄类型，
/// 设计明写它「不落库，故不需要编码」。
fn state_note(state: RoutableState) -> String {
    format!("state: {state:?}")
}
