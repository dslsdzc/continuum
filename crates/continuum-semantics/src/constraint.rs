//! §225 的 Contract 不可越权原则与 §6 的 Constraint Violation Gate 的判定
//! （设计 §11.2／§11.3／§11.4）。
//!
//! 计划「文件结构」一节把 `CandidatePlan、PlanFootprint、ConstraintVerdict、
//! ViolatedProhibition、ConstraintError、validate` 归在 `src/constraint.rs`
//! （`docs/superpowers/plans/2026-10-06-p4-semantic-layer.md:312`）。
//!
//! # 两条拦下路径，加一条「判不了」（§11.3）
//!
//! §6 的图给的是一条拦下路径：「违反 REQUIRED」；§225 另给一组**不依赖任何
//! Requirement 的 `class`** 就能判的禁令。两臂都在本文件：
//!
//! | §225 的 MUST NOT | 落在哪一臂 |
//! |---|---|
//! | 修改 REQUIRED | [`ConstraintVerdict::ViolatesRequired`]（`constraints` 一侧的判定） |
//! | 扩大权限 | [`ConstraintVerdict::ViolatesProhibition`]（[`ViolatedProhibition::Authority`]） |
//! | 降低隐私约束 | [`ConstraintVerdict::ViolatesProhibition`]（[`ViolatedProhibition::Privacy`]） |
//! | 擅自增加高风险外部副作用 | [`ConstraintVerdict::ViolatesProhibition`]（[`ViolatedProhibition::Effect`]） |
//!
//! **「判不了」走 `Err`，不走第三个裁决值**（§11.3 判据 1、2）：
//! 一个悬空的约束键必须被拦下，而「拦下」在 `Result` 上的形态是
//! [`ConstraintError::UnknownConstraintKey`]——`Satisfied` 会让它以「已判过、通过」
//! 的形式流下去（fail-open）；往 `ConstraintVerdict` 里加第三支等于把「本层判不了」
//! 说成一个 §6 的结论（§6 只给「满足 Contract／违反 REQUIRED」两种结果）。
//!
//! # 输入面（§11.2）：候选计划是值；图侧输入以原样类型进入
//!
//! 「候选计划」＝ [`CandidatePlan`]：§228 的 [`Plan`] ＋ 声明的影响面
//! [`PlanFootprint`]，两部分都由调用方以值给出。图的 `Node.constraints` 以
//! `&[String]` 进入、`Node.execution_policy` 以 `&serde_json::Value` 进入，
//! **不经过本层的类型转换**（§11.2、§13.2 末行）——本 crate 对 `continuum-graph`
//! 的引用为零（`ALLOWED` 条目 ＋ Task 21 的 grep；照片在 `tests/constraint.rs` 的
//! `the_node_side_inputs_keep_their_original_types`）。
//!
//! # 三条判据的原文对齐（§11.3）
//!
//! 1. **一个悬空的约束键不是「满足」**：它意味着写它的那一方与当前 Contract
//!    对不上（计划是在旧 Contract 下写的、或键拼错了），故必须被拦下——
//!    形态是 `Err`（[`ConstraintError::UnknownConstraintKey`]）。
//! 2. **它也不该变成一个 §6 的裁决值**：`Result` 的 `Err` 不可被忽略地当成通过。
//! 3. **触发条件是「不在 id 集合里」，不是「不指向 REQUIRED」**——`Err` 的判据
//!    与 `class` 无关；**命中**一条非 REQUIRED 的键时，裁决由该 Requirement 的
//!    `class` 决定（`PREFERRED`／`FLEXIBLE`／`UNSPECIFIED` 一律**不拦下**）。
//!    没有这一格，「不指向 REQUIRED 就报错」的实现会绕掉「三类违反不拦下」。
//!
//! # `execution_policy` 的读法（**本设计的读法**，收件人：复审者）
//!
//! §11.2 说「`Node.execution_policy` 的判定按 §225 的 MAY/MUST NOT 两条清单」。
//! 但 `execution_policy` 的**结构规范未给**（§236 只有字段名，`node.rs:17` 的
//! `/// 结构保存，判定属 P4。` 也没给），故本层要选一个读法，申报如下：
//!
//! - **形状**：把 `execution_policy` 读作**一组变更名的 JSON 数组**——每个字符串
//!   元素是一条「本次执行拟做出的变更」的声明，字面取清单在**文档里逐字出现过的
//!   写法**：MAY 七条照 §225（`docs/spec/05-normative.md:265-275`）；MUST NOT 后
//!   三条照 §225（同文件 `:277-284`）＋设计 §11.2 正文的省略版
//!   （`design:1377`——它明说该写法「属 MUST NOT（拦下）」）。中文原文，
//!   本层不翻译（翻译等于发明一张新词表）。
//! - **判定**：命中 MUST NOT 后三条（[`PROHIBITED_CHANGES`]）→
//!   [`ConstraintVerdict::ViolatesProhibition`]（首个命中即返回，按数组序）；
//!   命中 MAY 七条（[`MAY_CHANGES`]）→ 放行；**不在两张清单上的条目 → 也不拦**。
//! - **边界与代价（照实写）**：
//!   - 形状**不是数组**（含 `Value::Null`——`Node::new` 的默认值）时视为
//!     **没有任何变更声明**，不拦。否则每个未设置策略的节点都会被拦。
//!   - 「不在两张清单上的条目不拦」是**本读法定下的处置**：设计只给了两条清单，
//!     **未知条目的处置未给**，而 [`ConstraintError`] 的触发条件设计已写死为
//!     「不在 Requirement id 集合里」（那是 `constraints` 一侧的）——本层不为
//!     `policy` 一侧发明 Err。**代价**：一个拼错的禁令名（如「扩大权根」）会被
//!     视为未知条目而放行（fail-open 侧）；要收口须由规范给出 `execution_policy`
//!     的结构与词表，或明写「未知条目一律拦下」。
//!   - 数组元素的字面是**逐字比对**（同「约束引用键」的按字符串比对）：同义写法
//!     **不归一化**（例：「扩大权根」「降低隐私级别」不命中任何一条）。**唯一带
//!     两个字面的条目是第三条禁令**：「擅自增加高风险外部副作用」（§225 原文）与
//!     「增加高风险外部副作用」（设计 §11.2 正文与 task brief 的转述，§11.2 明说
//!     它「属 MUST NOT（拦下）」）**都拦**——两个都是文档逐字有源的字面，
//!     不是本层的词表归一化（fix 轮 1 订正：此前只收含「擅自」的那一条）。
//!
//! §11.2 的用例数**写死：7（MAY 逐项放行）＋ 3（MUST NOT 后三条逐项拦下）＝ 10**
//! ——来历（初稿「14 项」的订正、7＋4＝11 也不是 14、第四条 MUST NOT「修改 REQUIRED」
//! 不是本表的行而是 `ViolatesRequired` 那一路）写在 `tests/constraint.rs` 的文件头。
//!
//! # 处置顺序（**本实现定**的，设计未给）
//!
//! 1. `constraints` 先**全扫一遍**找悬空键——任何悬空键 → `Err`；
//! 2. 再扫 `constraints` 的 REQUIRED 命中（扫描序第一个）；
//! 3. 最后判 `execution_policy`。
//!
//! 即「判不了」优先于裁决：输入里有无法解释的部分时，本层先报出它。设计没有给
//! 顺序判据（用例也没有钉「同时出错」的组合），此处照实申报。
//!
//! # `plan` 今天不被读（申报，收件人：复审者）
//!
//! [`validate`] 收 [`CandidatePlan`]，但**不读 `plan` / `footprint` 的字段**：
//! §225 后三条的「**计划的声明值**与 Contract 的 `authority_limit` / `data_policy`」
//! 之比需要值的语义（两边的字段都是不透明值），而「怎么比」设计未给——本层不为它
//! 发明判据。本 task 可判的两条路径（约束引用键、`execution_policy` 声明）都用不到它。
//! `plan` 参数是 §11.2 的**输入面**（候选计划的两部分以值进入），不是漏读；
//! [`PlanFootprint`] 是「让 §225 有判据」的载体——Task 14 的 `classify` 是它的
//! 第一个消费者（收 `&PlanFootprint` 判「权限扩大／成本显著扩大」）。
//!
//! # `ConstraintError` 为何落在本文件而不是 `error.rs`（申报，收件人：复审者）
//!
//! 计划「文件结构」一节里它**出现两次**（line 312 的 `constraint.rs` 行与 line 314
//! 的 `error.rs` 行——同一件事两行各属一次的既有形状）。本实现的取法：
//! 按 line 312 落此处，且本 task 的 Files 清单（brief）不含 `error.rs` 的改动。
//! 要移动的话是一处一字之改。
//!
//! # §11.5 的守卫（本文件这一侧）
//!
//! 本文件**不读预算**、不引用 `continuum-budget`——§11.5 第 1 条守卫
//! （Constraint Validator 与 Budget Validator 两道门分离；`ALLOWED` 条目里没有
//! `continuum-budget` 由 `crates/continuum-runtime/tests/dependency_direction.rs`
//! 机械断言）。第 3 条守卫（`continuum-budget` 一侧没有任何以
//! [`ConstraintVerdict`] 为输入的函数）归 Task 17／19。
//!
//! # §11.4：本组件不挂起、不写 Intent 状态
//!
//! `Violates*` 由调用方转成 `Decision` 与 Intent 的 `SUSPENDED`——
//! [`validate`] **只返回裁决**：它不收 `&Tx`、不写任何表、不读 Intent 存储
//! （§2.3 的层内依赖把它写成 `Constraint Validator ← Contract + Plan`）。
//! 挂起与建 Decision 是调用方（本 crate 的 Intent 路径，Task 15）的下一步。
//!
//! # 用例
//!
//! 十二条用例（输入面 2 ＋ `execution_policy` 10 条折成 2 个表驱动 fn ＋ 原样类型 1 ＋
//! 裁决 5 条折成 2 个 ＋ 「判不了」三侧 3 ＋ **读法边界 2**）全在 `tests/constraint.rs`，
//! 逐条的红条件写在那里的文件头。

