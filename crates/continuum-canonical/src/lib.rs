//! 输入规范化前端与引用解析（P4 语义层；设计
//! `docs/superpowers/specs/2026-10-06-p4-semantic-layer-design.md`）。
//!
//! 本 crate 是 P4 的**语义层**成员，落《工程》§2.1 组件表里 Input Canonicalization Frontend
//! 的七个组件（`§270`）与 Reference Resolver ＋ Alias 存储（`§271`–`§278`）。
//!
//! # 走到哪为止
//!
//! Task 1 建骨架：crate 文档、workspace 登记，并落 `§270` 的 **Surface Normalizer**
//! （[`normalize`] / [`Surface`]，`surface` 模块）与 **Mention Detector**
//! （[`detect`] / [`Mention`] / [`MentionKind`]），以 `tests/surface.rs` 的五条用例钉住
//! 四步折叠、不纠错、`span` 是字节偏移、零命中 fail-closed、`kind` 不透明五件事。
//!
//! Task 2 落 `§3.2` 的**两支互斥产物**与 `§3.1` 的最后两个组件：
//! [`Canonicalized`] / [`CanonicalRepresentation`] / [`UnresolvedMention`]（`canonical` 模块）、
//! Ambiguity Manager [`finish`]（`ambiguity` 模块，含四态 [`ResolutionResult`] 的**前向声明**
//! ——该类型的载荷按设计 §4.1 由 Task 4 补，理由写在那个模块的文件头），
//! 以及 Temporal Resolver [`resolve_temporal`] / [`Now`] / [`TemporalBinding`]（`temporal` 模块）。
//! 以 `tests/canonical.rs` 的七条用例钉住：全解落 `Canonical`、三种未解态各落 `NeedsResolution`
//! 且列表恰是那一枚、两支互为补集、四类脏输入各落一支、`Now` 是入参。
//!
//! Task 3 落 §272／§4.3 的**候选面**：[`EntityId`] / [`Candidate`] / [`CandidateSource`] /
//! [`CandidateSources`]（`candidate` 模块），以 `tests/candidate.rs` 的四条用例钉住
//! `EntityId` 的往返与空串处置、候选集不塌成一枚、`Candidate` 的三字段、分数的可选性；
//! 并以 `tests/type_level.rs` 驱动的 `tests/compile_fail/` 三份样例钉住
//! 「crate 外写不出 `EntityId` 的字面量构造」「`CandidateSource` 上没有 `pick_one` 这个口子」
//! 「crate 外拼不出 `CanonicalRepresentation`」（最后一条是 Task 2 转来的 P-2-1）。
//!
//! Task 4 落 §4.1.1 的**显式约束面**（[`ExplicitConstraint`] / [`EntityKind`] /
//! [`ResolutionScope`] / [`CandidateFacts`] / [`CandidateSet`] / [`satisfies`] /
//! [`qualifying`]，`explicit` 模块）、§278 四值与 §274 九值的映射
//! （[`AliasScope`] / [`alias_scope_to_resolution_scope`] / [`resolution_scope_to_alias_scope`]，
//! `alias` 模块——**这个文件是 Task 7 的家，本 task 只提前落四枚词汇那一半**，
//! 那处计划次序缺口写在 `alias.rs` 的文件头）、§4.1 的四态与 §4.4 的两个数不同轴
//! （[`ResolutionResult`] 与四枚载荷 / [`ResolutionConfidence`] / [`RequiredStrength`] /
//! [`resolve`]，`resolver` 模块）。**`ResolutionResult` 的类型本体从 `ambiguity` 模块搬到了
//! `resolver`**——Task 2 落的是「是哪一态」那一层，载荷按设计由本 task 补。
//! 以 `tests/explicit.rs` 与 `tests/resolver.rs` 钉住上面那些判据，
//! 以 `tests/type_level.rs` 追加的编译失败／通过样例钉住
//! 「`ResolutionConfidence` 没有能把值变成数的读者」「它不能由分数构造」。
//!
//! 本段是**逐 task 更新的进度注记**，不是对代码性质的可跑断言，故没有用例钉它
//! ——与 `continuum-node` 的 `lib.rs` 同一体例：它的「走到哪为止」也由各 task 改写。
//!
//! # 边
//!
//! 设计 §2.2 给本 crate 定的入边只有一条：`continuum-canonical ← continuum-persist`
//! （`alias` 表经 `Tx` 读写，§4.6），**由用它的那个 task 登记**。
//! 本 task 对 workspace 内其它 crate 的**引用为零**，故 `crates/continuum-runtime/tests/
//! dependency_direction.rs` 的 `ALLOWED` 里本 crate 的允许集是**空条目**——
//! 它不能省（那张表要求每个 workspace 成员都在里面），只是今天还没有边。
//! 反向的 `continuum-semantics ← continuum-canonical` 是**读取方**的边，登记在那一侧。

