//! Effect Journal：记录体、状态机、写入与推进（设计上篇第 7 节、下篇第 8 节）。
//!
//! 分三层：
//! - [`effect`]：记录体与纯函数状态机，不落库；
//! - [`persist`]：`effect` 表的迁移、枚举列编码与行级读写；
//! - [`journal`]：语义层——写入、推进、幂等键查询；
//! - [`recovery`]：重启恢复的钩子，把停在 `EXECUTING` 的记录标成 `UNKNOWN`。

pub mod effect;
pub mod journal;
pub mod persist;
pub mod recovery;

pub use effect::{Effect, EffectId, EffectState, EffectType, StateError, transition};
pub use journal::{advance, find_by_idempotency_key, load_effect, record_planned};
pub use persist::p2_effect_migrations;
pub use recovery::MarkExecutingAsUnknown;
