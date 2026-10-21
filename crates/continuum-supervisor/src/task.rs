//! 任务来源与授权证据（设计 §4.2／§4.3）。

use std::fmt;

use continuum_semantics::{IntentKind, IntentStatus};

/// §299 的四类后台任务来源。恰好四变体；没有第五类，没有 `ModelSuggested`。
/// 不实现 `FromStr`／`parse`：模型输出或裸字符串不能构造来源。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskSource {
    /// §299 的 `authorized Intent`：非 Product 的 READY Intent 加有效 contract。
    AuthorizedIntent,
    /// §299 的 `policy`：第 5 层的非空授权引用。
    Policy,
    /// §299 的 `maintenance schedule`：维护计划的非空 occurrence 引用。
    MaintenanceSchedule,
    /// §299 的 `product drive`：P7A 交接的 READY Product Intent。
    ProductDrive,
}

impl TaskSource {
    /// 四个落库／展示字面量，逐字冻结。
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AuthorizedIntent => "authorized_intent",
            Self::Policy => "policy",
            Self::MaintenanceSchedule => "maintenance_schedule",
            Self::ProductDrive => "product_drive",
        }
    }
}

/// `AuthorizedTask` 的拒绝臂。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskError {
    /// Intent 不是 READY。
    NotReady { status: IntentStatus },
    /// Intent 的 kind 与构造器不匹配。
    WrongKind { kind: IntentKind },
    /// contract 的 id 为空或版本小于 1。
    InvalidContract { id: String, version: u32 },
    /// Product Intent 缺少非空的 product_id。
    MissingProductId,
    /// policy 来源的两个字段有空串。
    EmptyPolicyGrant,
    /// maintenance 来源的两个字段有空串。
    EmptyMaintenanceRef,
}

impl fmt::Display for TaskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotReady { status } => {
                write!(f, "Intent 状态 {status:?} 不是 READY")
            }
            Self::WrongKind { kind } => {
                write!(f, "Intent kind {kind:?} 与任务来源不匹配")
            }
            Self::InvalidContract { id, version } => {
                write!(f, "contract 无效：id={id:?}，version={version}")
            }
            Self::MissingProductId => f.write_str("Product Intent 缺少非空的 product_id"),
            Self::EmptyPolicyGrant => f.write_str("policy 授权引用有空字段"),
            Self::EmptyMaintenanceRef => f.write_str("maintenance occurrence 引用有空字段"),
        }
    }
}

impl std::error::Error for TaskError {}

/// policy 来源的不透明授权引用。两个字段都非空；P7D 不解释它们的语义。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyGrantRef {
    issuer: String,
    grant_id: String,
}

impl PolicyGrantRef {
    /// 只拒绝空串；不 trim、不改写。
    pub fn new(issuer: impl Into<String>, grant_id: impl Into<String>) -> Result<Self, TaskError> {
        let issuer = issuer.into();
        let grant_id = grant_id.into();
        if issuer.is_empty() || grant_id.is_empty() {
            return Err(TaskError::EmptyPolicyGrant);
        }
        Ok(Self { issuer, grant_id })
    }

    pub fn issuer(&self) -> &str {
        &self.issuer
    }

    pub fn grant_id(&self) -> &str {
        &self.grant_id
    }
}

/// maintenance schedule 来源的不透明 occurrence 引用。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenanceOccurrenceRef {
    schedule_id: String,
    occurrence_id: String,
}

impl MaintenanceOccurrenceRef {
    /// 只拒绝空串；不 trim、不改写。
    pub fn new(
        schedule_id: impl Into<String>,
        occurrence_id: impl Into<String>,
    ) -> Result<Self, TaskError> {
        let schedule_id = schedule_id.into();
        let occurrence_id = occurrence_id.into();
        if schedule_id.is_empty() || occurrence_id.is_empty() {
            return Err(TaskError::EmptyMaintenanceRef);
        }
        Ok(Self {
            schedule_id,
            occurrence_id,
        })
    }

    pub fn schedule_id(&self) -> &str {
        &self.schedule_id
    }

    pub fn occurrence_id(&self) -> &str {
        &self.occurrence_id
    }
}
