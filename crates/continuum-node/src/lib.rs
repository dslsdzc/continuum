//! 计算节点注册与节点放置（P3 子项目 E；设计
//! `docs/superpowers/specs/2026-10-06-p3e-compute-placement-design.md`）。
//!
//! 本 crate 是 P3 的**资源层**，落《工程》§4.1 组件表的两行——Compute Node 注册
//! （`§287`–`§291`）与节点放置（`§291` `§243` `§93`）。
//!
//! # 走到哪为止
//!
//! 本 task（Task 1）只建骨架：crate 文档与 workspace 登记。`src/node.rs`、
//! `src/registry.rs`、`src/placement.rs`、`src/error.rs` 四个模块与它们的导出面由后续
//! task 各自登记自己那几行；本 task 的 `lib.rs` 里没有 `pub mod` 声明。
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
