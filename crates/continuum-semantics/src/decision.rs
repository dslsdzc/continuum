//! §331 的 Decision 对象与 §332／§107 的恢复链的**值**（设计 §10.1／§10.2）。
//!
//! 计划「文件结构」一节把 `Decision、DecisionOption、DecisionResult` 归在
//! `src/decision.rs`（`docs/superpowers/plans/2026-10-06-p4-semantic-layer.md:311`）；
//! 本文件与 `persist.rs` 的分工照 Task 8／10／11／13 的惯例：**本文件是 §331／§332
//! 的词汇与记录体**，`persist.rs` 是 `decision` / `decision_result` 两张表的迁移 110
//! 段与 I/O（[`crate::create_decision`] / [`crate::load_decision`] /
//! [`crate::save_decision_result`] / [`crate::apply_result`]）。
//!
//! # §331 的四个字段 → 本结构体的五个成员（**两处申报**）
//!
//! §331（`docs/spec/05-normative.md:2471-2482`）：`Decision { issue, options[],
//! consequences[], blocking }`，「而不是只发一句自由文本问题」。
//!
//! - `options[]` / `consequences[]` 在类型上是**一个** `Vec<DecisionOption>`——
//!   每一项是「选项 ＋ 后果」的配对（§106 的例子：每个选项带后果）。`consequences`
//!   作为派生视图（[`Decision::consequences`]），**等长因此由构造期保证**（见
//!   [`DecisionOption`]），不需要检查。**这是本设计的落点**：§331 列两个平行列表，
//!   本层把它读成一组配对——平行两列表在类型上可不等长，而 §106 的「每个选项带后果」
//!   要求的是配对。
//! - **补两个字段**（照 [`crate::ContractRef`] 补 `version` 的同一判据）：
//!   [`Decision::id`]（落库主键与「`DecisionResult` 指向某个 `Decision`」需要；
//!   §331 没给，理由写在 [`DecisionId`]）与 [`Decision::intent_id`]（§10.2 的 Resume
//!   要「Intent 从 `SUSPENDED` 回 `RUNNING`」——决策单挂在哪一个 Intent 上是恢复链
//!   的必需输入，§331 也没给）。**读的人不该以为 §331 里有这两个名字。**
//!
//! # `blocking` 的处置（重申设计 §10.1）
//!
//! §331 只给一个布尔字段，**取值域未给**——`false` 的语义「不阻塞」在规范里没有
//! 用武之地（设计 §16 第 16 条）。本层**保留该字段**（落库、读回、结构体成员都在），
//! 并只在 §10.1 的两个触发点写 `true`（[`crate::create_decision`] 照收调用方给出的
//! 事实——**本层不从预算或任何判定算它**），**不发明 `false` 的触发条件**。
//! 照片：`tests/decision.rs` 的
//! `a_decision_is_created_with_blocking_true_at_the_two_trigger_points`（两段），
//! 「为什么没有 `false` 的照片」写在那个用例里。
//!
//! # 命名冲突一处（设计 §10.1、§16 第 31 条）
//!
//! `continuum-policy` 已有 `Decision`（`Allow / Deny / RequireApproval`）——**本层的
//! §331 对象与它不是同一个概念**，但同名。本设计的处置：本层用 `Decision`，
//! 「在同时引用两者的模块里，策略的那个一律以 `PolicyDecision` 别名导入」。
//! **这是命名约定、不是类型守卫**——本 crate 对此的可拍形态是「根本不引用 policy」
//! （依赖边为零、`src/` 零命中），照片在 `tests/decision.rs` 的
//! `the_public_face_has_exactly_one_type_named_decision`（限度写在那里）。
//!
//! # §332／§107：`DecisionResult` 的形状是**本实现**定的（申报）
//!
//! 设计 §10.2 对 `DecisionResult` 只给了两条：它是「**结构化的、指向某个 `Decision`
//! 与某个 `DecisionOption` 的值**」，「`apply_result` 的输入只有它（不是自由文本、
//! 不是重跑 Intent Compiler）」；照片 (a) 要求「一个 `DecisionResult` 只改被它
//! **指名**的 Requirement / Plan 字段」。**字段单没给**，故 [`DecisionResult`] 与
//! 三个 update 类型（[`ContractUpdate`] / [`RequirementUpdate`] / [`PlanUpdate`]）
//! 的形状是本实现的落点，逐条申报写在各自类型上。

