//! Capability：能力的半封闭词汇表，以及它与 `EffectType` 的对应（设计第 2 节）。
//!
//! 分两层：
//! - [`capability`]：resource 的封闭枚举与各自的动作集，以及 `EffectType` 的对应；
//! - [`error`]：本 crate 的错误类型。
//!
//! 本 crate 不含 I/O：数据库连接在 `continuum-persist` 内私有，落库只经
//! [`continuum_persist::Tx`] 访问（使用点在后续 task 的 `tool` 表）。
//!
//! **凭据不落库**：`Capability` 只在内存中传递（设计第 2.3 节），故本 crate 至今
//! 没有任何枚举落库编码。

pub mod capability;
pub mod error;

pub use capability::{
    CapabilityKind, EmailAction, EnvAction, FsAction, GitAction, GithubAction, PaymentAction,
    RegistryAction,
};
pub use error::CapabilityError;
