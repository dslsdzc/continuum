//! §277 的 `ExplicitBinding`／`BindingSource`，以及 §4.6 的**唯一注入点** [`apply`]。
//!
//! # 注入是**调用方的入参**，不是由 `BoundTo` 约束自动发生的东西
//!
//! 设计 §4.6 逐字给出的是（`docs/superpowers/specs/2026-10-06-p4-semantic-layer-design.md:634-638`，
//! 行号本 task `grep -n` 实测）：
//!
//! ```text
//! // 工作集 = 检索集 ＋ binding 注入（§4.6）。A 约束的是这个式子里的两个来源，不是工作集本身。
//! let working = retrieved.clone();
//! if let Some(b) = binding { working.push(injected_candidate(b)); }   // 唯一允许的注入点
//! ```
//!
//! ——`binding` 是**调用方带来的一枚值**，故 `resolve` 收它作参数（本模块的 [`apply`] 就是那个
//! `if let Some(b) = binding` 那一步）。**这里不是另一个读法**：约束（[`crate::ExplicitConstraint`]）
//! 是**判据**（「候选必不必须绑到某实体」），binding 是**事实**（「用户点名了某一枚」），
//! 两者位置相反——约束进 `constraints`，binding 进注入点。**若反过来**（由 `BoundTo` 约束
//! 自动注入被点名的实体），则**一条「必须绑到 X」的约束会自己制造出一个 X 候选**，
//! 而在候选集里没有 X 时，那条约束就从「筛子」变成了「造币机」——§4.2 的断言 A
//! 正是防这个（「解析器自造实体」）。**这一处分岔有照片**：
//! `tests/resolver.rs::assertion_a_injection_guard` 的调用点传 `None`，它钉的就是
//! 「除 `ExplicitBinding` 之外没有第二个来源」，也即**自动注入这条路在本层不存在**。
//!
//! # `ExplicitBinding` 的两个「不是」
//!
//! - **不是加权**：§277 的「最高优先级」落在**取值顺序**上——[`apply`] 在过滤**之前**
//!   把工作集**缩到那一枚**（`design:779-781`），而不是给它的 `retrieval_score` 加分再排序。
//!   照片：`tests/risk.rs::an_explicit_binding_shrinks_the_candidate_set_instead_of_weighting_it`
//!   （与它冲突的那一枚检索分数更高，仍不改变结果）。
//! - **不是约束的豁免**：注入的那一枚**同走** [`crate::qualifying`] 那一道
//!   ——binding 只提高**检索**这一侧，不豁免 `ExplicitConstraint`（`design:797-799`）。
//!   照片两条，两侧都钉：`tests/risk.rs` 的
//!   `a_binding_whose_entity_is_not_in_the_retrieved_set_is_injected_and_resolved`（照片 (a)）
//!   与 `a_binding_does_not_exempt_the_entity_from_the_explicit_constraints`（照片 (b)）。

use crate::candidate::{Candidate, EntityId};
use crate::explicit::{CandidateFacts, CandidateSet, EntityKind, ResolutionScope};
use crate::surface::Mention;

/// §277 的 `source`。**闭集一枚**：§277（`docs/spec/05-normative.md:1464-1484`）
/// 的代码块里 `source = USER` 是唯一一个取值。
///
/// # 为什么是一枚枚举、而不是一个常量或 `bool`
///
/// 它是「这条绑定**由谁产生**」这一维——本层今天只知道一个答案（用户），
/// 而**这一维是有内容的**：设计 §4.6 把它写进了 `ExplicitBinding` 的字段。
/// 取一个常量（如 `()`）会让这一维在类型上消失，「将来多一个来源」就成了一次
/// **改形状**而不是加一枚变体。
///
/// **反面**：若规范明天给第二个来源，照 §277 的先例加一枚变体即可——
/// 判据是 `tests/risk.rs::an_explicit_binding_is_recorded_with_its_scope_and_source` 里那个
/// **不带 `_` 臂**的穷尽 `match`：加一枚变体即**编译不过**（`E0004`）。
/// **照实记**：那是编译期照片，运行期无红（本仓纪律：编译不过不算变红）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingSource {
    /// §277 的 `USER`：这条绑定由用户产生。
    User,
}

