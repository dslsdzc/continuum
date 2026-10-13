//! 持久化机制：迁移框架、事务 API、恢复钩子注册表。
//!
//! 数据库连接在本 crate 内私有，外部 crate 只能经 `Tx` 访问（见 P0 设计第 4 节）。

pub mod db;
pub mod error;
pub mod recovery;
pub mod value;

mod tx;

pub use db::{builtin_migrations, Db, Migration};
pub use error::PersistError;
pub use recovery::{
    run_recovery, PhaseReport, RecoveryHook, RecoveryPhase, RecoveryRegistry, RecoveryReport,
};
pub use tx::{EventLogScan, Tx};
pub use value::Value;

// 事件与审计面经本 crate 转出，供 P4 语义层使用。
//
// 依据：设计 §2.2（`docs/superpowers/specs/2026-10-06-p4-semantic-layer-design.md:259-261`）
// ——「`EventType` 与 `AuditKind` 是 `continuum-events` 的类型，本层经 `continuum-persist`
// 的 `Tx::append_event` / `Tx::append_audit` 写它们，**故不新增边**」。`Tx::append_event` /
// `Tx::append_audit` 的参数类型与 `Tx::scan_event_log` 的参数类型都来自 `continuum-events`，
// 调用方（语义层）要构造事件、写审计、装配链就必须能命名它们，故在此转出
// ——`EventType` 由 Task 9、`AuditKind` 由 Task 11 补入（后者的第一个语义层消费者是
// `continuum-semantics` 的 `save_contract` 写的 `AuditKind::ContractChanges`）。**这是转出，
// 不是新依赖边**：`continuum-persist → continuum-events` 早已存在（`tx.rs` / `recovery.rs`
// 在用）。
//
// `default_chain` 与 `EventCodecChain` 是 `scan_event_log` 的配套：链的装配点只有
// `continuum-events::codec::default_chain`（它自己的文档写着「运行期唯一的链装配点」），
// `scan_event_log` 的调用方（语义层写 `intent.completed` 后的收口自检）经这里取。
pub use continuum_events::{AuditKind, default_chain, Event, EventCodecChain, EventType};
