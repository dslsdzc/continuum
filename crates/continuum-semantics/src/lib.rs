//! P4 语义层（设计 `docs/superpowers/specs/2026-10-06-p4-semantic-layer-design.md`）。
//!
//! 本 crate 是 P4 的**语义层**成员，落《工程》§2.1 组件表里 Intent 存储与状态机
//! 那一格（§222 / §223）以及它之后的 Specification / Contract / Plan / Decision /
//! Constraint Validator（后五者由后续 task 追加进本 crate）。
//!
//! # 本层不依赖任何下层（设计 §1.2／§1.2.3）
//!
//! 本 crate **不登记任何指向执行层／资源层／边界层的 `Cargo.toml` 边**。凡跨层的输入，
//! 一律以「值」由装配方传入。今天有两条 workspace 内的边：
//!
//! - `continuum-semantics ← continuum-canonical`：**§1.3 读法甲的类型边**——
//!   `Intent.goal` 存规范化后的表示，取 [`continuum_canonical::EntityId`] 这一个类型；
//!   不透明占位 [`continuum_canonical::Opaque`] 也经这条边取。
//! - `continuum-semantics ← continuum-persist`：`intent` 表经 `Tx` 读写。
//!
//! **允许集里没有 `continuum-budget`**——§11.5 的第 1 条守卫（Constraint Validator 与
//! Budget Validator 两道门分离）。`continuum-budget` 那一侧的第 3 条守卫（它的允许集里
//! **有**本 crate，且没有任何以 `ConstraintVerdict` 为输入的函数）归 Task 17／19。
//!
//! # 走到哪为止
//!
//! Task 8 建骨架：crate 文档、workspace 登记（`Cargo.toml` 的 members 与
//! `crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED`），并落
//! §222 的整套 Intent 数据模型（[`Intent`] 及它的字段类型，`intent` 模块）、本层的
//! 标识类型（[`IntentId`] / [`PlanId`] / [`ContractId`]，`ids` 模块）、`intent` 表
//! （迁移 **110**）与三条 I/O（[`save_intent`] / [`load_intent`] / [`list_children`]，
//! `persist` 模块）以及错误类型 [`IntentError`]（`error` 模块）。
//!
//! 以 `tests/intent.rs` 的用例钉住：§222 的字段清单恰是那些、子列表不落列、
//! §5.2 的版本语义两侧（语义变化 +1、状态迁移不变）、五个语义字段**逐项** +1、
//! 历史不堆在 `intent` 表里、`ContractRef` 带版本、`graph_id` / `product_id` 是不透明串、
//! `intent` 表列清单逐列、枚举列的编码纪律与表外取值处置。
//!
//! # 本 task 明确不做
//!
//! - **不建状态机**：`IntentStatus` 只是词汇与落库编码，合法迁移的判定（§223）归 Task 9
//!   （`IntentStatusError` / `apply_intent_transition`）。
//! - **不写事件与审计**：§5.4 的六个写入点由用它们的 task 落；本 task 的写路径不写 `events`。
//! - **不建 ADFIR 图、不引用 `continuum-graph`**：`graph_id` 取不透明串
//!   （[`GraphIdRef`]，设计 §1.2.3 的规则 3），故本 crate 对执行层的依赖边为零
//!   （照片：`tests/intent.rs` 的 `graph_id_and_product_id_are_opaque_strings`——
//!   它扫本 crate 的 `src/`，断言执行层 crate 的**下划线标识符拼法**零命中）。
//! - 迁移 110 的其余六张表（`specification` / `contract` / `contract_requirement` /
//!   `plan` / `decision` / `decision_result`）由 Task 10／11／13／15 追加。

mod error;
mod ids;
mod intent;
mod persist;

pub use error::IntentError;
pub use ids::{ContractId, IntentId, PlanId};
pub use intent::{
    ContractRef, GraphIdRef, Intent, IntentKind, IntentStatus, Priority, ProductIdRef,
};
pub use persist::{list_children, load_intent, p4_semantics_migrations, save_intent};
