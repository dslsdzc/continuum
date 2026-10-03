//! Policy Engine：规则形状、六级优先级与受限条件（设计下篇第 5 节）。
//!
//! 分三层：
//! - [`context`]：求值条件时可见的事实集合；
//! - [`rule`]：规则、层级、决策与条件的解析和求值；
//! - [`engine`]：裁决（取最高层、同层取更严、无匹配即 `Deny`）。
//!
//! **落库**不在本 crate 内：规则与效应的记录由后续 task 的 `persist` 模块给出。
//! 本 crate 不含 I/O。

pub mod context;
pub mod engine;
pub mod rule;

pub use context::{ExplicitApproval, PolicyContext};
pub use engine::decide;
pub use rule::{Condition, Decision, Level, Policy, PolicyError, Scope};
