//! 合成作业的两阶段记录面（设计 §8.1／§8.2）。
//!
//! 本 task 只落 `begin_synthesis`／`fail_synthesis` 与作业形状；proposal 与
//! `apply_synthesis` 在 Task 8。写函数**不 `commit`**：事务由调用方持有。

use std::collections::HashSet;

use continuum_persist::{PersistError, Tx, Value};
use serde_json::json;

use crate::claim::{MemoryScope, validate_scope_anchor};
use crate::error::MemoryError;
use crate::source::{MemorySourceId, SourceKind, load_source};

/// 作业的 newtype 主键；`new` **不校验唯一性**（唯一性由 G6 判）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SynthesisJobId(String);

impl SynthesisJobId {
    pub fn new(id: impl Into<String>) -> SynthesisJobId {
        SynthesisJobId(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// §8.1 的三臂状态机；只用于「同一份作业不被 apply 两次」与失败收尾。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SynthesisStatus {
    Running,
    Applied,
    Failed,
}

impl SynthesisStatus {
    /// 落库编码的唯一产生点。
    pub fn as_str(&self) -> &'static str {
        match self {
            SynthesisStatus::Running => "running",
            SynthesisStatus::Applied => "applied",
            SynthesisStatus::Failed => "failed",
        }
    }

    /// 表外串返回 `None`，不取默认臂。
    pub fn parse(raw: &str) -> Option<SynthesisStatus> {
        match raw {
            "running" => Some(SynthesisStatus::Running),
            "applied" => Some(SynthesisStatus::Applied),
            "failed" => Some(SynthesisStatus::Failed),
            _ => None,
        }
    }
}

/// §171 的四个时机，逐字（设计 §8.2）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SynthesisTrigger {
    SessionEnded { session: MemorySourceId },
    SystemIdle { since: i64 },
    BulkProjectStateChanged { changed: Vec<MemorySourceId> },
    UserRequested,
}

impl SynthesisTrigger {
    /// 落 `trigger_kind` 的唯一产生点；穷尽 `match`，**不提供 `parse`**。
    pub fn as_str(&self) -> &'static str {
        match self {
            SynthesisTrigger::SessionEnded { .. } => "session_ended",
            SynthesisTrigger::SystemIdle { .. } => "system_idle",
            SynthesisTrigger::BulkProjectStateChanged { .. } => "bulk_project_state_changed",
            SynthesisTrigger::UserRequested => "user_requested",
        }
    }
}

/// `begin_synthesis` 的输入（设计 §8.1）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SynthesisRequest {
    pub job: SynthesisJobId,
    pub scope: MemoryScope,
    pub anchor: Option<String>,
    pub trigger: SynthesisTrigger,
    pub sources: Vec<MemorySourceId>,
    pub started_at: i64,
}

fn require_text(value: &Value, column: &str) -> Result<String, MemoryError> {
    match value {
        Value::Text(s) => Ok(s.clone()),
        other => Err(MemoryError::Persist(PersistError::Database(format!(
            "memory_synthesis_job.{column} 应为文本，实际 {other:?}"
        )))),
    }
}

