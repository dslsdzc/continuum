//! 计算节点注册与节点放置（P3 子项目 E；设计
//! `docs/superpowers/specs/2026-10-06-p3e-compute-placement-design.md`）。
//!
//! 本 crate 是 P3 的**资源层**，落《工程》§4.1 组件表的两行——Compute Node 注册
//! （`§287`–`§291`）与节点放置（`§291` `§243` `§93`）。
//!
//! # 走到哪为止
//!
//! Task 1 建骨架：crate 文档与 workspace 登记。
//! Task 2 补 §287 的 [`ComputeNode`] 与它的三个取值类型 [`ComputeNodeId`] / [`NodeClass`] /
//! [`NodeTrust`]（[`node`] 模块），并以 `tests/type_level.rs` ＋
//! `tests/compile_fail/compute_node_fields_are_private.rs` 钉住「crate 外写不出 `ComputeNode`
//! 的字段字面量」这条不可表达性。
//! `src/registry.rs`、`src/placement.rs`、`src/error.rs` 三个模块与它们的导出面由后续
//! task 各自登记自己那几行。
//!
//! 本段是**逐 task 更新的进度注记**，不是对代码性质的可跑断言，故没有用例钉它
//! ——与 `continuum-model-registry` 的 `lib.rs` 同一体例：它的「走到哪为止」也由各 task 改写。
//!
//! # 边
//!
//! 设计 §7.2 给本 crate 定死的只有一条 crate 边：`continuum-artifact`（见 `Cargo.toml`）。
//! 不登记 `continuum-model-registry` 的理由（《工程》§4.3 的「节点放置 ← Router 输出」
//! 已记为阻断）写在设计 §7.2 与 `crates/continuum-runtime/tests/dependency_direction.rs`
//! 的 `ALLOWED` 条目里——**不是漏登记**。
//!
//! `trybuild` 是**外部 crate**，不进那张 `ALLOWED` 表——该表只断言 workspace 成员之间的边
//! （设计 §7.3）；它在 `Cargo.toml` 的 `[dev-dependencies]` 里，由 Task 2 登记。

pub mod node;

pub use node::{ComputeNode, ComputeNodeId, NodeClass, NodeTrust};
