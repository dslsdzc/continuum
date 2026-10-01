//! 节点执行接口（设计 §11.2）与执行档案（§246）。
//!
//! 三个类型定义在本 crate 而非 `continuum-operator`：`NodeContext` 需要携带
//! `NodeId`，放在 operator 会使其反向依赖本 crate。

use crate::failure::RetryPolicy;
use crate::ids::NodeId;
use continuum_artifact::{Artifact, ArtifactId, ContentHash};
use continuum_operator::{BackendId, Operator, OperatorError};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// `§246`。每次 Node 执行产生一条。
///
/// 六处字段在本子项目内恒为 None 并以 `String` 承载：`model`、`provider`、
/// `tool`、`compute_node`、`reasoning_effort`、`cost_budget`。
/// 前四者 P3 接入 `ModelProvider` 与 `ToolProvider` 时收紧为强类型 id；
/// 后两者的对应类型在本子项目内不存在。`backend` 已是强类型 `BackendId`。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ExecutionProfile {
    pub model: Option<String>,
    pub provider: Option<String>,
    pub tool: Option<String>,
    pub backend: Option<BackendId>,
    pub compute_node: Option<String>,
    pub reasoning_effort: Option<String>,
    pub parallelism: Option<u32>,
    pub timeout_ms: Option<u64>,
    /// 非可选：设计 §14 与第 15 节的表列都是非空。默认值为「单次尝试、不重试」。
    pub retry_policy: RetryPolicy,
    pub cost_budget: Option<String>,
}

/// 对输入 Artifact 的引用。只带定位所需的两个身份（设计第 4.1 节）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactRef {
    pub id: ArtifactId,
    pub content_hash: ContentHash,
}

/// 节点执行上下文。
#[derive(Debug, Clone)]
pub struct NodeContext {
    node: NodeId,
    profile: ExecutionProfile,
    cancelled: Arc<AtomicBool>,
}

impl NodeContext {
    pub fn new(node: NodeId, profile: ExecutionProfile) -> Self {
        Self {
            node,
            profile,
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn node(&self) -> &NodeId {
        &self.node
    }

    pub fn profile(&self) -> &ExecutionProfile {
        &self.profile
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

/// 设计 §11.2 的算子执行接口。本子项目只定义，不提供领域实现。
pub trait OperatorImpl: Send + Sync {
    fn execute(
        &self,
        inputs: &[ArtifactRef],
        ctx: &NodeContext,
    ) -> Result<Vec<Artifact>, OperatorError>;
}

/// `§245` 的后端解析接口：判定给定后端是否在 Operator 声明的候选列表内。
///
/// 只做判定，不做选择。具体后端的选择由 Router 决定（P3）。
pub fn is_candidate_backend(operator: &Operator, backend: &BackendId) -> bool {
    operator.backend_candidates.contains(backend)
}
