//! 连接器边界层（§124 / §125）：连接器的操作集与它的**声明完整性**、
//! 操作 → 授权的映射、连接器侧的强制点 (2)、以及凭据交付路径。
//!
//! 分五层：
//! - [`binding`]：一条「操作 → 授权」的绑定 [`OpBinding`]——两个字段都是既有类型，
//!   本 crate 不新增授权词汇表（设计 §3.2 决定 B-3）；
//! - [`registry`]：连接器作者实现的扩展 trait [`ConnectorImpl`] 与注册入口
//!   [`ConnectorRegistry`]——注册期全部核对的唯一产生点，也是绑定的唯一产生点；
//! - [`entry`]：入口 [`ConnectorRegistry::invoke`]——**唯一一条通往 `Connector::invoke`
//!   的路**（决定 B-2）：四步核对、按出示的那枚能力逐次签发凭据、取料、再经适配器转发。
//!   入口收两臂的出示值 [`ConnectorAuthorization`]（效应臂 / 非效应臂，设计 §3.5）；
//! - [`adapter`]：逐次调用的适配器——§124 的 `invoke` 放不下凭据（决定 B-4），材料经它
//!   的**带外参数**到达实现，绝不进 `input`。**不开放为公开 API**（没有消费者，公开它
//!   等于给材料多一个入口）；
//! - [`error`]：本 crate 的错误类型 [`ConnectorError`]。
//!
//! # 已落地的范围
//!
//! 注册期的**四条核对已齐**：声明集 ↔ 绑定集的双向覆盖、一一（`DuplicateKindBinding`）、
//! 以及操作的服务半边相符（`OperationServiceMismatch`）。入口自 Task 5 起**四步齐**：
//! 未注册（第 1 步）、操作未声明（第 2 步）、臂错（第 3 步，`EffectAuthorizationRequired`
//! ——**fail-open 的那一侧**）、kind 不符（第 4 步）。两臂都由本入口把关，判据同一条
//! （决定 B-3b）。
//!
//! 本 crate **不**决定、也不产出授权：连接器是能力的持有者与行使者，不是来源
//! （§124 定性，设计 §3.1 的读法）。

mod adapter;
mod entry;
pub mod binding;
pub mod error;
pub mod registry;

pub use binding::OpBinding;
pub use entry::ConnectorAuthorization;
pub use error::ConnectorError;
pub use registry::{ConnectorImpl, ConnectorRegistry};
