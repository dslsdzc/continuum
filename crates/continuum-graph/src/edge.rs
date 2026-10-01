//! Edge 六类型（§238）。三类语义由本设计确定，见设计第 8.2 节。

use crate::ids::NodeId;
use continuum_port::PortId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EdgeKind {
    /// 携带 Artifact 的端口连接。参与失效传播。
    Data,
    /// 仅执行序关系。不参与失效传播。
    Control,
    /// 结构性依赖。参与失效传播。
    Dependency,
    /// 验证关系。不参与调度与失效传播。
    Evidence,
    /// 外部副作用的记入关系（§268）。不参与调度与失效传播。
    Effect,
    /// 纯失效传播边（§306）。不参与调度。
    Invalidation,
}

impl EdgeKind {
    /// 是否参与失效传播：DATA、DEPENDENCY、INVALIDATION。
    pub fn propagates_invalidation(&self) -> bool {
        matches!(
            self,
            EdgeKind::Data | EdgeKind::Dependency | EdgeKind::Invalidation
        )
    }

    /// 是否参与环检测：DATA、CONTROL、DEPENDENCY、INVALIDATION。
    pub fn participates_in_cycle_check(&self) -> bool {
        !matches!(self, EdgeKind::Evidence | EdgeKind::Effect)
    }

    /// 是否参与调度排序：DATA、CONTROL、DEPENDENCY。
    pub fn orders_execution(&self) -> bool {
        matches!(
            self,
            EdgeKind::Data | EdgeKind::Control | EdgeKind::Dependency
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Edge {
    pub from_node: NodeId,
    pub from_port: PortId,
    pub to_node: NodeId,
    pub to_port: PortId,
    pub kind: EdgeKind,
}