/// §277 的 `ExplicitBinding`：用户点名了一条绑定。
///
/// 四字段逐字照录 §277 的 `ExplicitBinding { mention, entity, scope, source = USER }`
/// （`docs/spec/05-normative.md:1464-1484`）。**字段是 `pub` 的**——同 [`crate::Candidate`]
/// 与 [`CandidateFacts`] 的理由：没有不变量可挂，四个字段各自是一个已封闭或本层不解释的值。
/// 照片：`tests/risk.rs::an_explicit_binding_is_recorded_with_its_scope_and_source`
/// 用**不带 `..`** 的穷尽解构钉住字段清单恰好是这四个。
///
/// # `scope` 取 §274 的 [`ResolutionScope`]，与 [`crate::ExplicitConstraint::Scope`] 同一套词汇
///
/// §277 只给了 `scope` 这一维的**名字**，没给它取哪一套词汇。本层取 §274 的九值，
/// 依据是 §4.1.1 的分工（`src/explicit.rs` 的文件头与本类型的会话都写着）：
/// 「`scope` 那一维归 `Scope` 那一枚」——同一个概念在本层只应有一套取值域，
/// 否则「这条 binding 的 scope」与「那条约束的 scope」会各自漂移。
///
/// # 这个结构**不表示**「用户的选择已经被检索支持」
///
/// §277 的字面前提是「用户**选择候选**」（候选已由检索给出），而本层用同一个结构
/// 处理「用户**直接点名**一个检索没给出的实体」——**后者是本设计补的一次扩张**，
/// 不是 §277 的原文（`design:787-793`，**收件人：规范维护者**）。
/// 扩张的理由：用户的权威高于检索（§277 的「SHOULD 在当前 scope 拥有最高优先级」），
/// 若因为注册表没检索到就把它拒在集合外，等于**让检索结果否决用户**。
#[derive(Debug, Clone)]
pub struct ExplicitBinding {
    /// 用户提到的那一枚 mention（§270 的 [`Mention`]）。
    pub mention: Mention,
    /// 它绑到的实体（§272 的 [`EntityId`]）。
    pub entity: EntityId,
    /// 这条绑定所在的作用域（§274 的九值；见上）。
    pub scope: ResolutionScope,
    /// 这条绑定由谁产生（§277 的 `source = USER`，见 [`BindingSource`]）。
    pub source: BindingSource,
}

/// §4.6 注入的那一枚候选的 `evidence` 标记（`design:784-785` 逐字要求）。
///
/// 它是**公开常量**而不是散在 [`apply`] 里的一段字面量：调用方要能认出「这一枚是用户点名的」，
/// 而认的方式是读 `Candidate.evidence`（§272 那个字段就是「来源凭据」）。
/// 名字取 `user_explicit` 与 §277 的 `source = USER` 同源。
///
/// **断言它的是字面量，不是这个名字**：`tests/risk.rs` 的照片 (a) 把期望值写成
/// `vec!["user_explicit".to_owned()]`（而不是引用本常量）——若那条断言写成
/// `vec![USER_EXPLICIT_EVIDENCE.to_owned()]`，改本常量的值就会**静默跟着改**，
/// 于是「标记恰是 §4.6 说的那个串」这件事就没有照片了（本仓「正控制不得自证」的同一条：
/// 语料若从被测物里取，就是恒真的假照片）。
pub const USER_EXPLICIT_EVIDENCE: &str = "user_explicit";