use continuum_canonical::Opaque;

use crate::contract::{RequirementClass, VerificationRequirement};
use crate::ids::{ContractId, DecisionId, IntentId, PlanId, RequirementId};
use crate::plan::PlanVersion;

/// §331 的 `Decision { issue, options[], consequences[], blocking }`——
/// 「而不是只发一句自由文本问题」。
///
/// # 字段清单（五个成员，其中两个是补的——见模块文档）
///
/// | §331 字段 | 本结构体 | 说明 |
/// |---|---|---|
/// | `issue` | [`Decision::issue`] | 不透明串：§331 只给字段名，本层不解释它 |
/// | `options[]` ＋ `consequences[]` | [`Decision::options`]（`Vec<DecisionOption>`）＋ [`Decision::consequences`]（派生视图） | 一组「选项 ＋ 后果」的配对；等长由构造期保证 |
/// | `blocking` | [`Decision::blocking`] | **调用方给出的事实**；本层不发明 `false` 的触发条件（见模块文档） |
/// | ——（补） | [`Decision::id`] | 落库主键与「指向」的键（见 [`DecisionId`]） |
/// | ——（补） | [`Decision::intent_id`] | 决策单挂的 Intent（§10.2 的 Resume 必需，见模块文档） |
///
/// # 字段是 `pub` 的（同 [`crate::Intent`] / [`crate::Plan`]）
///
/// 本类型是 §331 的记录，本层没有比 §331 更强的约束可挂（构造期的那个不变量挂在
/// [`DecisionOption`] 上）。`options` 的每个元素都是「选项 ＋ 后果」——
/// 这是本类型唯一的记录级不变量，由元素类型保证。
#[derive(Debug, Clone)]
pub struct Decision {
    /// 本层补的**记录键**（§331 没有；理由与限度见 [`DecisionId`]）。落库主键。
    pub id: DecisionId,
    /// 本层补的字段（§331 没有）：决策单挂在哪一个 Intent 上——
    /// §10.2 的 Resume（Intent 从 `SUSPENDED` 回 `RUNNING`）用它。
    pub intent_id: IntentId,
    /// §331 的 `issue`。**不透明串**：§331 只给字段名，本层不解释、不裁剪。
    pub issue: Opaque,
    /// §331 的 `options[]`：每一项是「选项 ＋ 后果」的配对（见 [`DecisionOption`]）。
    pub options: Vec<DecisionOption>,
    /// §331 的 `blocking`。**调用方给出的事实**——本层不从任何判定算它，
    /// 也不发明 `false` 的触发条件（模块文档；设计 §16 第 16 条）。
    pub blocking: bool,
}

impl Decision {
    /// §331 的 `consequences[]`：由 [`Decision::options`] 派生的**配对视图**——第 k 项是
    /// 第 k 个选项的后果。
    ///
    /// **等长不是本函数检查出来的**：它与 `options` 是同一组配对的两种读法，
    /// 「每个选项带后果」的强制在 [`DecisionOption::new`] 的构造期（§106）。
    /// 照片：`tests/decision.rs` 的
    /// `options_and_consequences_are_equal_length_for_every_decision`（读回侧，枚举三档）。
    pub fn consequences(&self) -> Vec<&Opaque> {
        self.options.iter().map(|o| &o.consequence).collect()
    }
}

/// §331 的 `options[]` 的**元素**：一个选项与它的后果的配对（§106 的例子：每个选项
/// 带后果）。
///
/// # 「等长」由构造期保证
///
/// [`DecisionOption::new`] 的**两个参数都是必填的** `Opaque`——不存在「只带选项」的
/// 记录形态，故 [`Decision::options`] 与 [`Decision::consequences`] 的等长不需要检查。
/// 照片：`tests/type_level.rs` 驱动的
/// `tests/compile_fail/a_decision_option_cannot_be_built_without_a_consequence.rs`
/// （两个面：`new` 少一个参数、结构体字面量少 `consequence` 字段，都编不过）＋
/// 它的正控制 `tests/compile_pass/decision_option_new_takes_option_and_consequence.rs`。
///
/// # 字段是 `pub` 的（同 [`Decision`]）
///
/// 锁起来不加任何不变量（两个字段本来就是全给才编得过），只让调用方多写一个构造函数。
/// 正控制样例顺带证明两个字段读得出来。
#[derive(Debug, Clone)]
pub struct DecisionOption {
    /// 选项本身（不透明：§331／§106 只给名字，本层不解释选项的格式）。
    pub option: Opaque,
    /// 这个选项的后果（不透明，同上）。**必填**——见类型文档。
    pub consequence: Opaque,
}

