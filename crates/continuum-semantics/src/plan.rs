//! §228 的 Plan 数据模型、§229／§338 的审查状态机与 §339／§340 的审查记录
//! （设计 §9.1／§9.2／§9.3）。
//!
//! 计划「文件结构」一节把 `Plan、PlanType、PlanVersion、ReviewState、ReviewFinding、
//! Severity、ReviewIndependence` 归在 `src/plan.rs`
//! （`docs/superpowers/plans/2026-10-06-p4-semantic-layer.md:308`）；
//! §341 的八项视图在 [`crate::review_view`]，本文件不含它。本文件与 `persist.rs` 的
//! 分工照 Task 8／10／11 的惯例：**本文件是 §228 的词汇与记录体**，
//! `persist.rs` 是 `plan` 表（迁移 110 的第五张表）。**本 task 的 `plan` 表没有行级
//! 读写**——表由本 task 建、列清单由 `tests/plan.rs` 钉；行级 I/O（`save_plan` 一类）
//! 随第一个有 I/O 用例的 task（§10.2 的恢复链要写 `plan` 行）落地，本文件因此
//! **不含 I/O**。
//!
//! # §228 的 `version` 取两段（本设计的决定，§9.1）
//!
//! §228（`docs/spec/05-normative.md:334-360`）只给 `version` 一个字段名，没给结构。
//! 本层取 `(major: u32, minor: u32)`，依据是 §230 的两类变更
//! （MINOR：换模型、重试、并行调整、工具替换、内部步骤重排；MATERIAL：重大架构变化、
//! Requirement 改变、权限扩大、成本显著扩大、Milestone 目标变化）与 §213
//! （《总纲》§2.6）的两档（`v1.1` 可自动、`v2` 需重审）：`major` 在 MATERIAL 时 +1、
//! `minor` 在 MINOR 时 +1。**收件人：复审者**（两段式是本设计的决定，不是规范转录）。
//!
//! **「major +1 时 minor 归零」这一半是本层从 §213 的例子读出来的**：那里的链是
//! `Plan v1 → v1.1 → v1.2 →（重大变化）→ Plan v2`——若 `minor` 不归零，从 `v1.x`
//! 出发的重大版本应写作 `v2.x`，而原文写的是 `v2`。§228／§9.1 只写了「+1」。
//!
//! # 审查状态机（§229 §338）：表是本设计的决定
//!
//! §229 给了六态（`DRAFT` / `SELF_REVIEWED` / `AI_REVIEWED` / `USER_APPROVED` /
//! `REJECTED` / `SUPERSEDED`）与顺序，**没给迁移关系**；[`ReviewState::transition`]
//! 的表逐条照设计 §9.2 录收（九对，含两条按设计批注的条目——见那个函数的文档）。
//!
//! **「修订」不是第六个态**（§9.2）：§338 的阶梯 `草稿 → 自审 → 独立强 AI 审查 →
//! 修订 → 用户审查 → 批准` 里的「修订」是**一次新版本**（§213 的版本化），
//! 落成 [`revise`]——产生新版本（`DRAFT`）＋旧版本转 `SUPERSEDED`。
//!
//! # §340 的落点：使事实可观察，不判「够不够」
//!
//! §340 的三条（独立上下文、优先更强模型、必要时跨 family）是 **SHOULD**，
//! **「分离到什么程度算够」规范没给判据**（设计 §16 第 14 条）。故本层
//! **不禁止**「同一模型 + 同一上下文」的审查（那会把 SHOULD 抬成 MUST = 发明判据），
//! 只把三条事实作为 [`ReviewIndependence`] **必填**记下来；那一条判定**没有照片**。
//!
//! # 本层明确不建（免得被读成漏项）
//!
//! - **本组件不是 UI**（§2.1 的组件表里没有用户界面组件）：§341 的八项视图
//!   （[`crate::UserReviewView`]）是本层产出的投影，呈现方不在本层。
//! - **`REJECTED` / `SUPERSEDED` 的「不可执行」判定**：§229 只给了
//!   `USER_APPROVED` 这一条 MUST（见 [`ReviewState::reaches_ready`] 的限度申报），
//!   更严的执行门**没有规范来源**，不为它发明判据。
//! - **§230 的 Plan Change 分类**（含 `classify` 与 `apply_plan_change`）归 Task 14
//!   （`src/plan_change.rs`），本文件不含它。

