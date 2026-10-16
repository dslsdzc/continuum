//! 持久化机制：迁移框架、事务 API、恢复钩子注册表。
//!
//! 数据库连接在本 crate 内私有，外部 crate 只能经 `Tx` 访问（见 P0 设计第 4 节）。
//!
//! # 写入模式：批事务
//!
//!（工程指引——不是规范判据；判据由各层的设计文档给出。）
//!
//! `Tx::commit` 的代价由存储层的 **fsync** 决定：`PRAGMA synchronous=FULL`
//! 下 SQLite 在每次 COMMIT 时把 WAL 同步落盘——一次 COMMIT 就是一次 fsync；
//! 事务内的 INSERT / UPDATE / SELECT 本身不产生 fsync。于是同一批写入，
//! 「每行一个新事务」与「同事务多行、一次提交」的耗时差是**行数 × fsync 成本**
//! 的数量级，而不是某个常数。
//!
//! 差距有实测：QA 摸底在本机 btrfs（97% 满、fsync 独立探针中值约 108 ms；
//! 数字与复现命令见 `docs/2026-10-16-qa-质量摸底.md`）测得——
//! `Tx::execute` 单行插入（1 tx/行）中值 **≈178 ms/行**，批量（100 行/事务）
//! **≈2 ms/行**（约 **89×**）；`Tx::append_event` 每事件一事务中值
//! **≈268 ms/事件**，同事务 1000 条 **≈21.7 µs/事件**（约 **12 300×**）。
//! 慢盘把差距放大。即便 fsync 快 10 倍，收益仍因批次规模而异：
//! 「每事件一事务 vs 同事务千条」那一组仍在**千倍量级**（12 300× → ~1200×），
//! 而「单行 vs 百行批」那一组降到**约十倍**（89× → ~9×）——批量化的收益
//! 随批次规模收窄，但方向一致。
//!
//! **正确样板**——循环内逐条写、循环外一次提交：
//!
//! ```ignore
//! let tx = db.begin()?;
//! for event in events {
//!     tx.append_event(&event)?;   // 只写，不提交
//! }
//! tx.commit()?;                   // 全批只在这一次 COMMIT / 一次 fsync
//! ```
//!
//! **反例**——循环内逐条提交（本仓库**没有**这样的生产路径；写新路径时不要引入）：
//!
//! ```ignore
//! for event in events {
//!     let tx = db.begin()?;
//!     tx.append_event(&event)?;
//!     tx.commit()?;               // 每行一次 fsync——在本机是分钟级
//! }
//! ```
//!
//! 判据不是「事务越少越好」：**事务边界是语义的**——同一事务内的写入原子生效，
//! 崩溃后要么都可见、要么都不可见；跨事务的写入各自独立（见 `Tx` 的 Drop 与
//! 恢复侧用例）。所以先按语义划分事务，再看同一事务内是否还有「本来属于同一逻辑
//! 操作」的循环写入可以并进来（一批效应、一次导入、一组测试夹具）。反过来，
//! 为了省 fsync 把**语义上应当分开**的写入并进一个事务，是拿原子性换性能，不做。
//!
//! 现有生产路径的三种形态都合乎本指引：一次命令 / 一次工具调用一个事务
//! （`continuum-runtime` 的 task 与 tool 路径）、恢复每个阶段一个事务
//! （`recovery::run_recovery`，阶段粒度是 `failing_hook_stops_recovery_and_rolls_back_its_phase`
//! 钉住的语义），以及迁移每个迁移一个事务（`Db::migrate`——失败迁移不影响
//! 已应用的，见该函数的文档）。

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