use continuum_canonical::Opaque;

use crate::contract::{RequirementClass, TaskContract};
use crate::ids::RequirementId;
use crate::plan::Plan;

/// 候选计划（§11.2 的本设计类型）＝ §228 的 [`Plan`] ＋ 声明的影响面
/// [`PlanFootprint`]，**两部分都由调用方以值给出**（§1.2.3 规则 3：跨层输入以值进入）。
///
/// 字段是 `pub` 的（同 [`Plan`] / [`TaskContract`]）：两个引用由调用方现场拼，
/// 本层没有比 §11.2 更强的约束可挂。字段清单**逐项**的照片是
/// `tests/constraint.rs` 的 `the_candidate_plan_is_a_value_of_a_plan_plus_a_footprint`
/// （穷尽解构、**不带 `..`**）。
#[derive(Debug, Clone, Copy)]
pub struct CandidatePlan<'a> {
    /// §228 的候选计划本体。
    pub plan: &'a Plan,
    /// 候选计划的声明影响面（§11.2；见 [`PlanFootprint`] 的文档）。
    pub footprint: &'a PlanFootprint,
}

/// 计划的**声明影响面**（§11.2 的本设计类型）。
///
/// # 本类型的由来（不是本层新造的判据）
///
/// §228 的 [`Plan`] **没有**这几个字段，而 §225 的四条 MUST NOT 的后三条比的是
/// **计划的声明值**与 Contract 的 `authority_limit` / `data_policy`（§224 有这两个
/// 字段）。没有计划侧的对偶声明，后三条**无可判**。**故本类型是「让 §225 有判据」
/// 的载体**——判据是 §225 给的，本类型只是它的第二个操作数。
///
/// 四个字段一律 [`Opaque`]：§11.2 只给名字与「不透明」的处置（同 [`TaskContract`]
/// 的六个声明字段）；「计划声明的权限**超过**契约允许」这类比较需要值的语义，
/// 那不是本层今天能做的（见模块文档「`plan` 今天不被读」一节）。
/// **第一个读它的消费者是 Task 14 的 `classify`**（判「权限扩大／成本显著扩大」，
/// 比较由调用方给的阈值入参承载）。
///
/// 字段清单**逐项**（穷尽解构、**不带 `..`**）的照片是 `tests/constraint.rs` 的
/// `the_four_footprint_fields_are_all_declared`。
#[derive(Debug, Clone)]
pub struct PlanFootprint {
    /// 计划要用的权限上限。
    pub authority: Opaque,
    /// 计划触及的最高隐私等级。
    pub privacy_class: Opaque,
    /// 计划要产生的**外部副作用**（类型 ＋ 目标）。
    pub external_effects: Vec<Opaque>,
    /// 计划的成本估计（§110 的输入，§12.4）。
    pub estimated_cost: Opaque,
}

