//! 研究领域算子（设计 `docs/superpowers/specs/2026-10-09-p5d-research-domain-operators-design.md`）。
//!
//! 本 crate 是 P5d 的交付物：把规范 §330 的七步研究链（`Question` → `Search` →
//! `SourceSet` → `EvidenceExtraction` → `ContradictionCheck` → `Synthesis` →
//! `CitationVerification`）落成七枚可注册的 `Operator`。`pub` 面最终恰三枚项——
//! `all_operators()`（本 task 交付）、`register_research_operators()`（Task 2）与
//! `bind_research_methods()`（Task 3）；后两枚由各自的 task 增量加。
//!
//! **本块零类型、零判定、零迁移**（设计 §2.2）：所有类型来自 `continuum-operator`
//! 与 `continuum-artifact`，本 crate 不新造任何类型、不新加任何枚举变体。

pub mod operators;

pub use operators::all_operators;