use continuum_canonical::Opaque;

use crate::error::PlanError;
use crate::ids::{IntentId, PlanId};

/// §228 的 `version` 取**两段**（本设计的决定，§9.1）：`major` 在 MATERIAL 时 +1、
/// `minor` 在 MINOR 时 +1。
///
/// 两段式与「归零」两半的来历见模块文档。**`PartialEq` 是语义能力不是装饰**：
/// 「两个版本相等」的判据就是两个数分别相等（两个 `u32` 的相等），本类型因此
/// 与 [`crate::IntentId`] 那一类不透明串不同——那一类**没有**给 `PartialEq`
/// （见 `ids.rs` 的模块文档），这里的相等有定义。
///
/// # 两个推进方法（供 §213 的两档与 §230 的两类使用）
///
/// [`PlanVersion::next_material`] 与 [`PlanVersion::next_minor`] 是**纯函数**
/// （取 `self`，返回新值，不改旧的）：版本推进由调用方决定何时发生
/// （写路径的记录在后续 task；Task 14 的 `apply_plan_change` 也用它们）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlanVersion {
    /// `vN` 里的 N。
    pub major: u32,
    /// `vN.M` 里的 M。
    pub minor: u32,
}

impl PlanVersion {
    /// MATERIAL 变更的推进：`major` +1、`minor` 归零（§9.1 ＋ §213 的读法，见模块文档）。
    pub fn next_material(self) -> PlanVersion {
        PlanVersion {
            major: self.major + 1,
            minor: 0,
        }
    }

    /// MINOR 变更的推进：`minor` +1、`major` 不动（§9.1）。
    pub fn next_minor(self) -> PlanVersion {
        PlanVersion {
            major: self.major,
            minor: self.minor + 1,
        }
    }
}

/// §228 的 `type`，**封闭两值**。
///
/// §228（`docs/spec/05-normative.md:354-359`）给两个词：`STRATEGIC` / `EXECUTION`。
/// 两值各有一条照片：`EXECUTION` 在
/// `tests/plan.rs` 的 `an_execution_plan_reaches_ready_without_user_approval`
/// （执行门不拦它）、`STRATEGIC` 在 `a_strategic_plan_cannot_be_executed_without_user_approval`
/// （执行门拦它）——本 task 的两条用例恰好各覆盖一枚。
///
/// # 编码（小写、多词接 `_`）与 `as_str` / `parse` 同址
///
/// [`PlanType::as_str`] **无通配臂**：加第三枚即**编译不过**；解码走带通配臂的
/// [`PlanType::parse`]（输入是开放集）。`plan` 表的 `type` 列用它落库
/// （表已建，行级读写随其后用它的 task；表外取值读回具体 `Err` 的处置也在那时落地）。
///
/// # 为什么这一对是 `pub`（加宽申报，与 `RequirementClass` 的 `pub(crate)` 不同）
///
/// 两条理由：
///
/// 1. **照片的可核位置**：本 task 的 `plan` 表**没有行级读写**（行级 I/O 随其后用它的
///    task 落地），故「编码字面量逐枚」「`parse` 的定义域」这两件事唯一能落在
///    **集成测试**（`tests/plan.rs` 的
///    `enum_codes_use_lowercase_underscore_and_reject_out_of_table_values`）里的方式
///    就是这一对可见；收在 `pub(crate)` 会让照片只住在 `src` 的 `#[cfg(test)]` 里
///    （可核性更差）。
/// 2. **0 warning 是硬约束**：`pub(crate)` 且非测试构建下无消费者会触发
///    `dead_code`——替代是 `#[allow(dead_code)]` 豁免，那会**掩盖**「这一对没人用」
///    这件事（本仓对豁免的既有处置见 `continuum-model-registry` 的 Task 7 复盘：
///    豁免要带复核义务）。加宽到 `pub` 比一个要长期背着复核义务的豁免更干净。
///
/// **收窄回 `pub(crate)` 的时点是行级 I/O 落地之后**（那时照片可经落库 raw 读列，
/// 同 [`crate::RequirementClass`] 的现状）——这是一处有复核条件的加宽，不是永久决定。
impl PlanType {
    /// 两枚之一 → 落库字面量。**全函数**（闭集两枚）。**无通配臂**。
    pub fn as_str(&self) -> &'static str {
        match self {
            PlanType::Strategic => "strategic",
            PlanType::Execution => "execution",
        }
    }

    /// [`PlanType::as_str`] 的逆。**定义域是所有串**，认不出就是 `None`
    /// （形状照 [`crate::IntentKind::parse`]：返回 `Option` 而不返回一枚「未知」变体）。
    /// **带通配臂是刻意的**（与 `as_str` 的无通配相反）：这一边的输入是开放集。
    pub fn parse(s: &str) -> Option<PlanType> {
        match s {
            "strategic" => Some(PlanType::Strategic),
            "execution" => Some(PlanType::Execution),
            _ => None,
        }
    }
}