mod alias;
mod ambiguity;
mod canonical;
mod candidate;
mod error;
mod explicit;
mod surface;
mod temporal;
// **`resolver` 是 `pub mod`**（其余模块私有 ＋ 根重导出）：Task 4 的 Interfaces 逐字把入口
// 写作 `resolver::resolve`（`docs/superpowers/plans/2026-10-06-p4-semantic-layer.md:636`），
// 故那个模块本身可从 crate 外按名字取到；其类型仍逐个在根上重导出，
// 使 `continuum_canonical::ResolutionResult` 与 `continuum_canonical::resolver::ResolutionResult`
// 是**同一个**类型（不是两份）。
pub mod resolver;

pub use alias::{AliasScope, alias_scope_to_resolution_scope, resolution_scope_to_alias_scope};
pub use ambiguity::finish;
pub use canonical::{CanonicalRepresentation, Canonicalized, UnresolvedMention};
pub use candidate::{Candidate, CandidateSource, CandidateSources, EntityId};
pub use error::CanonicalError;
pub use explicit::{
    CandidateFacts, CandidateSet, EntityKind, ExplicitConstraint, ResolutionScope, qualifying,
    satisfies,
};
pub use resolver::{
    Ambiguous, ConfirmRequired, RequiredStrength, ResolutionConfidence, ResolutionResult, Resolved,
    Unknown, UnknownReason, resolve,
};
pub use surface::{Mention, MentionKind, Surface, detect, normalize};
pub use temporal::{Now, TemporalBinding, resolve_temporal};

/// **不透明值**：一个包 `String` 的 newtype。
///
/// 设计里凡写 `Opaque` 的地方都取它——`completion_predicate`、`Priority`、
/// `ResolutionRisk` 的四个字段、`TaskContract` 的七个字段、`PlanFootprint` 的四个字段……
/// **本计划把它的定义收在这一处**：`continuum-canonical` 定义，
/// `continuum-semantics` 经**既有的** `← continuum-canonical` 边取它，
/// **两个 crate 都不为此新增任何边**（`continuum-budget` 不用它）。
///
/// # 它是占位，不是值域
///
/// 本类型**不提供**语义判定（无 `PartialEq`／`Eq`）、**不提供**排序（无 `Ord`／`PartialOrd`）、
/// **不提供**解释。理由：设计对它的每一处用法都写明「规范只给字段名、未给取值域」，
/// 故本层的处置是**保存事实**，不是**发明判据**——与 [`MentionKind`] 同一条理由
/// （设计 §16 第 3 条）。
///
/// **`Debug` 与 `Clone` 不是语义能力**：前者是断言失败时必须打得出来，
/// 后者是结构体要能装进 `Vec` 并随值传递（设计 §1.2 的「以值进入」）。
///
/// # 本类型今天没有取值域约束，也没有消费者
///
/// 本 task 只定义它、不用它。第一个用它的是 `continuum-semantics` 的
/// Intent／Contract／Plan 三个 task。
#[derive(Debug, Clone)]
pub struct Opaque(String);

impl Opaque {
    /// 取里子串。**不改写、不裁剪**。
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 从任意串构造。**无校验**——本类型没有「合法取值」这个概念，
    /// 故它不返回 `Result`（一个恒返回 `Ok` 的 `Result` 会把「可能失败」写成谎话）。
    pub fn from_str(s: &str) -> Self {
        Opaque(s.to_owned())
    }
}
