//! Capability：能力的半封闭词汇表，以及它与 `EffectType` 的对应（设计第 2 节）。
//!
//! 分三层：
//! - [`capability`]：resource 的封闭枚举与各自的动作集、`EffectType` 的对应，
//!   以及 §253 的五要素 [`Capability`] 与它唯一的签发点 [`mint`]；
//! - [`tool`]：§252 的工具定义 [`Tool`] 与 §87 的登记项 [`ToolProfile`]；
//! - [`error`]：本 crate 的错误类型。
//!
//! 本 crate 不含 I/O。落库（`tool` 表，设计第 3.3 节）由后续 task 接上，
//! 届时经 `continuum-persist` 的 `Tx` 访问，该依赖也在那时才声明。
//!
//! **凭据不落库**：`Capability` 只在内存中传递（设计第 2.3 节），故本 crate 至今
//! 没有任何枚举落库编码——[`Capability::resource`] / [`Capability::action`] 返回的
//! 串是 §253 的展示词汇，不是库列编码（它按给例照录，不按落库约定拼）。

pub mod capability;
pub mod error;
pub mod tool;

pub use capability::{
    Capability, CapabilityKind, EmailAction, EnvAction, FsAction, GitAction, GithubAction, Issuer,
    PaymentAction, RegistryAction, mint,
};
pub use error::CapabilityError;
pub use tool::{Cost, Latency, Tool, ToolProfile, Trust};

/// §252 的 `Tool.id` 与 §316 的 ToolProvider 接口类型**是同一个类型**：
/// `ToolId` 定义在 `continuum_core::tool`，本 crate 只**再导出**，不另建
/// （设计 §3.1：同一件事两个类型正是本项目一贯判为缺陷的那一类）。
pub use continuum_core::tool::ToolId;