impl DecisionOption {
    /// 把「选项 ＋ 后果」配成一对。**不接受缺后果的选项**（两个参数都必填）——
    /// 「等长由类型保证而不是由检查保证」的落点。
    pub fn new(option: Opaque, consequence: Opaque) -> Self {
        DecisionOption {
            option,
            consequence,
        }
    }
}

/// §107／§332 的 `DecisionResult`：**一次用户选择的、结构化的结果**——
/// 「不得将其视为普通会话消息并重新解释整个任务」。
///
/// # 形状（本实现的落点，设计只给了语义——见模块文档）
///
/// - [`DecisionResult::decision_id`] ＋ [`DecisionResult::option_index`]：**指向**
///   某个 `Decision` 与某个 `DecisionOption`（下标——「指向」需要能定位到那一项；
///   用下标而不是值的理由：值没有 `PartialEq`，而下标是稳定、可校验的定位）。
/// - [`DecisionResult::contract_update`] / [`DecisionResult::plan_update`]：对
///   Contract / Plan 的**逐字段 patch**（`None` 字段 = 不动）——照片 (a) 的
///   「只改被它指名的字段、其余逐项不变」由「从旧版本拷贝再套 patch」的机制保证
///   （[`crate::apply_result`]）；`None` 那两个 update 本身 = 本次决定不动那一侧。
/// - [`DecisionResult::decided_at`]：用户做出决定的时刻（`i64` 毫秒）。
///   **由调用方给**——本层不取时钟（同 [`crate::Intent::created_at`] 的纪律）。
///
/// # `apply_result` 的输入只有它
///
/// 不是自由文本、不是重新编译——照片 (a) 在
/// `tests/decision.rs::a_decision_result_changes_only_the_fields_it_names`，
/// 否定式照片 (b)（没有「序列化成文本再重新编译」的路径）在
/// `tests/decision.rs::there_is_no_path_that_recompiles_a_decision_result_as_text`。
///
/// 字段是 `pub` 的（同 [`Decision`]）。
#[derive(Debug, Clone)]
pub struct DecisionResult {
    /// 指向的那个 [`Decision`]（`decision` 表的主键）。
    pub decision_id: DecisionId,
    /// 被选中的那一项——`Decision.options` 里的下标（指向某个 [`DecisionOption`]）。
    /// **越界是错误**（[`crate::DecisionError::UnknownOption`]），不钳到某一项。
    pub option_index: usize,
    /// 对 Contract 的指名修改（`None` = 本次决定不动 Contract）。
    pub contract_update: Option<ContractUpdate>,
    /// 对 Plan 的指名修改（`None` = 本次决定不动 Plan）。
    pub plan_update: Option<PlanUpdate>,
    /// 用户做出选择的时刻（`i64` 毫秒）。**由调用方给**：本层不取时钟。
    pub decided_at: i64,
}

/// §332 的 Contract update（设计 §10.2 的「Contract/Plan update：本层产生新版本」）的
/// **指名载体**：对某一条 Contract 的一组 requirement 字段修改。
///
/// # 形状是本实现的落点（申报）
///
/// 设计只要求「产生新版本」与「只改被指名的 Requirement 字段」；本实现取：
/// - [`ContractUpdate::id`] ＋ [`ContractUpdate::base_version`]：基准版本（§226 的
///   新版本要以一个真实基准起算——基准不存在是 [`crate::DecisionError::UnknownContract`]，
///   不静默新建）；
/// - [`ContractUpdate::requirements`]：逐条 [`RequirementUpdate`]；指名一个**不在**
///   基准里的 requirement id 是 [`crate::DecisionError::UnknownRequirement`]
///   （同 Constraint Validator 悬空键的 fail-closed 处置：判不了走 `Err`，不静默添加）。
///
/// # 新版本号由本层推进：`base_version + 1`
///
/// 与 [`crate::save_contract`] 的「版本由调用方给、不存在自动 +1」**不冲突**：
/// 那是「单条写入函数不替调用方创造版本」的纪律；这里是 §10.2 明确要的
/// 「本层**产生**新版本」这个动作——推进规则（+1）由恢复链定，撞上已存在的版本号
/// 由复合主键拒绝（fail-closed）。
#[derive(Debug, Clone)]
pub struct ContractUpdate {
    /// 被更新的 Contract（`id` 不变，新版本是 `base_version + 1`）。
    pub id: ContractId,
    /// 基准版本（被 patch 的来源版本）。
    pub base_version: u32,
    /// 逐条指名要改的 requirement 字段。
    pub requirements: Vec<RequirementUpdate>,
}

