//! §276 的四个风险事实 [`ResolutionRisk`]，以及**本层不提供**的那条基线映射。
//!
//! # 规范给到哪一步
//!
//! §276（`docs/spec/05-normative.md:1450-1462`）给的原文只有两句：
//!
//! ```text
//! required_resolution_strength = f(effect, cost, reversibility, privacy)
//! 风险越高，自动解析要求越严格。
//! ```
//!
//! **`f` 的形态没有给**（设计 §4.5，`docs/superpowers/specs/2026-10-06-p4-semantic-layer-design.md:750-773`），
//! 四个维度的**序与量纲也都没有给**。故本模块的处置与 D 对 `BudgetView` 的处置同形：
//! **把没给判据的东西做成不可省略的输入**（[`crate::RequiredStrength`]，它是 `resolve` 的必填入参），
//! 而**不发明一个 `f`**。
//!
//! # 本模块**没有**的东西（这一节是本模块定义的一半）
//!
//! - **没有** `impl From<ResolutionRisk> for RequiredStrength`；
//! - **没有** `fn from_risk` 一类的具名构造函数；
//! - **本层没有读这四个字段来做判定的代码**——唯一会那样做的是一条「风险 → 门槛」的基线，
//!   而那条路被两处照片钉住（`tests/risk.rs::the_layer_provides_no_baseline_mapping_from_risk_to_strength`
//!   的源码清单 ＋ `tests/compile_fail/required_strength_has_no_from_risk.rs` 的编译期反例）。
//!   本类型连访问器都没有，四个字段是 `pub` 的
//!   （它是纯数据记录，同 [`crate::Candidate`]，理由见那个类型）。
//!
//! 理由不是洁癖：「风险越高要求越严格」是一条**单调性**要求，而单调性需要一个**序**——
//! `cost` 的高、`reversibility` 的低、`privacy` 的高，四者之间的序规范没有给，
//! 故**一个「保守的」基线也无从写起**：无中生有的序会把「高成本」判成「低风险」，
//! **基线比没有更坏**（设计 §4.5 写死，`design:767-771`）。
//!
//! **这张「没有」的清单有照片，且两侧都有**：
//! 反面是 `tests/compile_fail/required_strength_has_no_from_risk.rs`（两枚路各试一次，
//! `.stderr` 实跑取值）与 `tests/risk.rs::the_layer_provides_no_baseline_mapping_from_risk_to_strength`
//! 的源码清单；正面（正控制）是同一份用例里 `RequiredStrength::new` 可用那一段——
//! 没有它，反面的绿可以来自「名字整个不存在」这类与判据无关的原因。
//!
//! **为什么没有运行期照片**：要拍「风险越高要求越严」，得先有四个维上的序，而那个序没有规范出处。
//! **这不是「没有照片」，是「没有判据」**（设计 §15.2 第 2 条）。

use crate::Opaque;

/// §276 的四个风险事实：`effect` / `cost` / `reversibility` / `privacy`。
///
/// # 字段名逐字照录 §276，四个都是[不透明值](Opaque)
///
/// §276 只给了这四个名字与那条单调性要求，**四者的取值域规范未给**（§16 第 6 条）。
/// 故本层**保存事实**、不**发明判据**——与 [`crate::MentionKind`] 同一条理由：
/// 把没有取值域的维度封成一个本层编出来的枚举，就是发明一条判据。
///
/// # 字段是 `pub` 的（与 [`crate::CanonicalRepresentation`] 相反）
///
/// 同 [`crate::Candidate`] 的理由：**没有不变量可挂**。本类型是四个不透明值的笛卡尔积，
/// 每一个组合都是合法的风险事实——把它锁起来只会让每个调用方多写一个构造函数，
/// 而不变量一条也不会多。**这条选择本身有照片**：
/// `tests/risk.rs::the_risk_facts_are_four_and_their_field_names_are_recorded_verbatim`
/// 用**不带 `..`** 的穷尽解构钉住字段清单恰好是这四个（crate 外写得出，正因为字段是 `pub`）。
///
/// # 它今天**没有消费者**
///
/// 本层没有任何函数读它（连访问器都没给）：门槛 [`crate::RequiredStrength`] 由调用方算好传来，
/// 而「从风险事实算出门槛」那一步正是本层不提供的 `f`（见模块头）。
/// **本类型的存在不是为了被读，而是为了让「`f` 没给」这件事在类型上有落点**——
/// 一个调用方若想算门槛，手上必然先要有这四枚事实。
#[derive(Debug, Clone)]
pub struct ResolutionRisk {
    /// §276 的 `effect`：待执行动作的效应。
    pub effect: Opaque,
    /// §276 的 `cost`：成本。
    pub cost: Opaque,
    /// §276 的 `reversibility`：可逆性。
    pub reversibility: Opaque,
    /// §276 的 `privacy`：隐私等级。
    pub privacy: Opaque,
}
