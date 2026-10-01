//! ADFIR 图、Node、Edge 与连接校验。

pub mod edge;
pub mod graph;
pub mod ids;
pub mod node;

pub use edge::{Edge, EdgeKind};
pub use graph::{AdfirGraph, GraphError};
pub use ids::{ContractIdRef, GraphId, NodeId};
pub use node::{Node, NodeState, OperatorRef};