/// §6 的裁决：**两条**拦下路径，加一条「满足」——**不是**「通过／违反／不确定」
/// （「不确定」在 §6 里没有位置；「判不了」走 [`ConstraintError`]，见模块文档）。
///
/// # 两枚拦下与 §225 的对应
///
/// - [`ConstraintVerdict::ViolatesRequired`]：「修改 REQUIRED」那一臂——
///   `constraints` 里的键**命中**一条 `class = REQUIRED` 的 Requirement
///   （§11.3 判据 3 的订正段：「命中之后**按 REQUIRED 那一路去判**」）；
/// - [`ConstraintVerdict::ViolatesProhibition`]：§225 的 MUST NOT 后三条——
///   `execution_policy` 声明的变更命中 [`PROHIBITED_CHANGES`]，是哪一枚见
///   [`ViolatedProhibition`]。
///
/// # 派生（为什么没有 `PartialEq`）
///
/// `Debug` 是断言失败时必须打得出来；`Clone` 是裁决要随值传递（调用方转
/// `Decision` 与 `SUSPENDED` 时用）。**没有 `PartialEq`**：本类型带
/// [`RequirementId`]，而本层给 id 四枚的纪律是不派生 `PartialEq`
/// （见 `ids.rs` 的模块文档：本层的断言都经 `as_str` 比较）；
/// 测试因此经 `match` 读出是哪一枚（`tests/constraint.rs`）。
#[derive(Debug, Clone)]
pub enum ConstraintVerdict {
    /// 满足 Contract（§6 的「满足 Contract → Execute」一侧）。
    Satisfied,
    /// 违反 REQUIRED（§6 的「违反 REQUIRED → Suspend」一侧）。
    /// `requirement` 是**命中的那一个** id（原样带出）。
    ViolatesRequired {
        /// 被命中的 REQUIRED 约束。
        requirement: RequirementId,
    },
    /// 违反 §225 的 MUST NOT 后三条之一。`what` 指出是哪一枚。
    ViolatesProhibition {
        /// 命中的那一条禁令。
        what: ViolatedProhibition,
    },
}

