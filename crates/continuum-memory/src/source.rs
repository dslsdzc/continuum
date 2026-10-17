//! Memory Source 图的类型面与存储面（设计 §4）。
//!
//! 本模块**不引用 claim 侧的任何类型**（设计 §3.4 的模块方向）；`remove_source`
//! 因为要在 source 与 claim 之间动状态，落在 `claim.rs`。

use crate::error::MemoryError;
use continuum_persist::{PersistError, Tx, Value};

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

fn require_text(value: &Value, column: &str) -> Result<String, MemoryError> {
    match value {
        Value::Text(s) => Ok(s.clone()),
        other => Err(MemoryError::Persist(PersistError::Database(format!(
            "memory_source.{column} 应为文本，实际 {other:?}"
        )))),
    }
}

fn require_int(value: &Value, column: &str) -> Result<i64, MemoryError> {
    match value {
        Value::Int(i) => Ok(*i),
        other => Err(MemoryError::Persist(PersistError::Database(format!(
            "memory_source.{column} 应为整数，实际 {other:?}"
        )))),
    }
}

/// 插入一条 Source。`source_id` 已存在 ⇒ `DuplicateSourceId`（不覆盖）。
///
/// **不 `commit`**：事务由调用方持有；返回 `Err` 后调用方必须丢弃句柄回滚。
pub fn insert_source(tx: &Tx<'_>, source: &MemorySource) -> Result<(), MemoryError> {
    let existing = tx.query(
        "SELECT 1 FROM memory_source WHERE source_id = ?",
        &[Value::text(source.id().as_str())],
    )?;
    if !existing.is_empty() {
        return Err(MemoryError::DuplicateSourceId);
    }
    tx.execute(
        "INSERT INTO memory_source
           (source_id, kind, external_ref, observed_at, removed_at)
         VALUES (?1, ?2, ?3, ?4, NULL)",
        &[
            Value::text(source.id().as_str()),
            Value::text(source.kind().as_str()),
            Value::text(source.external_ref()),
            Value::Int(source.observed_at()),
        ],
    )?;
    Ok(())
}

/// 按 id 读一条 Source；未知 id 返回 `Ok(None)`（读操作没有逻辑拒绝臂）。
pub fn load_source(
    tx: &Tx<'_>,
    id: &MemorySourceId,
) -> Result<Option<MemorySource>, MemoryError> {
    let rows = tx.query(
        "SELECT source_id, kind, external_ref, observed_at, removed_at
           FROM memory_source WHERE source_id = ?",
        &[Value::text(id.as_str())],
    )?;
    let Some(row) = rows.into_iter().next() else {
        return Ok(None);
    };

    let raw_kind = require_text(&row[1], "kind")?;
    let kind = SourceKind::parse(&raw_kind).ok_or_else(|| {
        MemoryError::Persist(PersistError::Database(format!(
            "memory_source.kind 表外取值: {raw_kind}"
        )))
    })?;
    let removed_at = match &row[4] {
        Value::Null => None,
        Value::Int(t) => Some(*t),
        other => {
            return Err(MemoryError::Persist(PersistError::Database(format!(
                "memory_source.removed_at 应为整数或 NULL，实际 {other:?}"
            ))));
        }
    };

    Ok(Some(MemorySource {
        id: MemorySourceId::new(require_text(&row[0], "source_id")?),
        kind,
        external_ref: require_text(&row[2], "external_ref")?,
        observed_at: require_int(&row[3], "observed_at")?,
        removed_at,
    }))
}

/// 按 id 集合读 Source。
///
/// 读回形状（本计划定）：只返回存在的 id，按 `source_id` 升序；输入里的重复 id
/// 按集合折叠；缺失 id 静默省略（与 `load_source` 的 `Option` 同向）。
/// **不排除软删除的 source**：追溯要看见墓碑（设计 §4.4）。
pub fn load_sources(
    tx: &Tx<'_>,
    ids: &[MemorySourceId],
) -> Result<Vec<MemorySource>, MemoryError> {
    let mut wanted: Vec<&str> = ids.iter().map(MemorySourceId::as_str).collect();
    wanted.sort_unstable();
    wanted.dedup();
    let mut loaded = Vec::with_capacity(wanted.len());
    for id in wanted {
        if let Some(source) = load_source(tx, &MemorySourceId::new(id))? {
            loaded.push(source);
        }
    }
    Ok(loaded)
}
