//! 媒体领域算子（设计 §5）：17 枚 `Operator` 的清单与注册入口。
//! §5.4 的注册期规则（`Deterministic` 算子的候选 backend 须在「逐位可复现」名单内）
//! 在 [`operators::register_media_operators`] 内落地。
//! 本子项目只登记算子与声明，不建执行路径（设计 §1.2）。

pub mod error;
pub mod operators;

pub use error::MediaError;
pub use operators::{all_operators, register_media_operators};
