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
