//! §230 的 Plan Change 分类（设计 §9.4）。
//!
//! 计划「文件结构」一节把 `PlanChange、PlanChangeClass、classify` 归在 `src/plan_change.rs`
//! （`docs/superpowers/plans/2026-10-06-p4-semantic-layer.md:309`）；`apply_plan_change`
//! 是任务简报 Interfaces 一栏的第四枚产出。
//!
//! # §230 的两个 class 与十个例
//!
//! §230（`docs/spec/05-normative.md:384-420`）：`PlanChange { class, affected_scope }`、
//! `class ∈ {MINOR, MATERIAL}`、**MATERIAL MUST 重新触发相应 Review Gate**。
//! 十个例（五个 MINOR ＋ 五个 MATERIAL）**逐项的判定映射**见下表——
//! **映射是本实现的读法**（收件人：复审者），依据与边界写在表的后面。
//!
//! | §230 的例 | 在输入面上的表现 | 判定 |
//! |---|---|---|
//! | 模型替换 / 重试 / 并行调整 / 工具替换 / 内部步骤重排 | 两 [`Plan`] 与两 [`PlanFootprint`] **逐字段相等**（执行 HOW 的变更不在 §228 的十个字段上） | [`PlanChangeClass::Minor`] |
//! | 重大架构变化 / Requirement 改变 / Milestone 目标变化 | 计划内容（§228 的五个列表）**有任一变化** | [`PlanChangeClass::Material`] |
//! | 权限扩大 | `authority` 经调用方判据判真 | [`PlanChangeClass::Material`] |
//! | 成本显著扩大 | `estimated_cost` 经调用方判据判真 | [`PlanChangeClass::Material`] |
//!
//! **MINOR 五个例为何在输入面上不可见**：§230 的 MINOR 五个例（换模型、重试、
//! 并行调整、工具替换、内部步骤重排）都是**执行 HOW** 的变更——§225 的 MAY 清单
//! （`换模型 / 换工具 / 换节点 / 换算法 / 重新规划 / 改变并行度 / 改变内部执行顺序`，
//! `docs/spec/05-normative.md:265-275`）与它们重叠，而 MAY 的含义是
//! 「Runtime 可以自动改变 HOW」。计划与它声明的影响面没有变，故这一侧判 MINOR
//! （可自动处理，§213 的「v1.1 可自动」一侧）。**这不是漏判**：这五例的变化在
//! `classify` 的输入面（计划 ＋ 它声明的影响面）上确实无处可写。
//!
//! **MATERIAL 三条「内容变化」例为何同判**：设计 §9.4（`design:1206-1207`）把
//! MATERIAL 的判定量写作「权限、成本、**架构**」。`Milestone 目标变化` 直接点名
//! `milestones`；`Requirement 改变` 与 `重大架构变化` 在 §228 的字段面上没有同名者
//! （Contract 的 Requirement **不在** `classify` 的输入面——那是 §9.4 定格的）。
//! 本层**不发明「哪个字段算重大」的分层**——取「计划内容（五个列表）有任一变化即
//! 重触发审查门」这一条统一规则。**代价照实写**：`assumptions` / `risks` 的变化
//! 规范没有例子，本规则把它们同判 MATERIAL——取的是更保守的一侧（宁可多审、不漏审），
//! 而「假设变化可以自动」「风险变化必须重审」这类细分**没有规范来源**，本层不为它发明。
//!
//! **「变化」的判定是逐位置比较**（长度 ＋ 逐项 [`Opaque::as_str`]）：`Opaque` 没有
//! `PartialEq`（见 `continuum-canonical` 的文档），而「两个值是不是同一个值」在
//! `as_str` 上可判——同 Task 12 的 `diff` 用 `as_str` 检测表达式变化那一处。
//! **列表重排**（同内容不同顺序）按逐位置比较算「变化」：规范未给「重排算不算变化」
//! 的判据，本层的处置照此申报。
//!
//! **两个 class 各自的第一条例**（`模型替换` 与 `重大架构变化`）各有**直接调用**
//! 用例（不经表）：`tests/plan_change.rs` 的 `a_model_swap_is_a_minor_change`
//! （输入是「两 Plan 与两 footprint 逐字段相等」）与 `a_major_architecture_change_is_material`
//! （输入是「objectives 变化」）——它们与表驱动的十例用例互为对照
//! （「表驱动用例也可能覆盖错路径」）。
//!
//! # 阈值入参的处置链（设计 §9.4 → §16 第 15 条 → 计划遗留第四节 → R-B 裁决）
//!
//! [`classify`] 的签名是**五参**，其中第五参 `&MaterialThresholds` 的来历是一条链：
//!
//! 1. **设计 §9.4**（`design:1207-1208`）定格分类函数收「两份计划 ＋ 两份声明的影响面」
//!    四参，并把「『显著扩大』的阈值规范未给」挂给 **§16 第 15 条**
//!    （`design:2363`，收件人：规范维护者）。
//! 2. **计划遗留第四节**（`plan:2546-2548`）取「**阈值以入参进入、本层不发明数值**」
//!    ——代价是多一个入参；收益是不发明判据。但计划 Step 1/3 的签名速写
//!    （`plan:1575-1600`）漏记了该参（**计划文本自相抵**）。
//! 3. **协调者裁决 R-B**（2026-10-14，`progress.md:77`）定：`classify` 为五参，
//!    `MaterialThresholds` 的形状由本 task 按**最小原则**自定——「『成本显著扩大』与
//!    『权限扩大』两个例的判定能力由调用方承载——**本层连比较都不发明**」。
//!
//! 故 [`MaterialThresholds`] 是两个**判定引用**（`Fn(&Opaque, &Opaque) -> bool`：
//! before / after 两个不透明值进、bool 出）：`authority_expanded` 与 `cost_expanded`。
//! 「比较」的实现（数值、序、差额百分比……）完全在调用方；本层只把两个字段原样递给判据。
//!
//! **与计划 Step 3 速写的偏离申报**：速写的代码块给的是四参签名；本实现按 R-B 落五参。
//!
//! # `PlanFootprint` 的另两个字段不参与
//!
//! `privacy_class` 与 `external_effects` **不进入本文件的判定**：带「隐私」与
//! 「外部副作用」名字的两条是 §225 的 MUST NOT（设计 §11.2 把它们判给
//! [`crate::ConstraintVerdict`] 那一路，Task 16 已落），不是 §230 的分类例子。
//! §9.4 的三量（权限、成本、架构）里也没有它们。
//!
//! # `apply_plan_change`：§230 的 MUST 落在值层面
//!
//! §230 的 MUST（MATERIAL 重触发审查门）落成一个**纯函数**（同 [`crate::revise`] 的
//! 先例）：MATERIAL 时产出新版本（`major` +1、`minor` 归零）且新版本从 `DRAFT` 起
//! （重走 §338 的阶梯）；MINOR 时不重触发（`minor` +1、审查态保持）。**两侧都钉**
//! （`tests/plan_change.rs` 的 `a_material_change_re_triggers_the_review_gate`）。
//!
//! **它不处置旧版本**（照实申报）：§230 只说 MUST 重触发审查门；「新版本产生时旧版本进
//! `SUPERSEDED`」是 §9.2 给**修订**的规则，落在 [`crate::revise`]——本层不把两件事
//! 混成一件。落库版的变更应用（写 `plan` 行）随行级 I/O 落地（本 task 的 `plan` 表
//! 没有行级读写——同 Task 13 的现状）。
//!
//! # 本文件不落库（故不建 `as_str` / `parse`）
//!
//! Task 13 的 `PlanType` / `ReviewState` 建编码对是因为 `plan` 表有对应列；
//! `PlanChange` **没有表**（设计 §2.4 的 110 段七张表：`intent` / `specification` /
//! `contract` / `contract_requirement` / `plan` / `decision` / `decision_result`，
//! 没有 `plan_change`），故不建编码对——有落库需求的那天，在类型同址加（照同址先例）。