/// §225 的 MUST NOT 后三条，**三枚**（「修改 REQUIRED」不在此枚举——它是
/// [`ConstraintVerdict::ViolatesRequired`] 那一路）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViolatedProhibition {
    /// 扩大权限。
    Authority,
    /// 降低隐私约束。
    Privacy,
    /// 擅自增加高风险外部副作用。
    Effect,
}

/// 「判不了」的错误（§11.3 判据 1、2）——**不走第三个裁决值**。
///
/// # 只有一枚，触发条件写死
///
/// [`ConstraintError::UnknownConstraintKey`] 的触发条件是「**`key` 不在当前
/// Contract 的 Requirement id 集合里**」，**与 `class` 无关**（§11.3 判据 3）。
///
/// **没有第二枚**：`execution_policy` 一侧「不在两张清单上的条目」的处置设计未给
/// （见模块文档的读法边界），本层不为它发明 Err。
///
/// 本类型落在本文件而非 `error.rs` 的申报见模块文档。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConstraintError {
    /// `Node.constraints` 里的一个串**不在当前 Contract 的 Requirement id 集合里**
    /// （悬空引用：计划是在旧 Contract 下写的、或键拼错了）。`key` 是**那一个**
    /// 原始串，不改写、不裁剪。
    #[error("constraints 里的键不在当前 Contract 的 Requirement id 集合里: {key}")]
    UnknownConstraintKey { key: String },
}

/// §225 的 MAY 清单（**七条**）：Runtime 可以自动改变 HOW。**逐字照 §225**
/// （`docs/spec/05-normative.md:265-275`，中文原文——本层不翻译）。
///
/// 命中的条目经 [`classify_change`] 归 [`ChangeKind::May`]，[`validate`] 放行。
const MAY_CHANGES: &[&str] = &[
    "换模型",
    "换工具",
    "换节点",
    "换算法",
    "重新规划",
    "改变并行度",
    "改变内部执行顺序",
];

/// §225 的 MUST NOT **后三条**（第四条「修改 REQUIRED」不在此表——它走
/// [`ConstraintVerdict::ViolatesRequired`]）。**第三条禁令有两个文档字面**：
///
/// - 「擅自增加高风险外部副作用」——§225 原文逐字（`docs/spec/05-normative.md:277-284`，
///   含「擅自」二字；§11.3 的对应表亦逐字如此）；
/// - 「增加高风险外部副作用」——设计 §11.2 正文与 task brief 的转述
///   （`design:1377` **明说它「属 MUST NOT（拦下）」**）。
///
/// **两个写法都是文档逐字出现过的字面**（不是本层发明的词表），且描述同一条禁令，
/// 故**两条都拦**（第三条的 `Effect` 因此有两行，枚仍只三枚）。来历见 fix 轮 1
/// （brief 的验收字面是省略版）。前两条禁令各只有一个文档字面，照录。
const PROHIBITED_CHANGES: &[(&str, ViolatedProhibition)] = &[
    ("扩大权限", ViolatedProhibition::Authority),
    ("降低隐私约束", ViolatedProhibition::Privacy),
    ("擅自增加高风险外部副作用", ViolatedProhibition::Effect),
    ("增加高风险外部副作用", ViolatedProhibition::Effect),
];

