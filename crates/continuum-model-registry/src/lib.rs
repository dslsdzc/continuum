//! 模型注册表与路由器（P3 子项目 D；设计 `docs/superpowers/specs/2026-10-05-p3d-model-registry-router-design.md`）。
//!
//! 本 crate 是 P3 的**资源层**，落《工程》§4.1 模型侧的五项——`ModelProfile` 与
//! `SkillVector` 存储（§247 §248）、Model Registry 生命周期（§249）、
//! Router 与候选排序（§250 §84）、升级与降级的数据形状（§251 §85 §86）、
//! 成本输入的接口投影（ENG-005）。
//!
//! # 交付物是一份判断，不是一次执行
//!
//! `rank` 是同步纯函数，收 `&[RoutableModel]` 与 `&RoutingRequest`，**不接 `Tx`**、
//! 不持有 `CallId`、不消费 `ModelStream`。`ModelProvider::invoke` / `stream` 的调用方是
//! **子项目 G（模型调用路径）**——**除本句这处说明外**，本 crate 的代码里不出现这两个名字
//! （即：把它们当标识符用一次都没有；设计 §1.2）。
//!
//! # 走到哪为止
//!
//! Task 1 建骨架与两个标量取值类型 [`Ratio`] / [`SkillScore`] 及错误类型 [`ProfileError`]；
//! Task 2 补 §248 的九维 [`SkillDimension`]、§24 的观测 [`SkillObservation`] 与
//! 向量 [`SkillVector`]（含 [`current_observation`]）；Task 3 补 §247 的 [`ModelProfile`]
//! 本体的十二字段，并以 `tests/type_level.rs` 钉住它的两处不可表达性（crate 外构造不出来、
//! 读不到总分）；Task 4 补 §249 的生命周期十态 [`LifecycleState`]、可路由六态
//! [`RoutableState`]、闸门 [`RoutableModel`] 与迁移表 [`transition`]；Task 5 补 §3.1 的
//! 三张表（[`persist::p3d_model_migrations`]）与它在驱动装配处的注册。
//! 行级读写、路由、阶梯、预算各自在后续 task 落在自己的模块里。

pub mod error;
pub mod lifecycle;
pub mod persist;
pub mod profile;

pub use error::{LifecycleError, ProfileError, RoutingError};
pub use lifecycle::{transition, LifecycleState, RoutableModel, RoutableState};
pub use persist::p3d_model_migrations;
pub use profile::{
    current_observation, ModelProfile, Ratio, SkillDimension, SkillObservation, SkillScore,
    SkillVector,
};
