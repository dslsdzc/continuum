//! 代码领域算子（设计 §3）：九枚 `Operator` 的声明与 [`CodeError`]。
//! 本块只**声明**算子与注册期判定，不实现执行体（切分 §四第 4 条）——
//! 执行体的落点在第 3 层（未建）。
//!
//! 导出面（设计 §2.2／§3.2）：[`all_operators`] 与 [`CodeError`] 两枚从 crate 根
//! 可直达；注册入口 `register_code_operators`（§3.5）与 `bind_code_methods`（§7）
//! 由后续 task 增量加。

pub mod error;
pub mod operators;

pub use error::CodeError;
pub use operators::all_operators;