/// 对**一个** Requirement 的指名修改：`None` 的字段 = 不动，`Some` 的 = 改成它。
///
/// # 为什么字段不是「整个 Requirement」
///
/// 照片 (a) 要的是「只改被它**指名**的字段、其余字段**逐项不变**」——若拆成
/// 「删掉旧 requirement、加一个新 requirement」，未指名字段就由调用方重新给，
/// 「其余不变」不再是机制的保证。故这里是 patch：`id` 定位（不可改——
/// 改 id 是另一条 requirement），其余四个内容字段逐项可选。
///
/// **红条件**：把 `apply_result` 实现成「重新跑一遍 Intent Compiler」（未指名的
/// 字段被重新生成）——`tests/decision.rs` 的那条用例红。
#[derive(Debug, Clone)]
pub struct RequirementUpdate {
    /// 改哪一个 requirement（必须在基准 Contract 里；找不到是
    /// [`crate::DecisionError::UnknownRequirement`]）。
    pub id: RequirementId,
    /// `class` 的新值（`None` = 不动）。
    pub class: Option<RequirementClass>,
    /// `expression` 的新值（`None` = 不动）。
    pub expression: Option<Opaque>,
    /// `source` 的新值（`None` = 不动）。
    pub source: Option<Opaque>,
    /// `verification_requirement` 的新值（`None` = 不动）。
    pub verification_requirement: Option<VerificationRequirement>,
}

/// §332 的 Plan update 的**指名载体**：对某一条 Plan 的一组列表字段修改。
///
/// # 形状是本实现的落点（申报）
///
/// - [`PlanUpdate::id`] ＋ [`PlanUpdate::base_version`]：基准版本（不存在是
///   [`crate::DecisionError::UnknownPlan`]，不静默新建）；
/// - 五个列表字段逐项可选（`None` = 不动）——§228 的其余字段（`id` / `version` /
///   `intent_id` / `type` / `review_state`）**不开放 patch**：`type` 的改写没有语义，
///   `review_state` 的推进有 §9.2 的状态机（不是逐字段赋值），版本由下面那条规则推进。
///
/// # 新版本的推进：`base_version.next_material()` ＋ `DRAFT` ＋ 旧版本转 `SUPERSEDED`
///
/// 「用户的决定导致 Plan 更新」是与 §9.2 的**修订**同一档的动作（内容变更 ⇒ 重开
/// 审查门）——故照 [`crate::revise`] 的值层先例落库（Task 13 预告的「落库版修订」
/// 即此）：新版本 `major` +1、`minor` 归零、`review_state = DRAFT`；旧版本行转
/// `SUPERSEDED`（其余列不动）。撞上已存在的新版本号由复合主键拒绝（fail-closed）。
#[derive(Debug, Clone)]
pub struct PlanUpdate {
    /// 被更新的 Plan（`id` 不变，新版本是 `base_version.next_material()`）。
    pub id: PlanId,
    /// 基准版本（被 patch 的来源版本）。
    pub base_version: PlanVersion,
    /// `objectives` 的新值（`None` = 不动；`Some` 是**整列替换**，不是拼接）。
    pub objectives: Option<Vec<Opaque>>,
    /// `milestones` 的新值（同上）。
    pub milestones: Option<Vec<Opaque>>,
    /// `assumptions` 的新值（同上）。
    pub assumptions: Option<Vec<Opaque>>,
    /// `risks` 的新值（同上）。
    pub risks: Option<Vec<Opaque>>,
    /// `acceptance_conditions` 的新值（同上）。
    pub acceptance_conditions: Option<Vec<Opaque>>,
}
