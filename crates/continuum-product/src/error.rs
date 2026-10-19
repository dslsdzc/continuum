//! 错误类型：一个 [`ProductError`] 收全部臂，外加端口的 [`PortError`]。
//!
//! 每一臂都指一件具体的事；**没有 `Invalid`**——把「不可判」与「明确不成立」压成同一臂
//! 会直接违反 Progress Gate／Shippable 的 fail-closed 区分（设计 §4.9）。

use continuum_persist::PersistError;
use continuum_semantics::{DecisionError, IntentError};

/// 生成端口的错误：不透明消息；实现方把模型错误映射进来（设计 §4.3）。
#[derive(Debug, Clone, PartialEq)]
pub struct PortError {
    message: String,
}

impl PortError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

/// 产品循环层的全部错误臂（设计 §4.9）。
///
/// **不派生 `PartialEq`**：上游错误类型不因此被要求实现它；测试用 `matches!` 断臂。
#[derive(Debug, thiserror::Error)]
pub enum ProductError {
    // 上游错误原样透传（不包装成第二套判定）
    #[error("持久化错误: {0}")]
    Persist(#[from] PersistError),
    #[error("Intent 错误: {0}")]
    Intent(#[from] IntentError),
    #[error("Decision 错误: {0}")]
    Decision(#[from] DecisionError),
    #[error("端口错误: {0:?}")]
    Port(PortError),

    // Product 身份与创建
    #[error("未知 Product: {id}")]
    UnknownProduct { id: String },
    #[error("Product 已存在: {id}")]
    ProductExists { id: String },
    #[error("Intent 不是 Product: {id}")]
    IntentNotProduct { id: String },
    #[error("锚 Intent 缺少 product_id: {intent}")]
    MissingProductId { intent: String },
    /// 锚 Intent 不是 READY（设计 §4.1 不变量 4）：其余十二态一律拒绝。
    #[error("锚 Intent 不是 READY: {id}（{status}）")]
    AnchorIntentNotReady { id: String, status: &'static str },
    #[error("锚 Intent 的 Contract 无效: {intent}")]
    InvalidContract { intent: String },
    #[error("EngineeringBaseline 不可用，缺 {missing}")]
    BaselineUnusable { missing: &'static str },
    #[error("ShippablePredicate 为空")]
    EmptyShippablePredicate,
    #[error("重复的 required dimension: {dimension}")]
    DuplicateDimension { dimension: &'static str },
    #[error("重复的子句 id: {id}")]
    DuplicateClause { id: String },
    #[error("子句 id 为空")]
    EmptyClauseId,
    #[error("子句 statement 为空: {id}")]
    EmptyClauseStatement { id: String },

    // readiness / evidence
    #[error("证据未被提供: {id}")]
    EvidenceNotSupplied { id: String },
    #[error("重复的证据: {id}")]
    DuplicateEvidence { id: String },
    #[error("readiness 值为空: {dimension}")]
    EmptyReadinessValue { dimension: &'static str },
    #[error("readiness 有值但无证据: {dimension}")]
    ReadinessValueWithoutEvidence { dimension: &'static str },
    #[error("readiness 有证据但无值: {dimension}")]
    ReadinessEvidenceWithoutValue { dimension: &'static str },
    #[error("readiness 的值与 last_verified 不匹配: {dimension}")]
    ReadinessLastVerifiedMismatch { dimension: &'static str },

    // Milestone / lower
    #[error("未知 Milestone: {id}")]
    UnknownMilestone { id: String },
    #[error("Milestone 属于别的 Product: {milestone}（本 Product: {product}）")]
    MilestoneProductMismatch {
        milestone: String,
        product: String,
    },
    #[error("Milestone 尚未完成: {id}")]
    MilestoneNotCompleted { id: String },
    #[error("Milestone 处于终态: {id}")]
    TerminalMilestone { id: String },
    #[error("Milestone 状态不匹配: {id}（期望 {expected}，实际 {actual}）")]
    MilestoneStateMismatch {
        id: String,
        expected: String,
        actual: String,
    },
    #[error("Milestone objective 为空")]
    EmptyMilestoneObjective,
    /// `entry_conditions`／`exit_conditions` 的某个元素是空串。
    #[error("Milestone 条件为空: {field}[{index}]")]
    EmptyMilestoneCondition { field: &'static str, index: usize },
    #[error("Progress 已存在: {product}/{milestone}")]
    DuplicateProgress { product: String, milestone: String },
    #[error("已完成 Milestone 缺少 Progress: {milestone}")]
    MissingProgressForCompleted { milestone: String },
    #[error("批次边界无效")]
    InvalidBatchBoundary,
    #[error("批次为空")]
    EmptyBatch,
    #[error("批次过大: 上限 {limit}，实际 {actual}")]
    BatchTooLarge { limit: usize, actual: usize },
    #[error("批内依赖越界: {index}")]
    DependencyOutOfRange { index: usize },
    #[error("依赖深度超界: 上限 {limit}，实际 {actual}")]
    DependencyDepthExceeded { limit: u32, actual: u32 },
    #[error("Milestone id 冲突: {id}")]
    MilestoneIdCollision { id: String },
    /// `MilestoneId` 的 ordinal 与写入 seq 不一致。
    #[error("Milestone ordinal 与 seq 不一致: {id}（期望 {expected}，实际 {actual}）")]
    MilestoneOrdinalMismatch {
        id: String,
        expected: u64,
        actual: u64,
    },
    #[error("Intent id 冲突: {id}")]
    IntentIdCollision { id: String },
    #[error("lower 出的 Intent 无效: {id}")]
    InvalidLoweredIntent { id: String },

    // Progress Gate / Decision
    #[error("Progress Gate 未触发")]
    ProgressNotTriggered,
    #[error("Progress Gate 不可判")]
    ProgressUndetermined,
    #[error("replan 字段为空: {field}")]
    EmptyReplanField { field: &'static str },

    // drive / 相位
    #[error("缺少 drive 状态: {product}")]
    MissingDriveState { product: String },
    #[error("ContinuousProductDrive 已关闭")]
    DriveDisabled,
    #[error("尚未 Shippable")]
    NotShippableYet,

    // 存储
    #[error("库行损坏: {table}（{detail}）")]
    CorruptRow {
        table: &'static str,
        detail: String,
    },
}

/// `Port` 臂手写：`#[from]` 不会为不用 `#[from]` 的臂生成 `From`。
impl From<PortError> for ProductError {
    fn from(e: PortError) -> Self {
        Self::Port(e)
    }
}
