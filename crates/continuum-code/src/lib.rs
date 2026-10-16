//! 代码领域算子（设计 §3）：九枚 `Operator` 的声明与 [`CodeError`]。
//! 本块只**声明**算子与注册期判定，不实现执行体（切分 §四第 4 条）——
//! 执行体的落点在第 3 层（未建）。
//!
//! 导出面（设计 §2.2／§3.2／§3.5／§7.1）：[`all_operators`]、[`CodeError`] 与注册入口
//! [`register_code_operators`] / [`REPRODUCIBLE_BACKENDS`]、以及方法库的填写入口
//! [`bind_code_methods`]（§7）从 crate 根可直达。

pub mod error;
pub mod methods;
pub mod operators;
pub mod register;

pub use error::CodeError;
pub use methods::bind_code_methods;
pub use operators::all_operators;
pub use register::{register_code_operators, REPRODUCIBLE_BACKENDS};