/// §228 的 `type`（值与编码见 [`PlanType`]）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanType {
    /// §228 的 `STRATEGIC`。需要重大用户承诺时 MUST 经 `USER_APPROVED` 才可执行（§229）。
    Strategic,
    /// §228 的 `EXECUTION`。不进 `USER_APPROVED`（§341 把用户审查限定在战略层）。
    Execution,
}

/// §229 的六态。
///
/// §229（`docs/spec/05-normative.md:363-380`）给六个词与顺序
/// （`DRAFT → SELF_REVIEWED → AI_REVIEWED → USER_APPROVED`，另 `REJECTED` / `SUPERSEDED`），
/// **没给迁移关系**——迁移表是本设计的决定（§9.2），落在 [`ReviewState::transition`]。
///
/// # 编码与 `as_str` / `parse` 同址
///
/// 同 [`PlanType`]（含「为什么是 `pub`」的加宽申报）：`as_str` 无通配臂、
/// `parse` 带走通配臂；`plan` 表的 `review_state` 列用它落库。
impl ReviewState {
    /// 六枚之一 → 落库字面量。**全函数**（闭集六枚）。**无通配臂**。
    pub fn as_str(&self) -> &'static str {
        match self {
            ReviewState::Draft => "draft",
            ReviewState::SelfReviewed => "self_reviewed",
            ReviewState::AiReviewed => "ai_reviewed",
            ReviewState::UserApproved => "user_approved",
            ReviewState::Rejected => "rejected",
            ReviewState::Superseded => "superseded",
        }
    }

    /// [`ReviewState::as_str`] 的逆。**定义域是所有串**，认不出就是 `None`（理由同
    /// [`PlanType::parse`]）。
    pub fn parse(s: &str) -> Option<ReviewState> {
        match s {
            "draft" => Some(ReviewState::Draft),
            "self_reviewed" => Some(ReviewState::SelfReviewed),
            "ai_reviewed" => Some(ReviewState::AiReviewed),
            "user_approved" => Some(ReviewState::UserApproved),
            "rejected" => Some(ReviewState::Rejected),
            "superseded" => Some(ReviewState::Superseded),
            _ => None,
        }
    }
}

/// §229 的六态（编码见上）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewState {
    /// §229 的 `DRAFT`。
    Draft,
    /// §229 的 `SELF_REVIEWED`。
    SelfReviewed,
    /// §229 的 `AI_REVIEWED`。
    AiReviewed,
    /// §229 的 `USER_APPROVED`。
    UserApproved,
    /// §229 的 `REJECTED`（终态）。
    Rejected,
    /// §229 的 `SUPERSEDED`（终态：新版本产生时旧版本进这里）。
    Superseded,
}

