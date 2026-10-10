//! §4.1 的四态、§4.4 的两个数不同轴、以及 Reference Resolver 的入口 [`resolve`]。
//!
//! # 四态的定义从 §206 来，§271 只给名字
//!
//! §271（`docs/spec/05-normative.md:1366-1376`）只列了四个名字；**它们的含义在 §206**
//! （`docs/spec/04-method.md:865-899`）。设计 §4.1 的表（`docs/superpowers/specs/2026-10-06-p4-semantic-layer-design.md:360-372`，
//! 行号本 task `grep -n` 实测）把四条折成本层读法，本文件逐字实现那一张表：
//!
//! | 态 | 本层的判定读法 |
//! |---|---|
//! | [`Resolved`] | 约束解出**恰好一个**候选，且该候选满足**全部显式约束**，且门槛已过 |
//! | [`ConfirmRequired`] | 存在一个占优候选，但**门槛未过** |
//! | [`Ambiguous`] | 满足全部显式约束的候选**多于一个** |
//! | [`Unknown`] | 满足全部显式约束的候选**为零**（哪一枚「零」见 [`UnknownReason`]） |
//!
//! **四支都是返回值，不是错误**：§206 的 `abstain` 已把「没有可靠解析」收进 `Unknown`，
//! 而本层**没有第二种失败**（输入全是值、不做 I/O、不落库），故 [`resolve`] 的返回类型
//! 就是 [`ResolutionResult`]，**没有 `Err` 支**（设计 §4.1 写死，`design:433-437`）。
//!
//! # 本文件与 `ambiguity.rs` 的分工（本 task 的一处搬家）
//!
//! `ResolutionResult` 的**类型本体**原先在 `ambiguity.rs`（Task 2 落的是「是哪一态」那一层，
//! 载荷留空，那一处接缝写在那个模块的文件头）。**本 task 把类型本体搬到这里**，
//! 因为载荷（`EntityId` / `Candidate` / `ExplicitConstraint`）分别在 Task 3 与 Task 4 才存在。
//! `ambiguity.rs` 今天只用它做一次分流（`Resolved` 走一支、其余三枚走另一支）。
//!
//! # 四个载荷类型的字段是**私有**的：crate 外**构造不出** `ResolutionResult`
//!
//! 四个载荷（[`Resolved`] / [`ConfirmRequired`] / [`Ambiguous`] / [`Unknown`]）的字段
//! 一律私有，且**没有一个公开构造函数**——唯一的构造点是 [`resolve`]（本文件内）。
//! 故「一个 `ResolutionResult` 是不是 `resolve` 产出的」在本层**有类型判据**：
//! crate 外写不出那四个结构体字面量。
//! 照片：`tests/compile_fail/resolution_result_is_not_constructible_outside.rs`
//! （`.stderr` **实跑取值**，钉的是 `E0451: field … is private`）；**正控制**是
//! `tests/resolver.rs` 里那些**经 `resolve` 拿到载荷**的用例——没有它们，上面那份反例的绿
//! 可以来自「`Resolved` 这个名字整个不存在」这类与判据无关的原因（本仓「正控制不得自证」）。
//!
//! **读得出去，构造取不回来**：四个载荷各配一组**只读访问器**
//! （[`Resolved::entity`] / [`Resolved::satisfied`] / [`Resolved::candidate_count`] 等），
//! 供 crate 外的消费者读判定结果——Task 5 的四条断言（§4.2）就在 `tests/resolver.rs` 里读它们
//! （计划 `docs/superpowers/plans/2026-10-06-p4-semantic-layer.md` 的 Task 5 逐条点了
//! `Resolved.entity` / `Resolved.candidate_count` / `Ambiguous.surviving` / `Unknown.reason`）。
//! 访问器是**读**，不是构造：拿到一枚 `ResolutionResult` 之后看得见，但拼不出新的一枚。
//!
//! **字段清单那张照片因此搬进了本文件的 `#[cfg(test)]`**：§4.1 要求「四支各带哪些字段」
//! 逐项可断言，而可断言的机制是对四支**逐支穷尽解构、不带 `..`**——穷尽解构要在字面量里
//! 命名字段，而字段私有后只有 crate 内做得到。照片是
//! [`tests::every_arm_carries_exactly_its_declared_fields`] 与
//! [`tests::resolution_result_carries_no_score`]（本仓已有「照片住在 `src` 的 `#[cfg(test)]` 里」
//! 的体例）。
//!
//! **为什么不是「字段留 `pub`」**：那两条要求（字段清单可穷尽断言、crate 外构造不出）
//! 在字段 `pub` 时不可兼得；而「构造不出」那一侧今天有真实的收件人——它就是本层
//! 「四态只由 `resolve` 产出」这条不变量的判据，而 Task 5 的四条断言核的是**值**
//! （`e ∈ 检索集 ∪ {注入的那一枚}`），它默认拿到的是一枚**判定产出**的值。
//!
//! 与之对照：[`ResolutionConfidence`] / [`RequiredStrength`] 的里子也是私有的
//! ——而它们**连只读访问器都没有**：它们没有字段清单要断言，且「不能被分数构造」
//! 那一条正是靠私有里子 ＋ 不给 `From<f64>` 实现的（§4.4 第 2／3 条）。
//! 两处的差别写在这里，免得被读成同一件事：载荷是**纯数据**（读得出去没有代价），
//! 门槛是**判据的载体**（连读都不给，免得被读成一个数）。

use crate::Opaque;
use crate::candidate::{Candidate, EntityId};
use crate::explicit::{CandidateFacts, ExplicitConstraint, qualifying};

