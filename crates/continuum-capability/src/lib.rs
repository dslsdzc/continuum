//! Capability：能力的半封闭词汇表，以及它与 `EffectType` 的对应（设计第 2 节）。
//!
//! 分五层：
//! - [`capability`]：resource 的封闭枚举与各自的动作集、`EffectType` 的对应，
//!   以及 §253 的五要素 [`Capability`] 与它唯一的签发点 [`mint`]；
//! - [`tool`]：§252 的工具定义 [`Tool`] 与 §87 的登记项 [`ToolProfile`]；
//! - [`persist`]：`tool` 表的迁移与行级读写（设计第 3.3、7 节）；
//! - [`registry`]：强制点 (1)——[`authorize`] 与 [`AuthorizedTool`]，唯一能把工具
//!   交给调用方的路径（设计第 3.4 节）；
//! - [`error`]：本 crate 的错误类型。
//!
//! 本 crate 不持有数据库连接：落库经 `continuum-persist` 的 `Tx` 访问（该依赖自
//! Task 4 起声明），与 `continuum-core` 的「不含 I/O」是同一条边界。
//!
//! **凭据不落库**：`Capability` 本体只在内存中传递（设计第 2.3 节）。本 crate 落库的
//! 是 [`Tool`] / [`ToolProfile`]，其枚举落库编码为 [`CapabilityKind::as_str`]
//! （`required_capabilities` 列的元素）。[`Capability::resource`] /
//! [`Capability::action`] 返回的是 §253 的展示词汇，**不是**库列编码（按给例照录，
//! 不按落库约定拼；两者取值不同，见 [`CapabilityKind::action`] 的文档）。

pub mod capability;
pub mod error;
pub mod persist;
pub mod registry;
pub mod tool;

pub use capability::{
    Capability, CapabilityKind, EmailAction, EnvAction, FsAction, GitAction, GithubAction, Issuer,
    PaymentAction, RegistryAction, mint,
};
pub use error::CapabilityError;
pub use persist::{load_tool, load_tools, p3_capability_migrations, save_tool};
pub use registry::{AuthorizedTool, authorize};
pub use tool::{Cost, Latency, Tool, ToolProfile, Trust};

/// §252 的 `Tool.id` 与 §316 的 ToolProvider 接口类型**是同一个类型**：
/// `ToolId` 定义在 `continuum_core::tool`，本 crate 只**再导出**，不另建
/// （设计 §3.1：同一件事两个类型正是本项目一贯判为缺陷的那一类）。
pub use continuum_core::tool::ToolId;