use continuum_canonical::Opaque;

use crate::constraint::PlanFootprint;
use crate::plan::{Plan, ReviewState};

/// §230 的两个 class，**封闭两值**。MATERIAL MUST 重触发相应审查门——
/// 那条 MUST 的落点是 [`apply_plan_change`]。
///
/// 两枚各有照片：`tests/plan_change.rs` 的 `both_plan_change_classes_have_a_photo`
/// （判别式读出经穷尽 `match`、**不带 `..`**——加第三枚即编译不过）。
///
/// **本 task 不落库**（`PlanChange` 没有表），故不建 `as_str` / `parse` 编码对
/// （Task 13 的 `PlanType` / `ReviewState` 建那一对是因为 `plan` 表有对应列）——
/// 见模块文档。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanChangeClass {
    /// §230 的 `MINOR`——可自动处理的变更（§213 的 `v1.1` 一侧）。
    Minor,
    /// §230 的 `MATERIAL`——MUST 重触发审查门（§213 的 `v2` 一侧）。
    Material,
}

/// §230 的 `PlanChange`，**两个字段逐字照 §230**。
///
/// `affected_scope` 是 [`Opaque`]：§230 **未给取值域**（§16 第 15 条）——
/// 本层不发明，只承载（填什么由构造它的调用方决定）。字段清单逐项
/// （穷尽解构、**不带 `..`**）的照片：`tests/plan_change.rs` 的
/// `affected_scope_is_opaque`（表外取值逐字节往返）。
///
/// **今天没有填充 `affected_scope` 的生产点**（申报，收件人：复审者）：本 task 只有
/// [`classify`]（产出 class）与 [`apply_plan_change`]（消费 class）；`PlanChange`
/// 作为 §230 的记录体先落形状，填 `affected_scope` 的调用方随「谁产生变更声明」
/// 的第一步落地。
#[derive(Debug, Clone)]
pub struct PlanChange {
    /// §230 的 `class`。
    pub class: PlanChangeClass,
    /// §230 的 `affected_scope`（不透明：§230 未给取值域）。
    pub affected_scope: Opaque,
}