/// §4.1 的四态。四支都是**返回值，不是错误**（见文件头）。
///
/// # `ConfirmRequired` 与 `Ambiguous` 必须可区分
///
/// 前者是「有占优候选但门槛未过」，后者是「多个候选都合格」——**两支的判据不同源**。
/// 合并成一支会让**门槛失掉唯一的可观察后果**：门槛（§276）唯一能改变结果的地方
/// 就是「唯一合格候选那一格是 `Resolved` 还是 `ConfirmRequired`」。
/// `tests/resolver.rs::each_of_the_four_states_has_a_photo_and_the_state_is_asserted`
/// 各断言一枚，正是钉这条边界。
///
/// # 派生
///
/// `Debug` 是断言失败时必须打得出来、`Clone` 是随值传递。
/// **不派生 `PartialEq` / `Eq`**：`ConfirmRequired` 与 `Ambiguous` 里装的是
/// `Vec<Candidate>`，而 `Candidate` 没有 `PartialEq`（§272 只给了字段名，本层没有
/// 「两枚候选相等」这条判据，见 `src/candidate.rs`）。**故用例一律用
/// `matches!` 断言「是哪一枚」，不写 `assert_eq!` 比两个 `ResolutionResult`**
/// ——那正是本仓「不给出没有用例的表达力」的同一条：要 `PartialEq` 就得先有判据。
#[derive(Debug, Clone)]
pub enum ResolutionResult {
    /// §206「允许自动绑定」：唯一合格候选，且门槛已过。
    Resolved(Resolved),
    /// §206「第一候选明显，但不足以安全执行」：占优候选存在，但门槛未过。
    ConfirmRequired(ConfirmRequired),
    /// §206「多个候选仍然成立」：满足全部显式约束的候选多于一个。
    Ambiguous(Ambiguous),
    /// §206「当前没有可靠解析」：零个合格候选。
    Unknown(Unknown),
}

/// 解出了**恰好一个**合格候选，且门槛已过。
///
/// # 三个字段的分工（不写清就会被读成同一件事）
///
/// - [`entity`](Resolved::entity) 是判定输出的那一枚实体；
/// - [`satisfied`](Resolved::satisfied) 是**照录判定时用的那一组约束**——
///   不是重新推的。§4.2 的断言 D（唯一合格）要靠它可查：没有它，调用方只能自己
///   再构造一次约束集去核，那种核法是**拿自己的构造核自己的结果**；
/// - [`candidate_count`](Resolved::candidate_count) 是**判定输入的规模**——
///   它是**规模**（`usize`），**不是分数**。它与「已解出的结果携带不了分数」
///   （§4.4 第 1 条）不相抵：那一条禁的是 `retrieval_score` 一类**排序用的数**，
///   而这个数说的是「集合里有几个」，供 §4.2 的断言 A／D 核对。
///
/// 三者分开，是为了**不把判定输入混进判定输出**（`satisfied` 是输入那一侧的照录，
/// `candidate_count` 是输入的规模），同时让断言 A／D 不必自己再构造一次候选集。
#[derive(Debug, Clone)]
pub struct Resolved {
    /// 解出的那一枚实体。
    entity: EntityId,
    /// 判定时用的那一组显式约束，**照录**。
    satisfied: Vec<ExplicitConstraint>,
    /// 判定用的候选集规模（`usize`，不是分数）。
    candidate_count: usize,
}

impl Resolved {
    /// 解出的那一枚实体（§4.1 的 `entity`）。
    ///
    /// **只读访问器**：字段私有（crate 外构造不出本类型，见文件头），
    /// 而消费者读得到——读它的用例在 `tests/resolver.rs`。
    pub fn entity(&self) -> &EntityId {
        &self.entity
    }

    /// 判定时用的那一组显式约束，**照录**（§4.1 的 `satisfied`）。
    ///
    /// **只读访问器**，同上；读它的用例在 `tests/resolver.rs`。
    pub fn satisfied(&self) -> &[ExplicitConstraint] {
        &self.satisfied
    }

    /// 判定用的候选集**规模**（`usize`，不是分数；§4.1 的 `candidate_count`）。
    ///
    /// **只读访问器**，同上；读它的用例在 `tests/resolver.rs`（Task 5 的断言 A／D 也读它）。
    pub fn candidate_count(&self) -> usize {
        self.candidate_count
    }
}

/// 有一个**占优候选**，但**门槛未过**（§206 的「第一候选明显，但不足以安全执行」）。
///
/// # `surviving` 为什么是「全体」而不是空集／省略
///
/// 它是**判定输出**：与 `first` 一同满足全部显式约束的候选**全体**（含 `first`）。
/// 本层今天产生 `ConfirmRequired` 的那一格是「恰有一个合格候选 ＋ 门槛未过」，
/// 故这里的 `surviving` **长度恰是 1**。
/// **一处与设计原文的偏离照实记**：设计 §4.1 的字段清单在这个字段旁写了
/// 「**多于一**」（`design:398-400`），而那一句与本支的产生条件相抵——
/// 门槛未过这一格的前提就是**只有一个**候选合格（多于一个时判据换成
/// [`Ambiguous`]）。本层**不照抄那个括注**：一条绝对措辞要有对应的用例，
/// 而「多于一」在本支今天取不到照片（能取到的照片是相反的那一枚：
/// 门槛未过那一格里 `surviving.len() == 1`，`tests/resolver.rs` 断言了它）。
#[derive(Debug, Clone)]
pub struct ConfirmRequired {
    /// 那个占优候选的实体。
    first: EntityId,
    /// 满足全部显式约束的候选全体（含 `first`）。
    surviving: Vec<Candidate>,
    /// 判定用的候选集规模（`usize`，不是分数）。
    candidate_count: usize,
}

