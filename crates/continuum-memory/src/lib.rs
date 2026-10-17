//! P6C 记忆与学习（设计 `docs/superpowers/specs/2026-10-17-p6c-memory-and-learning-design.md`）。
//!
//! 本 crate 是**第 6 层认知层**成员，只依赖 P0 的 `continuum-persist`（落库）。
//! 模块方向见设计 §3.4：`source.rs` 不引用 claim 侧类型；`claim.rs` 使用 source；
//! `correction.rs`／`synthesis.rs` 使用 source＋claim；`retrieval.rs` 只读 claim。
//! 「Memory Source Graph 入度为零」落在同 crate 的模块方向上，不靠第二个 crate。

mod claim;
mod error;
mod persist;
mod source;

pub use claim::{Confidence, MemoryClaim, MemoryClaimId, MemoryScope, Validity};
pub use error::MemoryError;
pub use persist::p6c_memory_migrations;
pub use source::{
    MemorySource, MemorySourceId, SourceKind, insert_source, load_source, load_sources,
};