/// 「显著扩大」的**阈值入参**（R-B 裁决；处置链见模块文档）。
///
/// 两个字段是**判定引用**（`Fn(&Opaque, &Opaque) -> bool`）：before / after 两个
/// 不透明值进、bool 出——语义为「**这一对值，算不算显著扩大**」。**比较的语义完全在
/// 调用方**：本层不发明数值、不发明比较、也不解释 [`Opaque`]（「『显著扩大』的阈值
/// 规范未给」——§16 第 15 条）。
///
/// 形状（两个引用、这两个名字）是本 task 按协调者裁决 R-B 的**最小原则**自定的：
/// 「『成本显著扩大』与『权限扩大』两个例的判定能力由调用方承载」。
pub struct MaterialThresholds<'a> {
    /// 「权限扩大」的判定：收 `(before_fp.authority, after_fp.authority)`。
    pub authority_expanded: &'a dyn Fn(&Opaque, &Opaque) -> bool,
    /// 「成本显著扩大」的判定：收 `(before_fp.estimated_cost, after_fp.estimated_cost)`。
    pub cost_expanded: &'a dyn Fn(&Opaque, &Opaque) -> bool,
}

/// §230 的分类：给定变更前后两份计划与两份声明的影响面，判 `MINOR` / `MATERIAL`。
///
/// # 判定（三条判据取或：任一命中即 MATERIAL）
///
/// 1. **计划内容变化**：§228 的五个列表（`objectives` / `milestones` / `assumptions` /
///    `risks` / `acceptance_conditions`）有任一变化（逐位置比较，见模块文档）；
/// 2. **权限扩大**：调用方判据 `thresholds.authority_expanded` 判真；
/// 3. **成本显著扩大**：调用方判据 `thresholds.cost_expanded` 判真；
/// 4. 其余 → MINOR（计划与声明面均未变的变更——执行 HOW 的变更，见模块文档）。
///
/// 本函数**短路**（命中即返回），但**结果与判定次序无关**（三条都是「任一即材料」）；
/// 次序与短路的说明照此申报（设计未给次序判据）。
///
/// # 输入面与纯函数
///
/// 五参：两份 [`Plan`] ＋ 两份 [`PlanFootprint`]（§9.4 的四参）＋ 一个显式的阈值入参
/// （R-B 的第五参——处置链见模块文档）。**不收 `&Tx`、不读写库、不碰任何表**：
/// 跨层输入以值进入（§1.2.3 规则 3），同 [`crate::validate`] / [`crate::diff`] 的处置。
///
/// # 照片
///
/// `tests/plan_change.rs` 的七条用例：两个 class 各一条、§230 十例逐项一条（表驱动）、
/// 两条直接调用（两个 class 的第一例）、重触发审查门两侧、`affected_scope` 不透明、
/// 输入面签名（五参 ＋ 两个判据真的被调用）。逐条的红条件在那个文件的文件头。
pub fn classify(
    before: &Plan,
    after: &Plan,
    before_fp: &PlanFootprint,
    after_fp: &PlanFootprint,
    thresholds: &MaterialThresholds<'_>,
) -> PlanChangeClass {
    // (1) 计划内容变化（「架构」这一量——三个 MATERIAL 例的共同表现）。
    if plan_content_changed(before, after) {
        return PlanChangeClass::Material;
    }

    // (2) 权限扩大：比较由调用方承载（本层不发明「扩大」的判据）。
    if (thresholds.authority_expanded)(&before_fp.authority, &after_fp.authority) {
        return PlanChangeClass::Material;
    }

    // (3) 成本显著扩大：同上。
    if (thresholds.cost_expanded)(&before_fp.estimated_cost, &after_fp.estimated_cost) {
        return PlanChangeClass::Material;
    }

    // (4) 其余：计划与声明面均未变的变更（执行 HOW 的变更）——可自动处理。
    PlanChangeClass::Minor
}