impl ConfirmRequired {
    /// 那个占优候选的实体（§4.1 的 `first`）。**只读访问器**（字段私有的理由见文件头）。
    pub fn first(&self) -> &EntityId {
        &self.first
    }

    /// 满足全部显式约束的候选全体（含 `first`；§4.1 的 `surviving`）。**只读访问器**。
    pub fn surviving(&self) -> &[Candidate] {
        &self.surviving
    }

    /// 判定用的候选集**规模**（`usize`，不是分数）。**只读访问器**。
    pub fn candidate_count(&self) -> usize {
        self.candidate_count
    }
}

/// 多个候选仍然成立：满足全部显式约束的候选**多于一个**。
#[derive(Debug, Clone)]
pub struct Ambiguous {
    /// 满足全部显式约束的候选**全体**，至少两个。
    surviving: Vec<Candidate>,
    /// 判定用的候选集规模（`usize`，不是分数）。
    candidate_count: usize,
}

impl Ambiguous {
    /// 满足全部显式约束的候选**全体**（§4.1 的 `surviving`）。**只读访问器**
    /// （字段私有的理由见文件头；Task 5 的断言 D 第（2）条读它，且**逐项**断言）。
    pub fn surviving(&self) -> &[Candidate] {
        &self.surviving
    }

    /// 判定用的候选集**规模**（`usize`，不是分数）。**只读访问器**。
    pub fn candidate_count(&self) -> usize {
        self.candidate_count
    }
}

/// 当前没有可靠解析：零个合格候选。
#[derive(Debug, Clone)]
pub struct Unknown {
    /// 零个合格候选的**哪一枚**（两枚的处置方向相反，见 [`UnknownReason`]）。
    reason: UnknownReason,
    /// 判定用的候选集规模（`usize`，不是分数）。它把两枚 `reason` 区分开：
    /// `NoCandidate` 必伴 0，`AllCandidatesRejected` 必伴一个非零的数。
    candidate_count: usize,
}

impl Unknown {
    /// 零个合格候选的**哪一枚**（§4.1 的 `reason`）。**只读访问器**（字段私有的理由见文件头）。
    pub fn reason(&self) -> &UnknownReason {
        &self.reason
    }

    /// 判定用的候选集**规模**（`usize`，不是分数）。**只读访问器**；
    /// 它把两枚 `reason` 分开：`NoCandidate` 必伴 0，`AllCandidatesRejected` 必伴一个非零的数。
    pub fn candidate_count(&self) -> usize {
        self.candidate_count
    }
}

/// 「为什么是 `Unknown`」。**两枚都必须是可区分的**——§4.2 的断言 B 与断言 C 各钉一枚。
///
/// # 混成一枚会怎样
///
/// 「候选集本来就没有」与「候选集非空但全部不合格」**处置方向相反**：
/// 前者要**扩大检索**（§201 的 `0 candidate → NIL / Expand Retrieval`，
/// `docs/spec/04-method.md:691-692`），后者要**问用户**（检索给了东西，是约束把它们
/// 全否掉了——再检索一次不会改变这件事）。混成一枚，这两条路就再也没有分支点。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnknownReason {
    /// 候选集**为空**（§4.2 的断言 C）。
    NoCandidate,
    /// 候选集**非空**，但全部违反某条显式约束（§4.2 的断言 B）。
    AllCandidatesRejected,
}

/// §275 的 `ResolutionConfidence`：**只决定是否允许自动执行**，
/// 不定义「实体为真的概率」（`docs/spec/05-normative.md:1428-1448`）。
///
/// # 里子是私有的，且**没有一条路能把它变成一个数**
///
/// 「没有读者能把它变成一个数」（§4.4 第 2 条，`design:741-745`）落在两处：
/// 方法集只有 [`meets`](ResolutionConfidence::meets)，且**没有** `as_probability` /
/// `to_f64` 这类读者、**没有** `impl From<ResolutionConfidence> for f64`、
/// **没有** `From<f64>` / `from_score` 一类的构造函数。
/// 照片：`tests/compile_fail/resolution_confidence_has_no_numeric_reader.rs` 与
/// `tests/compile_fail/resolution_confidence_cannot_be_built_from_a_score.rs`（各配一份实跑取值的 `.stderr`），
/// 正控制是 `tests/compile_pass/resolution_confidence_meets_is_usable.rs`。
///
/// # 取值从哪来：**调用方给**（这一点与设计的一句话相抵，照实记）
///
/// 设计 §4.4 第 3 条（`design:748`）写「`ResolutionConfidence` 的取值只能由**门槛判定**产出」。
/// **本层做不到那句话**：§275 的「校准后的」是一个**校准过程**（设计 §15.2 已把
/// 「它必须经过校准」记为**本层没有判据、也没有照片**的一条，`design:2262-2263`），
/// 而本层不取系统时钟、不做 I/O、也没有任何校准所需的事实——**能产的只有「校准的结果」本身**，
/// 即调用方带来的那一枚值。故本类型是一个**入参**（`resolve` 的入参之一），
/// 本层对它的全部处置就是 [`meets`](ResolutionConfidence::meets)。
/// **不是「本层偷懒」**：给它写一条「产出规则」就需要一个本层没有的校准过程，
/// 那是**发明一条判据**，而它与「没有判据」是两件事（设计 §15.2 第 2 条同形：
/// 「这不是没有照片，是没有判据」）。
///
/// # 它**不出现在** `Candidate` 与 `Resolved` 的字段里
///
/// §4.4 第 2 条的后半：两个数的禁令要靠**类型的位置**兑现。
/// 照片是对两份字段清单的穷尽解构（[`tests::resolution_confidence_never_appears_in_a_candidate_or_a_resolved`]，
/// 本文件的 `#[cfg(test)]` 里——字段私有后穷尽解构只有 crate 内做得到，见文件头）。
#[derive(Debug, Clone)]
pub struct ResolutionConfidence(Opaque);

