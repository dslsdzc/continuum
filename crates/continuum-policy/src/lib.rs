//! Policy Engine：规则形状、六级优先级与受限条件（设计下篇第 5 节）。
//!
//! 分两层：
//! - [`context`]：求值条件时可见的事实集合；
//! - [`rule`]：规则、层级、决策与条件的解析和求值。
//!
//! 本 crate 到本 task 为止只表达与求值策略：**裁决**（取最高层、同层取更严、
//! 无匹配即 `Deny`）与**落库**不在其中，分别由后续 task 的 `engine` 与
//! `persist` 模块给出。本 crate 不含 I/O。

pub mod context;
pub mod rule;

pub use context::{ExplicitApproval, PolicyContext};
pub use rule::{Condition, Decision, Level, Policy, PolicyError, Scope};
