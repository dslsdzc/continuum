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
//! 三张表（[`persist::p3d_model_migrations`]）与它在驱动装配处的注册；Task 6 补
//! `model_registry` 一表的行级读写（[`register_model`] / [`load_lifecycle`] /
//! [`transition_in_tx`]）；Task 7 补 `model_profile` 一表的行级读写
//! （[`save_profile`] / [`load_profile`]）与那道「画像早于 `verified` 被拒」的闸门
//! （[`LifecycleError::ProfileBeforeVerified`]）；Task 8 补 `model_skill_score` 一表的行级读写
//! （[`save_skill_observation`] / [`load_skill_vector`] / [`load_skill_series`]），并让
//! [`load_profile`] **组合** [`load_skill_vector`] 填上画像的第十二个字段——三张表的行级读写
//! 至此齐了；Task 9 补 §333 的只读预算投影 [`BudgetView`]（[`budget`]）；
//! Task 10 补 §250 的**请求面**——非空需求 [`TaskSkillRequirement`]、
//! family 偏好 [`FamilyPreference`] 与请求 [`RoutingRequest`]（[`router`]）；
//! Task 11 补 §250 的**输出面与 [`rank`]**——候选集构造（判重、可用性过滤）、
//! 排序接口 [`RankingPolicy`] 与它的全序缺省实现、输出 [`RankedExecutionCandidates`]
//! 与 [`ExecutionCandidate`] / [`RoutingReason`] / [`FamilyRelation`]，
//! 以及 [`RoutingError`] 的三枚新变体。
//! Task 12 补**具名基线策略** [`BaselineRankingPolicy`]——一个只读已定义输入、用缺省全序的
//! 可替换实现（设计 §5.3 的「第二步」）。
//! 阶梯在后续 task，各自落在自己的模块里。

pub mod budget;
pub mod error;
pub mod lifecycle;
pub mod persist;
pub mod profile;
pub mod router;

pub use budget::BudgetView;
pub use error::{LifecycleError, ProfileError, RequirementError, RoutingError};
pub use lifecycle::{transition, LifecycleState, RoutableModel, RoutableState};
pub use persist::{
    load_lifecycle, load_profile, load_skill_series, load_skill_vector, p3d_model_migrations,
    register_model, save_profile, save_skill_observation, transition_in_tx,
};
pub use profile::{
    current_observation, ModelProfile, Ratio, SkillDimension, SkillObservation, SkillScore,
    SkillVector,
};
pub use router::{
    BaselineRankingPolicy, CandidateScore, ExecutionCandidate, FamilyPreference, FamilyRelation,
    RankedExecutionCandidates, RankingPolicy, RoutingReason, RoutingRequest, TaskSkillRequirement,
    rank,
};