impl ReviewState {
    /// §9.2 的迁移表：判定一次迁移是否合法。
    ///
    /// # 表（逐条，与 `tests/plan.rs` 的九条合法对同源）——**本设计的决定**（§9.2）
    ///
    /// | 从 | 到 |
    /// |---|---|
    /// | `Draft` | `SelfReviewed`、`Rejected` |
    /// | `SelfReviewed` | `AiReviewed`、`Draft`（自审发现内部不一致，§208 允许自动修正）、`Rejected` |
    /// | `AiReviewed` | `UserApproved`、`Draft`、`Rejected` |
    /// | `UserApproved` | `Superseded`（新版本产生时） |
    /// | `Rejected` / `Superseded` | 终态，没有出边 |
    ///
    /// **自环全部不在表里**（与 §223 的 intent 表不同——那里两条自环是 `Ok` 的）：
    /// 本表没有自环，六态自环一律 [`PlanError::IllegalReview`]。
    ///
    /// # 两对与「修订」的分界（照实申报）
    ///
    /// 表里两对带设计批注：`AI_REVIEWED → DRAFT`（批注「修订，见下」）与
    /// `USER_APPROVED → SUPERSEDED`（批注「新版本产生时」）。设计「见下」段说的
    /// 「修订」是**跨版本**动作（产生新版本 ＋ 旧版本进 `SUPERSEDED`，§9.2），
    /// 落在 [`revise`]；本函数是**一个版本内**的迁移判定，两对**照表录收**为合法对。
    /// **读法申报**：若复审判定表里那两对应从「同版本迁移」里移走（例如
    /// `AI_REVIEWED → DRAFT` 只是批注、不是边），改的是表本身——本层照录不误，
    /// 但请把「跨版本的那一半」与 [`revise`] 的分工一并裁。
    ///
    /// # 照片
    ///
    /// `tests/plan.rs` 的两条：`every_listed_review_pair_is_accepted`（九对**逐条**，
    /// 不抽代表）与 `unlisted_review_pairs_are_rejected`（终态出边、跳步、回退、
    /// 六态自环全数——各断言 `from` / `to` 原样带出）。
    pub fn transition(from: Self, to: Self) -> Result<(), PlanError> {
        use ReviewState::*;

        let listed = matches!(
            (from, to),
            (Draft, SelfReviewed)
                | (Draft, Rejected)
                | (SelfReviewed, AiReviewed)
                | (SelfReviewed, Draft)
                | (SelfReviewed, Rejected)
                | (AiReviewed, UserApproved)
                | (AiReviewed, Draft)
                | (AiReviewed, Rejected)
                | (UserApproved, Superseded)
        );
        if listed {
            Ok(())
        } else {
            Err(PlanError::IllegalReview { from, to })
        }
    }

    /// §229 的执行门：`USER_APPROVED` 才可执行——**只对 Strategic Plan**。
    ///
    /// §229 原文：「Strategic Plan MUST 在需要重大用户承诺时进入 `USER_APPROVED`
    /// 才可以执行」。**「在需要重大用户承诺时」这个条件规范没给判据**；设计把它读作
    /// 「`STRATEGIC` 一律要求 `USER_APPROVED`」（§9.2），本层照录。
    /// `EXECUTION` 的 Plan **不进 `USER_APPROVED`**（§341 把用户审查限定在战略层），
    /// 故这道门不拦它。
    ///
    /// # 名字与「去 READY」
    ///
    /// 「去 READY」的 READY 不是 [`ReviewState`] 的态（六态里没有它）——它是
    /// 「可以执行」这道门的名字（§9.2 的两条照片：「未经 `USER_APPROVED` 去 `READY`
    /// 得 `Err(UserApprovalRequired)`；`EXECUTION` 同路径得 `Ok`」）。
    ///
    /// # 限度照实写
    ///
    /// 本门只落实 §229 这一条 MUST。「终态（`Rejected` / `Superseded`）不可执行」
    /// 之类的更严判定**没有规范来源**——故 `EXECUTION` 在**任何**态下都 `Ok`，
    /// 本层不为更严的规则发明判据（要收紧的话，那是一处要带判据来的改）。
    ///
    /// # 照片
    ///
    /// `tests/plan.rs` 的两条（**两侧都钉**）：`a_strategic_plan_cannot_be_executed_without_user_approval`
    /// （未经时 `Err` ＋经时 `Ok`——反面也在）与
    /// `an_execution_plan_reaches_ready_without_user_approval`（同路径 `Ok`）。
    pub fn reaches_ready(&self, plan_type: PlanType) -> Result<(), PlanError> {
        match plan_type {
            PlanType::Strategic => match self {
                // 无通配臂：加第七态即编译不过（那时实现者必须显式决定它可不可执行）。
                ReviewState::UserApproved => Ok(()),
                ReviewState::Draft
                | ReviewState::SelfReviewed
                | ReviewState::AiReviewed
                | ReviewState::Rejected
                | ReviewState::Superseded => Err(PlanError::UserApprovalRequired {
                    plan_type,
                    state: *self,
                }),
            },
            PlanType::Execution => Ok(()),
        }
    }
}

