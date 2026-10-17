//! Memory Source 图的类型面与存储面（设计 §4）。
//!
//! 本模块**不引用 claim 侧的任何类型**（设计 §3.4 的模块方向）；`remove_source`
//! 因为要在 source 与 claim 之间动状态，落在 `claim.rs`。

use crate::error::MemoryError;

/// §170 八类与 §282 六词的并集：九臂（设计 §4.1）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    Conversations,
    TaskHistory,
    ProductHistory,
    ProjectArtifacts,
    Files,
    ConnectedServices,
    ExplicitUserCorrections,
    UserPolicies,
    Project,
}

impl SourceKind {
    pub const ALL: [SourceKind; 9] = [
        SourceKind::Conversations,
        SourceKind::TaskHistory,
        SourceKind::ProductHistory,
        SourceKind::ProjectArtifacts,
        SourceKind::Files,
        SourceKind::ConnectedServices,
        SourceKind::ExplicitUserCorrections,
        SourceKind::UserPolicies,
        SourceKind::Project,
    ];

    /// 落库编码的唯一产生点。
    pub fn as_str(&self) -> &'static str {
        match self {
            SourceKind::Conversations => "conversations",
            SourceKind::TaskHistory => "task_history",
            SourceKind::ProductHistory => "product_history",
            SourceKind::ProjectArtifacts => "project_artifacts",
            SourceKind::Files => "files",
            SourceKind::ConnectedServices => "connected_services",
            SourceKind::ExplicitUserCorrections => "explicit_user_corrections",
            SourceKind::UserPolicies => "user_policies",
            SourceKind::Project => "project",
        }
    }

    /// 表外串返回 `None`，不取默认臂（设计 §4.1）。
    pub fn parse(raw: &str) -> Option<SourceKind> {
        match raw {
            "conversations" => Some(SourceKind::Conversations),
            "task_history" => Some(SourceKind::TaskHistory),
            "product_history" => Some(SourceKind::ProductHistory),
            "project_artifacts" => Some(SourceKind::ProjectArtifacts),
            "files" => Some(SourceKind::Files),
            "connected_services" => Some(SourceKind::ConnectedServices),
            "explicit_user_corrections" => Some(SourceKind::ExplicitUserCorrections),
            "user_policies" => Some(SourceKind::UserPolicies),
            "project" => Some(SourceKind::Project),
            _ => None,
        }
    }
}

/// Source 的 newtype 主键。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MemorySourceId(String);

impl MemorySourceId {
    pub fn new(id: impl Into<String>) -> MemorySourceId {
        MemorySourceId(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// 一条来源观察记录（设计 §4.2）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemorySource {
    id: MemorySourceId,
    kind: SourceKind,
    external_ref: String,
    observed_at: i64,
    removed_at: Option<i64>,
}

impl MemorySource {
    /// 唯一构造点。`external_ref` 去空白后为空 ⇒ `EmptyExternalRef`；
    /// 非空时**原样存储**（不 trim）。
    pub fn new(
        id: MemorySourceId,
        kind: SourceKind,
        external_ref: impl Into<String>,
        observed_at: i64,
    ) -> Result<MemorySource, MemoryError> {
        let external_ref = external_ref.into();
        if external_ref.trim().is_empty() {
            return Err(MemoryError::EmptyExternalRef);
        }
        Ok(MemorySource {
            id,
            kind,
            external_ref,
            observed_at,
            removed_at: None,
        })
    }

    pub fn id(&self) -> &MemorySourceId {
        &self.id
    }

    pub fn kind(&self) -> SourceKind {
        self.kind
    }

    pub fn external_ref(&self) -> &str {
        &self.external_ref
    }

    pub fn observed_at(&self) -> i64 {
        self.observed_at
    }

    pub fn removed_at(&self) -> Option<i64> {
        self.removed_at
    }
}
