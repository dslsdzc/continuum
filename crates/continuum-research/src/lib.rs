//! 研究领域算子（设计 `docs/superpowers/specs/2026-10-09-p5d-research-domain-operators-design.md`）。
//!
//! 本 crate 是 P5d 的交付物：把规范 §330 的七步研究链（`Question` → `Search` →
//! `SourceSet` → `EvidenceExtraction` → `ContradictionCheck` → `Synthesis` →
//! `CitationVerification`）落成七枚可注册的 `Operator`。
//!
//! **本 crate 只有三枚 `pub` 项，逐枚列在此处**：
//! 1. `all_operators()`（Task 1）——本领域七枚算子的清单；
//! 2. `register_research_operators()`（Task 2）——「先核后写」地把一批算子注册进注册表；
//! 3. `bind_research_methods()`（Task 3）——把 `research/` 四条方法绑到它们的 `realized_by`，由该 task 增量加。
//!
//! **本块零类型、零判定、零迁移**（设计 §2.2）：所有类型来自 `continuum-operator`
//! 与 `continuum-artifact`，本 crate 不新造任何类型、不新加任何枚举变体。

pub mod operators;
pub mod register;

pub use operators::all_operators;
pub use register::register_research_operators;