/// §228 的 `Plan`，**十个字段逐字照 §228**。
///
/// # 字段与类型
///
/// | §228 字段 | 本结构体 | 说明 |
/// |---|---|---|
/// | `id` | [`PlanId`] | 落库主键的第一段（行级读写随后续 task） |
/// | `version` | [`PlanVersion`] | **两段式是本设计的决定**（§9.1，见模块文档） |
/// | `intent_id` | [`IntentId`] | 不建外键（同 `intent.contract_id` 的处置） |
/// | `type` | [`PlanType`] | Rust 关键字，用 raw identifier `r#type` 承载——**字段名没有**改成 `plan_type` |
/// | `objectives[]` / `milestones[]` / `assumptions[]` / `risks[]` / `acceptance_conditions[]` | `Vec<Opaque>` | §228 只给名单、未给元素结构（§7.1）：逐串不透明，本层不解释 |
/// | `review_state` | [`ReviewState`] | 合法迁移的判定在 [`ReviewState::transition`] |
///
/// # 五个列表为什么不各造 newtype（与 `spec.rs` 先例的分野）
///
/// `spec.rs` 的五个成员类型（`Goal` / `Invariant` / …）各包一个 `Opaque`，挡的是
/// 「把 preference 填进 goals」在类型上可写。本层**不跟随**那一处置：计划「文件结构」
/// 一节给 `src/plan.rs` 的类型清单里没有五个列表的成员类型
/// （`docs/superpowers/plans/2026-10-06-p4-semantic-layer.md:308`），而 §228 的五个
/// 列表字段名在构造点自明；代价是**列表位置互换在类型上可写**（与
/// [`crate::TaskContract`] 六个 `Opaque` 字段同一条处置）。将来若要收窄，逐列加
/// newtype 是一处小改。
///
/// # 字段是 `pub` 的（同 [`crate::Intent`] / [`crate::TaskContract`]）
///
/// 本类型是 §228 逐字给出的记录，本层没有比 §228 更强的约束可挂（版本推进挂在
/// 调用方，见 [`PlanVersion`]）。字段清单**逐项**的照片是 `tests/plan.rs` 的
/// `plan_carries_exactly_the_ten_fields_of_228`（穷尽解构、**不带 `..`**）。
#[derive(Debug, Clone)]
pub struct Plan {
    /// §228 的 `id`。
    pub id: PlanId,
    /// §228 的 `version`（两段式，见 [`PlanVersion`]）。
    pub version: PlanVersion,
    /// §228 的 `intent_id`。
    pub intent_id: IntentId,
    /// §228 的 `type`。字段名照 §228 是 `type`（Rust 关键字，raw identifier）。
    pub r#type: PlanType,
    /// §228 的 `objectives[]`。
    pub objectives: Vec<Opaque>,
    /// §228 的 `milestones[]`。
    pub milestones: Vec<Opaque>,
    /// §228 的 `assumptions[]`。
    pub assumptions: Vec<Opaque>,
    /// §228 的 `risks[]`。
    pub risks: Vec<Opaque>,
    /// §228 的 `acceptance_conditions[]`。
    pub acceptance_conditions: Vec<Opaque>,
    /// §228 的 `review_state`。
    pub review_state: ReviewState,
}