/// §230 的 MUST 落成的纯函数：把一次已分类的变更应用到计划上，产出**新版本**。
///
/// - **MATERIAL**：`major` +1（`minor` 归零）、新版本的 `review_state = DRAFT`
///   ——**重触发相应审查门**（新版本重走 §338 的阶梯）；
/// - **MINOR**：`minor` +1（`major` 不动）、**审查态保持**（不重触发）。
///
/// 新版本与输入**同一个 `id`**、内容（五个列表等）**逐项原样**（本函数只推版本与
/// 审查态，不发明内容——修订者拿到新版本后按需改 `pub` 字段）。**不改动输入**
/// （纯函数：取 `&Plan`，返回新值）。
///
/// **不处置旧版本**：见模块文档（旧版本进 `SUPERSEDED` 是修订一侧的规则，
/// 落在 [`crate::revise`]）；**不落库**（本 task 的 `plan` 表没有行级读写）。
///
/// # 红条件（写在用例里）
///
/// 两侧都钉（`tests/plan_change.rs` 的 `a_material_change_re_triggers_the_review_gate`）：
/// MATERIAL 不重置审查门（**移除档**）即红；MINOR 也重置（**放宽档**）即红。
pub fn apply_plan_change(plan: &Plan, class: PlanChangeClass) -> Plan {
    let mut next = plan.clone();
    match class {
        PlanChangeClass::Material => {
            next.version = plan.version.next_material();
            next.review_state = ReviewState::Draft;
        }
        PlanChangeClass::Minor => {
            next.version = plan.version.next_minor();
            // 审查态保持：`next` 是输入的克隆，`review_state` 原样不动。
        }
    }
    next
}

/// §228 的**计划内容**（五个列表）是否有任一变化：逐位置比较（长度 ＋ 逐项
/// [`Opaque::as_str`]）——`Opaque` 没有 `PartialEq`，而 `as_str` 上的相等就是
/// 「同一个值」在本层的可判形态（同 Task 12 的 `diff` 那一处）。
///
/// **只看这五个列表**：`id` / `version` / `intent_id` / `r#type` / `review_state`
/// 不是「内容」（版本推进是 [`apply_plan_change`] 的事、审查态是状态机的事）——
/// 本层不把它们接进内容判定。
fn plan_content_changed(before: &Plan, after: &Plan) -> bool {
    /// 两份列表是否逐项相等（长度 ＋ 逐位置 `as_str`）。
    fn same_list(a: &[Opaque], b: &[Opaque]) -> bool {
        a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.as_str() == y.as_str())
    }

    !same_list(&before.objectives, &after.objectives)
        || !same_list(&before.milestones, &after.milestones)
        || !same_list(&before.assumptions, &after.assumptions)
        || !same_list(&before.risks, &after.risks)
        || !same_list(&before.acceptance_conditions, &after.acceptance_conditions)
}
