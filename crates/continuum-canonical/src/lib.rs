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

mod error;
mod surface;

pub use error::CanonicalError;
pub use surface::{Mention, MentionKind, Surface, detect, normalize};

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