/// `execution_policy` 里一条变更声明经 §225 两张清单分类的结果。
enum ChangeKind {
    /// 命中 §225 的 MAY 清单（七条）——放行。
    May,
    /// 命中 §225 的 MUST NOT 后三条——拦下，是哪一枚。
    Prohibited(ViolatedProhibition),
}

/// 把一条变更声明按 §225 的两张清单分类。**定义域是所有串**：
/// 不在任何清单上的条目得 `None`（[`validate`] 对它的处置见模块文档）。
fn classify_change(name: &str) -> Option<ChangeKind> {
    if let Some((_, what)) = PROHIBITED_CHANGES.iter().find(|(word, _)| *word == name) {
        return Some(ChangeKind::Prohibited(*what));
    }
    if MAY_CHANGES.contains(&name) {
        return Some(ChangeKind::May);
    }
    None
}

/// §6 的 Constraint Validator：判候选计划是否满足 Contract。
///
/// # 判定（逐条与模块文档对齐）
///
/// 1. `constraints` 先**全扫一遍**找悬空键（任何位置）→ [`ConstraintError`]；
/// 2. 再扫 REQUIRED 命中（扫描序第一个）→ [`ConstraintVerdict::ViolatesRequired`]；
/// 3. `execution_policy` 声明的变更逐条按 §225 两张清单分类 → 命中禁令即
///    [`ConstraintVerdict::ViolatesProhibition`]；
/// 4. 其余 → [`ConstraintVerdict::Satisfied`]。
///
/// §11.4：本组件**自己不挂起、不写 Intent 状态**——只返回裁决（不收 `&Tx`、
/// 不写任何表）。`plan` 参数今天不被读的理由见模块文档。
///
/// # 照片
///
/// `tests/constraint.rs` 的十二条用例：签名两条（`fn` 指针标注 ＋ 穷尽解构）、
/// `execution_policy` 的 7＋3 条逐项、裁决五条、判不了三侧、**读法边界两条**
/// （表外条目不拦／非字符串元素跳过）。逐条的红条件在那个文件的文件头。
pub fn validate(
    plan: &CandidatePlan<'_>,
    contract: &TaskContract,
    constraints: &[String],
    execution_policy: &serde_json::Value,
) -> Result<ConstraintVerdict, ConstraintError> {
    // 本函数今天**不读** `plan` 的字段（见模块文档「`plan` 今天不被读」）：
    // §225 后三条的「计划声明值 vs Contract」之比需要值的语义，而「怎么比」
    // 设计未给——本层不为它发明。消费掉绑定只是免得 unused-variable 警告
    // （同 `spec.rs` 的 `let _ = (…)`）。
    let _ = plan;

    // (1) 悬空键：任何位置先于裁决（本实现定的处置顺序：判不了优先于裁决）。
    for key in constraints {
        if !contract.requirements.iter().any(|r| r.id.as_str() == key) {
            return Err(ConstraintError::UnknownConstraintKey { key: key.clone() });
        }
    }

    // (2) 键命中 REQUIRED：扫描序第一个。
    for key in constraints {
        let matched = contract
            .requirements
            .iter()
            .find(|r| r.id.as_str() == key)
            .expect("上面第一遍扫描已保证每个键都命中");
        if matched.class == RequirementClass::Required {
            return Ok(ConstraintVerdict::ViolatesRequired {
                requirement: matched.id.clone(),
            });
        }
    }

    // (3) execution_policy：声明的变更逐条按 §225 两张清单分类。
    if let Some(names) = execution_policy.as_array() {
        for name in names {
            let Some(name) = name.as_str() else {
                // 非字符串元素不是变更名——不参与判定（读法边界见模块文档）。
                continue;
            };
            match classify_change(name) {
                Some(ChangeKind::Prohibited(what)) => {
                    return Ok(ConstraintVerdict::ViolatesProhibition { what });
                }
                // MAY 清单的条目放行；不在两张清单上的条目也不拦（设计未给它的
                // 处置，本层不为它发明 Err——见模块文档的读法边界）。
                Some(ChangeKind::May) | None => {}
            }
        }
    }

    // (4) 其余 → 满足 Contract（§6 的「满足 Contract → Execute」一侧）。
    Ok(ConstraintVerdict::Satisfied)
}
