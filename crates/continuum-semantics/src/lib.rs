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
//! Task 9 落 §223 的状态机与「状态迁移与事件同事务」（§5.4）：
//!
//! - **迁移判定**在 [`IntentStatus::transition`]（`intent` 模块，`pub(crate)`）：
//!   §5.3 的迁移表（**本设计的决定，不是规范转录**）、两条 `Ok` 自环
//!   （`RUNNING` / `WAITING`，与其余自环一律 `Illegal` 两条独立判定）以及
//!   `COMPLETED → EVOLVING` 的 `kind = PRODUCT` 条件；
//! - **落库版** [`apply_intent_transition`]（`intent_tx` 模块）：读当前态 → 判定 →
//!   写回 `status` / `updated_at` → 写事件，全在调用方的事务里，**不自己 `commit`**；
//! - **§5.4 的两个写入点**：[`save_intent`] 首次落库写 `EventType::IntentCreated`；
//!   [`apply_intent_transition`] 迁移到 `COMPLETED` 写 `EventType::IntentCompleted`，
//!   写完复扫事件日志做收口自检（可跳过记录之后不得存在 `intent.completed`，
//!   那是既有消费者的 MUST，见 `continuum-persist/src/tx.rs`）；
//! - 判定错误 [`IntentStatusError`] 与「没有这个 Intent」的 [`IntentError::UnknownIntent`]
//!   一并落在 `error` 模块。
//!
//! 以 `tests/intent_state.rs` 的用例钉住：§5.3 的迁移表**逐条**（31 条，不抽代表）、
//! 未列出的对得 `Illegal { from, to }` 且带出那一对、自环两侧、`COMPLETED → EVOLVING`
//! 的 kind 两侧、首次落库恰一条 `intent.created`、收口恰一条 `intent.completed`、
//! 失败路径回滚后不留半写的行、可跳过事件之后的 `intent.completed` 被拒（及其反例）。
//!
//! # 尚未做的（后续 task 的，不是本 crate 的终态）
//!
//! - **§5.4 的其余四个写入点**：`PlanReviewRequired` / `DecisionRequired`
//!   （`EventType`）与 `ContractChanges` / `UserApprovals`（`AuditKind`）由用它们的
//!   task 落（Task 11 / 13 / 15）。本 crate 今日写 `events` 的只有上面那两个写入点。
//! - **不建 ADFIR 图、不引用 `continuum-graph`**：`graph_id` 取不透明串
//!   （[`GraphIdRef`]，设计 §1.2.3 的规则 3），故本 crate 对执行层的依赖边为零
//!   （照片：`tests/intent.rs` 的 `graph_id_and_product_id_are_opaque_strings`——
//!   它扫本 crate 的 `src/`，断言执行层 crate 的**下划线标识符拼法**零命中）。
//! - 迁移 110 的其余六张表（`specification` / `contract` / `contract_requirement` /
//!   `plan` / `decision` / `decision_result`）由 Task 10／11／13／15 追加。

mod error;
mod ids;
mod intent;
mod intent_tx;
mod persist;

pub use error::{IntentError, IntentStatusError};
pub use ids::{ContractId, IntentId, PlanId};
pub use intent::{
    ContractRef, GraphIdRef, Intent, IntentKind, IntentStatus, Priority, ProductIdRef,
};
pub use intent_tx::apply_intent_transition;
pub use persist::{list_children, load_intent, p4_semantics_migrations, save_intent};
