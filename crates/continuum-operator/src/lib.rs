//! Operator 定义与注册表（§244、§245）。
//! 本子项目只定义与解析，不提供领域算子实现。

pub mod definition;
pub mod registry;

pub use definition::{
    BackendId, Determinism, Operator, OperatorId, OperatorVersion, SideEffectClass,
};
pub use registry::{OperatorError, OperatorRegistry};