/// §4.6 的**唯一注入点**：由检索集与（至多一条）binding 算出**工作集**。
///
/// # 两条输入、三种情形（写全，因为「缩」与「注入」是两条不同的路）
///
/// 1. **没有 binding** ⇒ 工作集 ≡ 检索集（原样克隆，一枚不改）。
/// 2. **有 binding，且 `entity` 已在检索集里** ⇒ 工作集**缩到那一枚**
///    （§277 的「最高优先级」＝**取值顺序**：`design:779-781` 的「在 `CandidateSet`
///    过滤之前先把候选集缩到 `entity` 这一个，**不是给它加权**」）。
/// 3. **有 binding，且 `entity` 不在检索集里** ⇒ 把它**作为一条候选追加**进工作集
///    （`evidence` 标 [`USER_EXPLICIT_EVIDENCE`]），**检索集的那几枚照样留着**
///    ——设计 §4.6 的照片 (a) 要求 `candidate_count` 比检索集**多 1**
///    （`design:801-803`），故这一支**不是**「缩到空再加一枚」，而是「检索集 ＋ 注入」。
///
/// **「缩」与「追加」是两条路、不是一个式子**：情形 2 与情形 3 的写法不同，
/// **这不是笔误**——§4.2 把工作集定义成「检索集 ＋ binding 注入（至多一次）」（`design:634-638`），
/// 而那是个**只增不减**的式子（`working = retrieved.clone()` 之后至多 push 一次），
/// 故它与**情形 2** 也不同：情形 2 无注入，工作集却**缩到 `entity` 这一个**。
/// 三个分支里只有情形 2 会让工作集比检索集小（情形 1 相等、情形 3 多一枚），
/// 检索集多于一枚时它即检索集的**真子集**
/// （照片：`tests/risk.rs::an_explicit_binding_shrinks_the_candidate_set_instead_of_weighting_it`）。
/// §4.6 把情形 2 表述成「缩到 `entity` 这一个」（`design:779-781`）时，它比上面那个式子**多**了这一步缩，
/// 而这一步**不能**外推到情形 3：「缩」若也施于情形 3，工作集就只剩注入的那一枚、
/// `candidate_count` 会等于 1，与照片 (a) 的「多 1」不符。
/// **本层按照片取**：情形 2 缩、情形 3 追加。设计正文的这一处措辞与它自己的照片不完全对齐，
/// **登记在 `docs/superpowers/p4-followups.md` 的「设计面未闭合」一节（终审 I-3）**。
///
/// # 注入那一枚的**判定事实**从哪来（一处设计没写、本层补的口）
///
/// 设计 §4.6 的草样写的是 `injected_candidate(b)`——只从 binding `b` 派生。
/// 而 `b` 的四个字段里**没有 `kind`**（§277 只给了 `mention` / `entity` / `scope` / `source`），
/// 工作集的一枚却是 `(Candidate, CandidateFacts)`、后者两个字段都要
/// （`src/explicit.rs`：注册表不在本层，故 `scope` 与 `kind` 只能随值进来）。
/// 于是本函数的第二个入参**只收 `kind`**（不收整个 `CandidateFacts`）：
/// - `scope` 从 `b.scope` 来——**一个来源**，故两者不可能互相矛盾；
/// - `kind` 从调用方来——它是那一枚实体在注册表里的种类，**本层查不到**（§1.2.3 的规则 3），
///   而「用户点名」这件事本身不带它。
///
/// **若照草样只收 `b`**，本层就只能**发明**一个 `kind`（四枚里挑一枚）——那是一条没有出处的判据，
/// 且会让「注入的候选在 `EntityKind` 约束下怎么判」变成掷骰子。
/// **这是本层对设计的一次补口，收件人：设计维护者。**
///
/// # 它是 `resolve` 里唯一能把实体放进判定的地方
///
/// 除本函数之外，本层**没有第二个**来源能往工作集里加一枚：`resolve` 收的 `candidates`
/// 就是检索集，而本函数是它之后唯一的一步（`resolve` 的文档写了这个式子）。
/// 照片：`tests/resolver.rs::assertion_a_injection_guard`（那条用例钉「除声明的两个来源外没有第三个」）。
pub fn apply(
    retrieved: &[(Candidate, CandidateFacts)],
    injection: Option<(&ExplicitBinding, EntityKind)>,
) -> CandidateSet {
    match injection {
        // 情形 1：没有 binding，检索集原样进工作集（一枚不改）。
        None => retrieved.to_vec(),
        Some((b, kind)) => {
            // 情形 2：点名的实体已在检索集里 ⇒ 缩到那一枚。
            // 比的是 `EntityId` 里那个串（`EntityId` 没有 `PartialEq`，见 `src/candidate.rs`）。
            if let Some(named) = retrieved
                .iter()
                .find(|(c, _)| c.entity_id.as_str() == b.entity.as_str())
            {
                vec![named.clone()]
            } else {
                // 情形 3：不在 ⇒ 追加一枚（检索集那几枚留着，见文档的照片 (a)）。
                let mut working = retrieved.to_vec();
                working.push((
                    Candidate {
                        entity_id: b.entity.clone(),
                        evidence: vec![USER_EXPLICIT_EVIDENCE.to_owned()],
                        // 注入的那一枚**没有检索分数**：它不是检索出来的，
                        // 故「来源没给分数」那一支（`None`），不是 `Some(0.0)`（§272 的区分）。
                        retrieval_score: None,
                    },
                    CandidateFacts {
                        // `scope` 从 binding 来（一个来源，不可能与它矛盾）；`kind` 从调用方来。
                        scope: b.scope.clone(),
                        kind,
                    },
                ));
                working
            }
        }
    }
}
