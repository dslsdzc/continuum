//! Event Stream 与 Audit Log。P0 的第二个交付物。

pub mod audit;
pub mod codec;
pub mod event;

pub use audit::{verify_chain, AuditError, AuditKind, AuditRecord, GENESIS_HASH};
pub use codec::{decode_event, DecodedEvent, EventCodecChain, EventLogError, SkipReason};
pub use event::{Event, EventType, CURRENT_SCHEMA_VERSION};