/// 开一份 `Running` 作业并记录输入集合。
///
/// 判定顺序照设计 §8.2：G0 → G1 → G2／G2b → G3 → G4 → G5 → G6。
/// 全部检查通过后才写作业行与输入行，拒绝路径不留半写。
/// **不 `commit`**：返回 `Err` 后调用方必须丢弃句柄回滚。
pub fn begin_synthesis(tx: &Tx<'_>, request: &SynthesisRequest) -> Result<(), MemoryError> {
    // G0：非空、去重、存在、未删。
    if request.sources.is_empty() {
        return Err(MemoryError::EmptySourceSet);
    }
    let mut seen = HashSet::new();
    for source in &request.sources {
        if !seen.insert(source.as_str().to_string()) {
            return Err(MemoryError::DuplicateSourceId);
        }
    }
    let mut loaded: Vec<(&str, SourceKind)> = Vec::with_capacity(request.sources.len());
    for id in &request.sources {
        let Some(source) = load_source(tx, id)? else {
            return Err(MemoryError::UnknownSource);
        };
        if source.removed_at().is_some() {
            return Err(MemoryError::SourceRemoved);
        }
        loaded.push((id.as_str(), source.kind()));
    }
    let kind_of = |id: &MemorySourceId| -> Option<SourceKind> {
        loaded
            .iter()
            .find(|(source_id, _)| *source_id == id.as_str())
            .map(|(_, kind)| *kind)
    };

    // G1／G2／G2b／G3／G4：触发臂的准入。
    let trigger_ref = match &request.trigger {
        SynthesisTrigger::SessionEnded { session } => {
            let Some(kind) = kind_of(session) else {
                return Err(MemoryError::TriggerSourceOutsideInput);
            };
            if kind != SourceKind::Conversations {
                return Err(MemoryError::WrongTriggerSourceKind);
            }
            json!({ "session": session.as_str() }).to_string()
        }
        SynthesisTrigger::BulkProjectStateChanged { changed } => {
            for id in changed {
                let Some(kind) = kind_of(id) else {
                    return Err(MemoryError::TriggerSourceOutsideInput);
                };
                if kind != SourceKind::Project {
                    return Err(MemoryError::WrongTriggerSourceKind);
                }
            }
            if changed.is_empty() {
                return Err(MemoryError::EmptyTriggerSet);
            }
            let ids: Vec<&str> = changed.iter().map(MemorySourceId::as_str).collect();
            json!({ "changed": ids }).to_string()
        }
        SynthesisTrigger::SystemIdle { since } => {
            if *since > request.started_at {
                return Err(MemoryError::InvalidTriggerTime);
            }
            json!({ "since": since }).to_string()
        }
        SynthesisTrigger::UserRequested => "null".to_string(),
    };

    // G5：scope／anchor 规则。
    validate_scope_anchor(&[request.scope], request.anchor.as_deref())?;

    // G6：作业 id 已存在。
    let existing = tx.query(
        "SELECT 1 FROM memory_synthesis_job WHERE job_id = ?",
        &[Value::text(request.job.as_str())],
    )?;
    if !existing.is_empty() {
        return Err(MemoryError::DuplicateJobId);
    }

    tx.execute(
        "INSERT INTO memory_synthesis_job
           (job_id, scope, anchor_id, trigger_kind, trigger_ref, status,
            started_at, finished_at, report)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL, NULL)",
        &[
            Value::text(request.job.as_str()),
            Value::text(request.scope.as_str()),
            match request.anchor.as_deref() {
                Some(anchor) => Value::text(anchor),
                None => Value::Null,
            },
            Value::text(request.trigger.as_str()),
            Value::text(trigger_ref),
            Value::text(SynthesisStatus::Running.as_str()),
            Value::Int(request.started_at),
        ],
    )?;
    for source in &request.sources {
        tx.execute(
            "INSERT INTO memory_synthesis_input (job_id, source_id) VALUES (?, ?)",
            &[
                Value::text(request.job.as_str()),
                Value::text(source.as_str()),
            ],
        )?;
    }
    Ok(())
}

/// 把一份 `Running` 作业显式收尾为 `Failed`，`report = {"error": reason}`。
///
/// job 不存在 ⇒ `UnknownJob`；status 非 `Running` ⇒ `JobNotRunning`。
/// **不 `commit`**。
pub fn fail_synthesis(
    tx: &Tx<'_>,
    job: &SynthesisJobId,
    reason: &str,
    occurred_at: i64,
) -> Result<(), MemoryError> {
    let rows = tx.query(
        "SELECT status FROM memory_synthesis_job WHERE job_id = ?",
        &[Value::text(job.as_str())],
    )?;
    let Some(row) = rows.into_iter().next() else {
        return Err(MemoryError::UnknownJob);
    };
    let raw_status = require_text(&row[0], "status")?;
    let status = SynthesisStatus::parse(&raw_status).ok_or_else(|| {
        MemoryError::Persist(PersistError::Database(format!(
            "memory_synthesis_job.status 表外取值: {raw_status}"
        )))
    })?;
    if status != SynthesisStatus::Running {
        return Err(MemoryError::JobNotRunning);
    }
    tx.execute(
        "UPDATE memory_synthesis_job
            SET status = ?1, finished_at = ?2, report = ?3
          WHERE job_id = ?4",
        &[
            Value::text(SynthesisStatus::Failed.as_str()),
            Value::Int(occurred_at),
            Value::text(json!({ "error": reason }).to_string()),
            Value::text(job.as_str()),
        ],
    )?;
    Ok(())
}
