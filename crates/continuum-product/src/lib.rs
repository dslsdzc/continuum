//! P7A 产品循环：产品对象、九维 readiness、Milestone 惰性生成、Progress Gate 与
//! Shippable 判定。
//!
//! 本 crate 是长期循环层（第 7 层）的一个库 crate。**迁移 200 与全部落库 API 条件于
//! C2 裁定**：裁定前本 crate 只交付纯类型与纯判定。

mod error;
mod predicate;
mod readiness;

pub use error::{PortError, ProductError};
pub use predicate::{ShippableClause, ShippableClauseId, ShippablePredicate};
pub use readiness::{
    ProductReadiness, ReadinessDimension, ReadinessDimensionName, ReadinessValue,
};
