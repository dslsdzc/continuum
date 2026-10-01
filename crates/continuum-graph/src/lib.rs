//! ADFIR 图、Node、Edge 与连接校验。

pub mod edge;
pub mod failure;
pub mod graph;
pub mod ids;
pub mod invalidation;
pub mod node;
pub mod reuse;
pub mod scheduler;
pub mod state;

pub use edge::{Edge, EdgeKind};
pub use failure::{
    decide_retry, Backoff, EscalationPolicy, FailureClass, RetryDecision, RetryPolicy,
};
pub use graph::{AdfirGraph, GraphError};
pub use ids::{ContractIdRef, GraphId, NodeId};
pub use invalidation::propagate_invalidation;
pub use node::{Node, NodeState, OperatorRef};
pub use reuse::{cache_key, can_reuse, CacheKey};
pub use scheduler::{apply_blocking, apply_unblocking, select_runnable, SchedulerConfig};
pub use state::{is_terminal, transition, StateError};
