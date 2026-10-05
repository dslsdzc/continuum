//! 连接器边界层（§124 / §125）：连接器的操作集与它的**声明完整性**、
//! 操作 → 授权的映射、连接器侧的强制点 (2)、以及凭据交付路径。
//!
//! 分三层：
//! - [`binding`]：一条「操作 → 授权」的绑定 [`OpBinding`]——两个字段都是既有类型，
//!   本 crate 不新增授权词汇表（设计 §3.2 决定 B-3）；
//! - [`registry`]：连接器作者实现的扩展 trait [`ConnectorImpl`] 与注册入口
//!   [`ConnectorRegistry`]——注册期全部核对的唯一产生点，也是绑定的唯一产生点；
//! - [`error`]：本 crate 的错误类型 [`ConnectorError`]。
//!
//! # 本 task 的范围
//!
//! 本轮只到注册期的四条核对中的**前两条**（声明集 ↔ 绑定集的双向覆盖）。
//! 「一一」（`DuplicateKindBinding`）与「操作的服务半边相符」（`OperationServiceMismatch`）
//! 由 Task 3 增量加；调用入口（四步核对、凭据的逐次签发与取料）自 Task 4 起。
//! 故 `ConnectorRegistry` 目前**只写不读**：登记下来的连接器与绑定由 Task 4 的入口读取。
//!
//! 本 crate **不**决定、也不产出授权：连接器是能力的持有者与行使者，不是来源
//! （§124 定性，设计 §3.1 的读法）。

pub mod binding;
pub mod error;
pub mod registry;

pub use binding::OpBinding;
pub use error::ConnectorError;
pub use registry::{ConnectorImpl, ConnectorRegistry};
