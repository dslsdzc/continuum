//! Policy Engine：规则形状、六级优先级与受限条件（设计下篇第 5 节）。
//!
//! 分四层：
//! - [`context`]：求值条件时可见的事实集合；
//! - [`rule`]：规则、层级、决策与条件的解析和求值，以及三个枚举列的落库编码；
//! - [`engine`]：裁决（取最高层、同层取更严、无匹配即 `Deny`）；
//! - [`persist`]：`policy` 表的迁移与行级读写。
//!
//! 本 crate 不含 I/O：数据库连接在 `continuum-persist` 内私有，[`persist`] 只经
//! [`continuum_persist::Tx`] 访问。

pub mod context;
pub mod engine;
pub mod persist;
pub mod rule;

pub use context::{ExplicitApproval, PolicyContext};
pub use engine::decide;
pub use persist::{load_policies, p2_policy_migrations, save_policy};
pub use rule::{Condition, Decision, Level, Policy, PolicyError, Scope};