impl ResolutionConfidence {
    /// 由调用方给出**已校准**的置信值。取值域规范未给（§16 第 6 条同形），
    /// 故里子取 [`Opaque`]——本层**不解释它、不归一、不换算**。
    ///
    /// **它不接受分数**：没有 `from_score` 一类，也没有 `From<f64>`
    /// （§4.4 第 3 条：两个数不同轴）。
    pub fn new(value: Opaque) -> Self {
        ResolutionConfidence(value)
    }

    /// §275／§276 的门槛判定：这一枚置信**是否达到**要求的强度。
    ///
    /// # 判据是等值——**这是本层在「序没给」下的选择，与 [`crate::satisfies`] 的 `Scope` 同形**
    ///
    /// §276（`docs/spec/05-normative.md:1450-1462`）给的是
    /// `required_resolution_strength = f(effect, cost, reversibility, privacy)`，
    /// 而设计 §4.5 写死**`f` 的形态与四个风险维之间的序与量纲规范都没给**
    /// （`design:763-771`）——故「越高越严」这条单调性在本层**没有判据**。
    /// 两枚值（已达的置信、要求的强度）都取 [`Opaque`]，本层**没有**它们之间的序。
    ///
    /// 于是只有两条可写的路：**造一个序**（无中生有，会把「高门槛」判成「低门槛」，
    /// 比没有更坏）或**取等值**（更严 ⇒ 合格更少 ⇒ 更容易落到 `ConfirmRequired`，
    /// 而它不会自动绑定）。**本层取等值**，理由与 §4.1.1 的 `Scope` 取等值完全同形
    /// （fail-closed 那一侧，`design:531-535`）：判据的严松方向不一样，
    /// 但「不发明一个没有规范出处的序」是同一条。
    /// **调用方给出两枚值时用同一套词汇**（给出「本动作要求的那一档」与「本次达到的那一档」），
    /// 同档即放行。
    ///
    /// 比较经 [`Opaque::as_str`]：调用点看得到自己在比那两个串，
    /// 而不是在比一个被本层背书过的「强弱」关系。
    ///
    /// **这条判据的照片**：`tests/resolver.rs` 的两条门槛用例（同档 ⟹ `Resolved`、
    /// 不同档 ⟹ `ConfirmRequired`）。**「门槛连续地参与排序」那种读法在本层不成立**
    /// ——上面那句「同档即放行」是本层明的判据，不是近似。
    pub fn meets(&self, required: &RequiredStrength) -> bool {
        self.0.as_str() == required.0.as_str()
    }
}

/// §276 的门槛：`required_resolution_strength = f(effect, cost, reversibility, privacy)`。
///
/// # 无 `Default`，且 [`resolve`] 不收 `Option<RequiredStrength>`
///
/// §4.5 把这条写死了（`design:763-773`）：一个「保守的基线」也无从写起
/// ——四维之间的序与量纲都没有，无中生有的序会把「高成本」判成「低风险」。
/// 故门槛是**必填入参**，且本层**不提供**任何把 `ResolutionRisk` 映射成门槛的基线
/// （那一条是 Task 6 的用例，`docs/superpowers/plans/2026-10-06-p4-semantic-layer.md:885-886`）。
///
/// **本类型没有 `Default`**：`Default` 会是一个**本层编出来的门槛**——
/// 上面那句「无从写起」正是对它的否决，而不是一句偏好。
/// **它也不派生 `Copy`**：里子是 `Opaque`（包一个 `String`），`Copy` 在类型上就不成立。
#[derive(Debug, Clone)]
pub struct RequiredStrength(Opaque);

impl RequiredStrength {
    /// 由调用方给出（§276 的 `f` 由调用方算，本层不发明 `f`）。
    pub fn new(value: Opaque) -> Self {
        RequiredStrength(value)
    }
}

