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
//! # 四个载荷类型的字段是 `pub`，且它们**可以**在 crate 外被构造
//!
//! 这一点照实写，免得被读成「四态只能由 `resolve` 产出」：本文件**没有**把
//! `Resolved` / `ConfirmRequired` / `Ambiguous` / `Unknown` 做成不可外部构造的句柄。
//! 理由是**字段清单那张照片**：§4.1 要求「四支各带哪些字段」逐项可断言，
//! 而可断言的机制是对四支**逐支穷尽解构、不带 `..`**（`tests/resolver.rs`）——
//! **穷尽解构要在 crate 外命名字段，字段就必须是 `pub`**。
//! 私有字段与那张照片不可兼得，本层取照片。
//! **代价照实写**：于是「一个 `ResolutionResult` 是不是 `resolve` 产出的」在本层
//! **没有类型判据**；本层今天也没有任何消费者会依赖那件事（第一个消费者是 Task 5 的四条断言，
//! 它核的是**值**：`e ∈ 检索集 ∪ {注入的那一枚}`，不是「这个值从哪来」）。
//!
//! 与之对照：`ResolutionConfidence` / `RequiredStrength` 的里子是**私有**的
//! ——它们没有字段清单要断言，而「不能被分数构造」那一条正是靠私有里子 ＋
//! 不给 `From<f64>` 实现的（§4.4 第 2／3 条）。

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
    pub entity: EntityId,
    /// 判定时用的那一组显式约束，**照录**。
    pub satisfied: Vec<ExplicitConstraint>,
    /// 判定用的候选集规模（`usize`，不是分数）。
    pub candidate_count: usize,
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
    pub first: EntityId,
    /// 满足全部显式约束的候选全体（含 `first`）。
    pub surviving: Vec<Candidate>,
    /// 判定用的候选集规模（`usize`，不是分数）。
    pub candidate_count: usize,
}

/// 多个候选仍然成立：满足全部显式约束的候选**多于一个**。
#[derive(Debug, Clone)]
pub struct Ambiguous {
    /// 满足全部显式约束的候选**全体**，至少两个。
    pub surviving: Vec<Candidate>,
    /// 判定用的候选集规模（`usize`，不是分数）。
    pub candidate_count: usize,
}

/// 当前没有可靠解析：零个合格候选。
#[derive(Debug, Clone)]
pub struct Unknown {
    /// 零个合格候选的**哪一枚**（两枚的处置方向相反，见 [`UnknownReason`]）。
    pub reason: UnknownReason,
    /// 判定用的候选集规模（`usize`，不是分数）。它把两枚 `reason` 区分开：
    /// `NoCandidate` 必伴 0，`AllCandidatesRejected` 必伴一个非零的数。
    pub candidate_count: usize,
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
/// 照片是对两份字段清单的穷尽解构（`tests/resolver.rs` 的
/// `resolution_confidence_never_appears_in_a_candidate_or_a_resolved`）。
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
/// # 判定顺序（每一格的依据都在上面）
///
/// 1. 用 [`qualifying`] 取出**合格**的候选：
///    满足**全部**显式约束的那些；
/// 2. 零个 ⟹ [`Unknown`]，`reason` 看**输入候选集**空不空
///    （空 ⟹ [`UnknownReason::NoCandidate`]，非空 ⟹ [`UnknownReason::AllCandidatesRejected`]）；
/// 3. 一个 ⟹ 看门槛：过 ⟹ [`Resolved`]，不过 ⟹ [`ConfirmRequired`]；
/// 4. 多于一个 ⟹ [`Ambiguous`]。
///
/// **第 4 格不看门槛**：`Ambiguous` 的判据是「多个合格」（§4.1 的表），
/// 与门槛无关——故门槛的**可观察后果只有第 3 格那一处**。这一句要写出来：
/// 否则「门槛参与每一格」是一个很自然的误读，而它会让下一个人以为
/// `Ambiguous` 的结果也随门槛变（`tests/resolver.rs` 的四态用例把这一格钉成
/// 「门槛变了、结果仍是 `Ambiguous`」）。
///
/// # 返回类型不是 `Result`
///
/// §4.1 写死（`design:433-437`）：§206 的 `abstain` 已把「没有可靠解析」收进
/// `Unknown`，而本层没有第二种失败（输入全是值、不做 I/O、不落库）。
/// 照片：`tests/resolver.rs` 里没有一条用例写 `unwrap()`，四条都直接对
/// [`ResolutionResult`] 断言（若某天给它套上 `Result`，那些断言全部编不过）。
///
/// # 今天没有真实的调用方
///
/// [`crate::CandidateSources`] 交出的候选集要经过 Task 6 的 `ExplicitBinding` 注入
/// 才是**工作集**（§4.6），而真实候选来源今天**零个**（设计 §16 第 5 条）。
/// 故本函数今天只被本 crate 的用例走到，钉的是**分支机制**
/// （与设计 §15.2 第 3 条同一体：能拍的只有机制）。**本条不是「四态已接通」**：
/// 接线归 Task 5 的四条断言与 Task 6 的注入点。
pub fn resolve(
    candidates: &[(Candidate, CandidateFacts)],
    constraints: &[ExplicitConstraint],
    confidence: &ResolutionConfidence,
    required: &RequiredStrength,
) -> ResolutionResult {
    let candidate_count = candidates.len();
    let surviving = qualifying(candidates, constraints);

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