/// §9.2 的「修订」：**不是 §229 的第六个态**——**产生新版本 ＋ 旧版本转 `SUPERSEDED`**。
///
/// 输入被修订的版本，返回**两枚**（顺序固定）：
///
/// 1. **新版本**——同一个 `id`、`major` +1（`minor` 归零）、`review_state = DRAFT`；
///    内容与输入相同（**本函数不发明内容**——修订者拿到新版本后按需改 `pub` 字段）；
/// 2. **被取代的旧版本**——内容与版本号都不变，`review_state = SUPERSEDED`。
///
/// # 修订属 MATERIAL 档（推导链）
///
/// 修订后重新从 `DRAFT` 走 §338 的审查阶梯 ⇒ 重开审查门；而 §230 的 MATERIAL
/// **MUST** 重触发审查门、§213 把「重大变化」写作 `Plan v2`（`REVIEW_REQUIRED`）。
/// 故 `major` +1（而不是 `minor` +1）。
///
/// # 为什么是纯函数、不碰库
///
/// 本 task 的 `plan` 表没有行级读写（见模块文档）——修订的**两件事**在值层面
/// 断言（`tests/plan.rs` 的 `a_revision_is_a_new_version_and_the_old_one_becomes_superseded`，
/// 一条用例断言两件事都成立）；落库版的修订（INSERT 新版本 ＋ UPDATE 旧版本行）
/// 随行级 I/O 一起落地。**这不是漏项**：规范要的是「不是原地改状态」这个事实，
/// 本函数在类型上把它钉住——「原地改状态」的实现在那条用例里至少红一件。
///
/// # 红条件（写在用例里）
///
/// 把修订实现成原地改状态（**移除档**）：只把输入的状态改成别的、不产生
/// `major` +1 的新版本（第一件红）；或产出新版本但没把旧版本标 `SUPERSEDED`（第二件红）。
pub fn revise(plan: &Plan) -> (Plan, Plan) {
    let mut revised = plan.clone();
    revised.version = plan.version.next_material();
    revised.review_state = ReviewState::Draft;

    let mut superseded = plan.clone();
    superseded.review_state = ReviewState::Superseded;

    (revised, superseded)
}

/// §339 的 severity，**封闭四值**。
///
/// §339（`docs/spec/05-normative.md:2621-2643`）给四个词：
/// `BLOCKER` / `MAJOR` / `MINOR` / `SUGGESTION`。**四值各一条照片**（`tests/plan.rs`
/// 的 `every_review_finding_severity_has_a_photo`：各构造一枚、断言是**哪一枚**，
/// 判别式读出经穷尽 `match`、不带 `..`）；落库字面量（小写）与 `parse` 定义域的照片
/// 在同文件的 `enum_codes_use_lowercase_underscore_and_reject_out_of_table_values`
/// （编码对为 `pub` 的加宽申报同 [`PlanType`]）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// §339 的 `BLOCKER`。
    Blocker,
    /// §339 的 `MAJOR`。
    Major,
    /// §339 的 `MINOR`。
    Minor,
    /// §339 的 `SUGGESTION`。
    Suggestion,
}

impl Severity {
    /// 四枚之一 → 落库字面量。**全函数**（闭集四枚）。**无通配臂**。
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Blocker => "blocker",
            Severity::Major => "major",
            Severity::Minor => "minor",
            Severity::Suggestion => "suggestion",
        }
    }

    /// [`Severity::as_str`] 的逆。**定义域是所有串**，认不出就是 `None`（理由同
    /// [`PlanType::parse`]）。
    pub fn parse(s: &str) -> Option<Severity> {
        match s {
            "blocker" => Some(Severity::Blocker),
            "major" => Some(Severity::Major),
            "minor" => Some(Severity::Minor),
            "suggestion" => Some(Severity::Suggestion),
            _ => None,
        }
    }
}