/// Reference Resolver 的入口：把候选集与显式约束判成 §4.1 的四态之一。
///
/// # 工作集 = 检索集 ＋ binding 注入（至多一次）
///
/// `candidates` 是**检索集**：`CandidateSources` 交出的那一集，调用方给什么就是什么。
/// §4.6 的 `ExplicitBinding` 会在**过滤之前**把一个检索没给出的实体**注入**进来，
/// 而那是**唯一允许的注入点**（[`crate::binding::apply`]）——除它之外本层**没有第二个**
/// 能把实体放进判定的来源。这个式子就是**工作集**：
///
/// ```text
/// 工作集 = 检索集 ＋ binding 注入（至多一次）
/// ```
///
/// **本函数收 `injection`**（Task 6 给它入口；此前工作集 ≡ 检索集）。
/// **注入是调用方传参、不是由 `BoundTo` 约束自动发生**——理由与两条读法的分岔写在
/// `src/binding.rs` 的文件头。`injection` 为 `None` 时工作集 ≡ 检索集，
/// 故 §4.2 那组用例（它们一律传 `None`）的判据一字未变。
///
/// **§4.2 的断言 A 拿的不是这个工作集，而是「检索集 ∪ 注入的那一枚」这两个来源的并**
/// ——注入是 A 里**显式列出的第二个来源**，故 A 既不因注入而失效（注入后它仍只约束这两个来源），
/// 也不因注入而恒真（恒真的那个版本拿的是**工作集**）。
/// 两者的差别由 `tests/resolver.rs::assertion_a_injection_guard` 钉住：
/// 「解析器自造一个既不在检索集、也没有 binding 背书的实体」必须让 A 变红，
/// 而「A 拿工作集」那个版本对自造实体**永不响**（设计 §4.2 的订正，`design:616-635`）。
///
/// # 判定的输入怎么来（Task 6 之后）
///
/// 1. **检索集**：`candidates`，调用方给什么就是什么；
/// 2. **注入**：`injection` 是 `Option<(&ExplicitBinding, EntityKind)>`——§4.6 的
///    `ExplicitBinding` 与**被点名实体的种类**。为何种类要单独来（§277 的四个字段里没有它）、
///    以及「已在检索集里就缩、不在就追加」这两条路，逐条写在 [`crate::binding::apply`]。
///    本函数只把它的输出当**工作集**，不重复那一层判断；
/// 3. **过滤与门槛**：步骤见下。
///
/// # 判定顺序（每一格的依据都在上面）
///
/// 1. 用 [`qualifying`] 从**工作集**里取出**合格**的候选：
///    满足**全部**显式约束的那些；
/// 2. 零个 ⟹ [`Unknown`]，`reason` 看**工作集**空不空
///    （空 ⟹ [`UnknownReason::NoCandidate`]，非空 ⟹ [`UnknownReason::AllCandidatesRejected`]）；
/// 3. 一个 ⟹ 看门槛：过 ⟹ [`Resolved`]，不过 ⟹ [`ConfirmRequired`]；
/// 4. 多于一个 ⟹ [`Ambiguous`]。
///
/// **第 2 格的界线是「工作集」不是「检索集」**：注入之后工作集非空而检索集为空是可能的，
/// 那时 `reason` 该是 `AllCandidatesRejected`——工作集里确实有候选，是约束把它们否掉了。
/// §4.6 的照片 (b) 是这一格的一个实例（注入的那一枚与检索集那一枚都不合格 ⟹
/// `AllCandidatesRejected`），但它的**检索集非空**，故它不展示「检索集为空」那一支；
/// 「**检索集为空 ＋ 有注入**」这一格**今天没有照片**（本仓的注入用例检索集都非空）。
///
/// **第 4 格不看门槛**：`Ambiguous` 的判据是「多个合格」（§4.1 的表），
/// 与门槛无关——故门槛的**可观察后果只有第 3 格那一处**。这一句要写出来：
/// 否则「门槛参与每一格」是一个很自然的误读，而它会让下一个人以为
/// `Ambiguous` 的结果也随门槛变（`tests/resolver.rs` 的四态用例把这一格钉成
/// 「门槛变了、结果仍是 `Ambiguous`」）。
///
/// # §4.2 的四条断言：**不存在正确候选时不强行绑定**（这句绝对措辞的照片在下面逐条点名）
///
/// 「不强行绑定」不是一个可以写在函数里的判断，而是**四条断言的合取**
/// （设计 §4.2，`design:608-686`；规范原文是 §273 与 §200）。逐条与用例名对钉：
///
/// | 断言 | 判据 | 用例 |
/// |---|---|---|
/// | **A** | `Resolved(e)` 蕴含 `e ∈ 检索集 ∪ {注入的那一枚}`（**同一个 `EntityId` 值**，经 `as_str` 比；不是「像某个候选」） | `assertion_a_a_resolved_entity_is_a_member_of_the_retrieved_set_or_the_injected_one` |
/// | **A 的注入守卫** | 除那两个来源外**没有第三个**——自造实体必须让 A 变红 | `assertion_a_injection_guard` |
/// | **B** | 候选集非空 ∧ 全部候选都违反某个显式约束 ⟹ **不是** `Resolved` | `assertion_b_each_candidate_violating_a_different_constraint_is_not_resolved` |
/// | **C** | 候选集为空 ⟹ `Unknown { NoCandidate }` | `assertion_c_an_empty_candidate_set_is_unknown_with_no_candidate` |
/// | **D①** | `Resolved(e)` 蕴含 `e` **满足全部**显式约束（不是「在集合里」） | `assertion_d_1_the_qualified_candidate_is_selected_even_when_an_unqualified_one_is_present` |
/// | **D②** | 多个合格候选 ⟹ `Ambiguous` 且 `surviving` **恰是**合格的那几枚 | `assertion_d_2_multiple_qualified_candidates_give_ambiguous` |
/// | 守卫的另一侧 | 非空且**唯一**合格 ⟹ `Resolved`（钉住「永远返回 `Unknown`」那个方向） | `the_other_side_a_nonempty_set_with_exactly_one_qualified_candidate_is_resolved` |
///
/// **A／B／D 三条都在 fail-open 侧**（少了任何一条，一个错的实现仍会返回一个看起来正常的
/// `Resolved`，从返回值上看不出错），故三条都必须有照片；C 在 fail-closed 侧，一条即可。
/// **D 的两条照片不是重复**：D① 钉「合格」，D② 钉「唯一」——前者在 `{合格一枚，不合格一枚}`
/// 上取照片（A 在这一格**不响**：绑到不合格的那一枚 A 仍成立），后者在 `{合格两枚，不合格一枚}`
/// 上取（`Ambiguous` 只被断言「是哪一态」时，第三枚**不承重**）。
///
/// **每条夹具都含至少一条非空显式约束**：约束集为空时每个候选都平凡地合格
/// （[`qualifying`] 的合取在空集上取中性元），那些用例会退化成「候选都在集合里」——**恒真**。
///
/// # 返回类型不是 `Result`
///
/// §4.1 写死（`design:433-437`）：§206 的 `abstain` 已把「没有可靠解析」收进
/// `Unknown`，而本层没有第二种失败（输入全是值、不做 I/O、不落库）。
/// 照片：本 crate 对 [`ResolutionResult`] 的用例里**没有一条写 `unwrap()`**——
/// `tests/resolver.rs` 与本文件的 `#[cfg(test)]` 各有一批，都直接对
/// [`ResolutionResult`] 断言（若某天给它套上 `Result`，那些断言全部编不过）。
///
/// # 今天没有真实的调用方
///
/// [`crate::CandidateSources`] 交出的候选集要经过 [`crate::binding::apply`] 的注入
/// 才是**工作集**（§4.6），而真实候选来源今天**零个**（设计 §16 第 5 条）。
/// 故本函数今天只被本 crate 的用例走到，钉的是**分支机制**
/// （与设计 §15.2 第 3 条同一体：能拍的只有机制）。**本条不是「四态已接通」**：
/// 上面那张表里的七条用例（Task 5）钉住了 §4.2 的四条断言与两侧守卫，
/// 而**注入点本身**归 Task 6 的 [`crate::binding::apply`]（它的三条路与照片在 `src/binding.rs`）。
/// 两者不是一件事：四条断言核的是**判定产出的值**，注入点决定**判定看哪一集**。
pub fn resolve(
    candidates: &[(Candidate, CandidateFacts)],
    injection: Option<(&crate::ExplicitBinding, crate::EntityKind)>,
    constraints: &[ExplicitConstraint],
    confidence: &ResolutionConfidence,
    required: &RequiredStrength,
) -> ResolutionResult {
    // 工作集 = 检索集 ＋ binding 注入（至多一次；见文档与 `src/binding.rs`）。
    // 注入点是**唯一**能把实体放进判定的地方，而注入**只来自调用方传进来的 binding**。
    let working = crate::binding::apply(candidates, injection);
    // 这个数就是断言 A 的第二个读数：解出的结果里那个数**不得超过**工作集的应有大小
    // （`|检索集| + [有注入且不在检索集里]`）。它数的是**判定输入**的规模，不是分数。
    let candidate_count = working.len();
    // 过滤施加在**工作集**上：由 `ExplicitBinding` 注入的那一枚**同走这一道**
    // ——binding 只提高**检索**这一侧，不豁免 `ExplicitConstraint`（§4.6 第 3 条）。
    let surviving = qualifying(&working, constraints);

    match surviving.len() {
        0 => {
            // 两枚 `reason` 的界线是**输入候选集**空不空，不是合格集空不空
            // （合格集在这里必然是空的——这正是本支的条件）。
            let reason = if candidate_count == 0 {
                UnknownReason::NoCandidate
            } else {
                UnknownReason::AllCandidatesRejected
            };
            ResolutionResult::Unknown(Unknown {
                reason,
                candidate_count,
            })
        }
        1 => {
            let only = surviving.into_iter().next().expect(
                "本支的条件就是「恰有一个合格候选」，故这里必须取得到那一枚（不是本层发的错误）",
            );
            if confidence.meets(required) {
                ResolutionResult::Resolved(Resolved {
                    entity: only.entity_id,
                    satisfied: constraints.to_vec(),
                    candidate_count,
                })
            } else {
                ResolutionResult::ConfirmRequired(ConfirmRequired {
                    // `entity_id` 先克隆一份：`only` 整个要进 `surviving`，
                    // 而 `EntityId` 不是 `Copy`（它包一个 `String`）。
                    first: only.entity_id.clone(),
                    surviving: vec![only],
                    candidate_count,
                })
            }
        }
        // 「多于一个」——`Ambiguous` 只要求「至少两个」，不要求恰好两个，
        // 故这里不比较具体数目（比较会让「三个合格」落进一条本层没写过的分支）。
        _ => ResolutionResult::Ambiguous(Ambiguous {
            surviving,
            candidate_count,
        }),
    }
}