/// §339 的 `ReviewFinding`：五个字段**逐字照 §339**（`severity` / `subject` / `problem` /
/// `evidence` / `recommendation`）。
///
/// 除 `severity` 外的四个描述字段取 [`Opaque`]（§339 只给名字、未给结构——
/// 同 [`crate::TaskContract`] 六个声明字段的处置；为什么不各造 newtype 见
/// [`Plan`] 的文档，理由相同）。字段是 `pub` 的（同 [`Plan`]）。
#[derive(Debug, Clone)]
pub struct ReviewFinding {
    /// §339 的 `severity`（四值封闭，见 [`Severity`]）。
    pub severity: Severity,
    /// §339 的 `subject`（不透明）。
    pub subject: Opaque,
    /// §339 的 `problem`（不透明）。
    pub problem: Opaque,
    /// §339 的 `evidence`（不透明）。
    pub evidence: Opaque,
    /// §339 的 `recommendation`（不透明）。
    pub recommendation: Opaque,
}

/// 对一个模型引用的引用（名字照设计 §9.3 与任务简报逐字给出的 `ModelRefRef`）：
/// **不透明 `String`**，处置同 [`crate::IntentId`] 一类的不透明串（不改写、不裁剪、无校验）。
///
/// # 为什么不是资源层的模型 id 类型（§1.2.3 规则 3）
///
/// 取 `continuum-model-registry`（或资源层任何地方）的模型 id 类型会写出一条
/// 指向资源层的 `Cargo.toml` 边——而本层**不登记**指向资源层的边。故这里取不透明串，
/// 代价是「本层这个串」与「资源层那个模型 id」之间的对应由装配方维持
/// （同 [`crate::GraphIdRef`] 那一处的代价）。
///
/// **「不改写、不裁剪」的照片**：`tests/plan.rs` 的
/// `a_same_model_same_context_review_is_observable_as_such`——两个 model ref 用
/// **带空白与全角字符**的串构造、逐字节读回（`from_str` 一裁剪即红）。
#[derive(Debug, Clone)]
pub struct ModelRefRef(String);

impl ModelRefRef {
    /// 取里子串。**不改写、不裁剪**。
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 从任意串构造。**无校验**——本类型没有「合法取值」这个概念
    /// （模型 id 的取值域属于资源层；本层只保存事实）。
    pub fn from_str(s: &str) -> Self {
        ModelRefRef(s.to_owned())
    }
}

/// §340 的三条独立性事实。**必填**——不是布尔标志位，是三条各带来源的记录（§9.3）。
///
/// 字段清单逐项：`separate_context`（独立上下文）/ `reviewer_model`（审查方模型）/
/// `author_model`（作者方模型）/ `cross_family`（是否跨 family）。
/// 照片：`tests/plan.rs` 的 `a_review_finding_record_carries_the_independence_facts`
/// （穷尽解构、**不带 `..`**）。
///
/// **本类型不禁止任何组合**：三条是 SHOULD（§340），「分离到什么程度算够」规范没给
/// （§16 第 14 条）——本层只使事实可观察。
#[derive(Debug, Clone)]
pub struct ReviewIndependence {
    /// 审查是否在独立上下文里进行（§340 的第一条）。
    pub separate_context: bool,
    /// 审查方模型（不透明，见 [`ModelRefRef`]）。
    pub reviewer_model: ModelRefRef,
    /// 作者方模型（不透明）。
    pub author_model: ModelRefRef,
    /// 审查是否跨模型 family（§340 的第三条）。
    pub cross_family: bool,
}

/// §339 的一次审查产出**记录**：一条 finding 与它的独立性事实（§340）。
///
/// `independence` **必填**——没有可省路径（构造时必给，解构时必列）；
/// 照片：`tests/plan.rs` 的 `a_review_finding_record_carries_the_independence_facts`
/// （穷尽解构、**不带 `..`**——少一个字段即编译不过）。
///
/// **本 task 不落库它**：§339／§340 没有给它的表与列（设计 §2.4 的七张表里没有
/// 审查记录表），落库形态随第一个需要它的消费者（如 §10.3 的审计载荷）落地。
/// 收件人：复审者（这也是「§340 的判定没有照片」的同一处不完备）。
#[derive(Debug, Clone)]
pub struct ReviewFindingRecord {
    /// 一条审查发现（§339）。
    pub finding: ReviewFinding,
    /// 它的独立性事实（§340）。**必带**。
    pub independence: ReviewIndependence,
}