// ============================================================================
// 下面这个 `#[cfg(test)]` 模块里的三条用例**原先住在 `tests/resolver.rs`**，
// 本轮（评审裁决一）随「四个载荷的字段转私有」搬进来：它们的机制是
// **对四支逐支穷尽解构、不带 `..`**，而穷尽解构要在字面量里命名字段——
// 字段私有后只有 crate 内做得到（本仓已有「照片住在 `src` 的 `#[cfg(test)]` 里」的体例）。
// 用例名、断言、红条件都**一字未改**，只调了跨文件的引用路径。
// ============================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use crate::candidate::{Candidate, EntityId};
    use crate::explicit::{CandidateFacts, EntityKind, ResolutionScope};

    /// 夹具：一枚候选。
    fn cand(id: &str, score: Option<f64>) -> Candidate {
        Candidate {
            entity_id: EntityId::from_str(id),
            evidence: Vec::new(),
            retrieval_score: score,
        }
    }

    /// 夹具：一枚候选的判定事实。
    fn facts(scope: ResolutionScope, kind: EntityKind) -> CandidateFacts {
        CandidateFacts { scope, kind }
    }

    /// 夹具：一枚「已达」的置信值（§275；本层不解释它的取值域，见本文件）。
    fn conf(level: &str) -> ResolutionConfidence {
        ResolutionConfidence::new(Opaque::from_str(level))
    }

    /// 夹具：一枚「要求」的强度（§276 的 `required_resolution_strength`）。
    fn req(level: &str) -> RequiredStrength {
        RequiredStrength::new(Opaque::from_str(level))
    }

    /// 把一批候选的实体串按**输入次序**取出来。
    fn ids(cs: &[Candidate]) -> Vec<&str> {
        cs.iter().map(|c| c.entity_id.as_str()).collect()
    }

    /// 把一条约束摊成一个串——**不带 `..` 的穷尽解构**。
    ///
    /// 用途：`Resolved.satisfied` 是「照录判定时用的那一组约束」，而
    /// `ExplicitConstraint` **没有** `PartialEq`（`BoundTo` 里是 `EntityId`，
    /// 而它没有相等判据——见 `src/explicit.rs`），故这里逐枚摊开再比。
    /// 顺带它自己也是一处「三枚穷尽、加一枚即编译不过」的解构。
    fn describe(k: &ExplicitConstraint) -> String {
        match k {
            ExplicitConstraint::Scope(s) => format!("scope:{s:?}"),
            ExplicitConstraint::EntityKind(t) => format!("kind:{t:?}"),
            ExplicitConstraint::BoundTo(e) => format!("bound_to:{}", e.as_str()),
        }
    }

    /// `describe` 对一批约束的投影（夹具函数，见它的文档）。
    fn described(ks: &[ExplicitConstraint]) -> Vec<String> {
        ks.iter().map(describe).collect()
    }

    /// 四支的**字段清单逐项**：机制 = 对四支**逐支穷尽解构、不带 `..`**。
    ///
    /// 解构的是四个载荷类型**自己的字段**（§4.1 的字段清单）：
    /// `Resolved { entity, satisfied, candidate_count }` /
    /// `ConfirmRequired { first, surviving, candidate_count }` /
    /// `Ambiguous { surviving, candidate_count }` /
    /// `Unknown { reason, candidate_count }`。
    ///
    /// 任一支加字段、少字段、改名 ⟹ **编译不过**（加字段 `E0027`、写不存在的字段 `E0026`、
    /// 改名 `E0026`／`E0609`）。
    ///
    /// **明写**：这是**编译期照片，运行期无照片**——四个载荷是纯数据声明，
    /// 运行期能断言的只有「字段装得进、读得回」，而那与「字段清单**恰好**是这几个」不是一回事。
    /// 加字段这件事**没有变红的观测**，只有一条编译错误，而按本仓纪律「编译不过」不算红。
    ///
    /// # 为什么这条用例住在 `src/` 里（而不是 `tests/resolver.rs`）
    ///
    /// 它的机制是**穷尽解构**——字段私有（本轮裁决一）之后，只有在 crate 内才写得出
    /// 不带 `..` 的字面量解构。见本文件的文件头。
    ///
    /// # `candidate_count` **不是分数**（免得这一条被读成与 §4.4 第 1 条相抵）
    ///
    /// 它是**规模**（`usize`，§4.2 的断言 A／D 拿它核「集合里有几个」）；
    /// §4.4 第 1 条禁的是 `retrieval_score` 一类**排序用的数**。两者的区别由
    /// `resolution_result_carries_no_score` 与 `resolution_confidence_never_appears_in_a_candidate_or_a_resolved`
    /// 各自断言（那里逐枚断言语义，这里只断字段清单）。
    #[test]
    fn every_arm_carries_exactly_its_declared_fields() {
        let k = [ExplicitConstraint::Scope(ResolutionScope::Project)];
        let one = vec![(
            cand("e-one", None),
            facts(ResolutionScope::Project, EntityKind::Tool),
        )];
        let two = vec![
            (
                cand("e-1", None),
                facts(ResolutionScope::Project, EntityKind::Tool),
            ),
            (
                cand("e-2", None),
                facts(ResolutionScope::Project, EntityKind::Skill),
            ),
        ];
        let empty: Vec<(Candidate, CandidateFacts)> = Vec::new();

        // —— `Resolved`：**不带 `..`** ——
        match resolve(&one, None, &k, &conf("l"), &req("l")) {
            ResolutionResult::Resolved(Resolved {
                entity,
                satisfied,
                candidate_count,
            }) => {
                assert_eq!(entity.as_str(), "e-one");
                assert_eq!(satisfied.len(), 1);
                assert_eq!(candidate_count, 1);
            }
            other => panic!("夹具前提：这一份输入该落 `Resolved`，实得 {other:?}"),
        }

        // —— `ConfirmRequired`：**不带 `..`** ——
        match resolve(&one, None, &k, &conf("l"), &req("other")) {
            ResolutionResult::ConfirmRequired(ConfirmRequired {
                first,
                surviving,
                candidate_count,
            }) => {
                assert_eq!(first.as_str(), "e-one");
                assert_eq!(ids(&surviving), vec!["e-one"]);
                assert_eq!(candidate_count, 1);
            }
            other => panic!("夹具前提：这一份输入该落 `ConfirmRequired`，实得 {other:?}"),
        }

        // —— `Ambiguous`：**不带 `..`** ——
        match resolve(&two, None, &k, &conf("l"), &req("l")) {
            ResolutionResult::Ambiguous(Ambiguous {
                surviving,
                candidate_count,
            }) => {
                assert_eq!(ids(&surviving), vec!["e-1", "e-2"]);
                assert_eq!(candidate_count, 2);
            }
            other => panic!("夹具前提：这一份输入该落 `Ambiguous`，实得 {other:?}"),
        }

        // —— `Unknown`：**不带 `..`** ——
        match resolve(&empty, None, &k, &conf("l"), &req("l")) {
            ResolutionResult::Unknown(Unknown {
                reason,
                candidate_count,
            }) => {
                assert_eq!(reason, UnknownReason::NoCandidate);
                assert_eq!(candidate_count, 0);
            }
            other => panic!("夹具前提：这一份输入该落 `Unknown`，实得 {other:?}"),
        }
    }

    /// §4.4 第 1 条：**已解出的结果携带不了分数**。
    ///
    /// # 机制（与上一条共用，为什么还要单独写一条）
    ///
    /// 判据仍然是对 `Resolved` 做**不带 `..` 的穷尽解构**——它的字段清单里**没有任何
    /// 分数字段**（`retrieval_score` 只出现在 [`Candidate`] 上，见 §272 与 §4.4）。
    /// 与上一条分开写，是因为它是**另一条要求**（§14 的表把它与「字段清单逐项」并列记为两条），
    /// 且这里还多断一件事：`candidate_count` 是**规模**、不是分数——它是 `usize`，
    /// 数的是「集合里有几个」。**这一句要写出来**，否则它会与「结果携带不了分数」读起来相抵。
    ///
    /// # 分数的去处：它是**判定输入**的一部分，不进判定输出
    ///
    /// 下面那份输入里的候选**带着分数**（`Some(0.97)`），而结果的三个字段里一个数都没有——
    /// 实体、约束、规模。**本条不主张「分数不参与任何判定」**（§272 说它 MAY 用于排序，
    /// 那是消费者的事；本层今天也没有排序），它主张的是**结果的形状**：
    /// 拿到一枚 `Resolved` 的调用方**在类型上取不到**那个分数。
    ///
    /// 红条件（**移除档**）：给 `Resolved` 加一个 `confidence` 或 `retrieval_score` 字段
    /// ⟹ 上一条与本条的解构都**编译不过**（`E0027`）；按本仓纪律那是**编译期照片**，
    /// 故这一枚变异体的性质照实记：**没有运行期红**。见报告 §变异轮。
    #[test]
    fn resolution_result_carries_no_score() {
        let k = [ExplicitConstraint::Scope(ResolutionScope::Project)];
        let cs = vec![(
            cand("e-scored", Some(0.97)),
            facts(ResolutionScope::Project, EntityKind::Tool),
        )];

        match resolve(&cs, None, &k, &conf("high"), &req("high")) {
            // **不带 `..`**：这一行就是本用例的机制（清单里没有第四个字段）。
            ResolutionResult::Resolved(Resolved {
                entity,
                satisfied,
                candidate_count,
            }) => {
                assert_eq!(entity.as_str(), "e-scored", "解出的还是那一枚带分数的候选");
                assert_eq!(satisfied.len(), 1, "照录的是那一条约束");
                // `candidate_count` 是**规模**：它是「集合里有几个」，不是任何分数。
                assert_eq!(candidate_count, 1usize, "规模是 usize，数的是候选个数");
            }
            other => panic!("该落 `Resolved`，实得 {other:?}"),
        }
    }

    /// §4.4 第 2 条的后半：`ResolutionConfidence` **不出现在** `Candidate` 与 `Resolved`
    /// 的字段里（**两侧都断**）。
    ///
    /// # 机制
    ///
    /// 两处**不带 `..` 的穷尽解构**：
    /// - [`Candidate`]（Task 3 的三字段，§272 照录）；
    /// - [`Resolved`]（本 task 的三字段，§4.1）。
    ///
    /// 把 `ResolutionConfidence` 塞进任何一侧 ⟹ 那一侧的解构**编译不过**（`E0027`）。
    /// **照实记**：这是编译期照片，运行期无红。
    ///
    /// 红条件（**取反档**）：给 `Candidate` 加 `pub confidence: ResolutionConfidence`
    /// ⟹ 本用例第一段编不过；**同一枚变异体还会让 Task 3 的**
    /// `tests/candidate.rs::the_candidate_records_exactly_the_three_fields_of_272` **先红**
    /// （两处钉同一张清单，那是设计有意的重复：一处钉 §272 的形、一处钉 §4.4 的位置）。
    #[test]
    fn resolution_confidence_never_appears_in_a_candidate_or_a_resolved() {
        // —— `Candidate` 那一侧：**不带 `..`** ——
        let c = cand("e-272", Some(0.5));
        let Candidate {
            entity_id,
            evidence,
            retrieval_score,
        } = c;
        assert_eq!(entity_id.as_str(), "e-272");
        assert!(evidence.is_empty());
        assert_eq!(retrieval_score, Some(0.5));

        // —— `Resolved` 那一侧：**不带 `..`** ——
        let k = [ExplicitConstraint::Scope(ResolutionScope::Project)];
        let cs = vec![(
            cand("e-res", None),
            facts(ResolutionScope::Project, EntityKind::Tool),
        )];
        match resolve(&cs, None, &k, &conf("high"), &req("high")) {
            ResolutionResult::Resolved(Resolved {
                entity,
                satisfied,
                candidate_count,
            }) => {
                assert_eq!(entity.as_str(), "e-res");
                assert_eq!(described(&satisfied), vec!["scope:Project".to_owned()]);
                assert_eq!(candidate_count, 1);
            }
            other => panic!("该落 `Resolved`，实得 {other:?}"),
        }
    }
}
