//! 合成作业的两阶段记录面（设计 §8.1／§8.2）。
//!
//! 本 task 只落 `begin_synthesis`／`fail_synthesis` 与作业形状；proposal 与
//! `apply_synthesis` 在 Task 8。写函数**不 `commit`**：事务由调用方持有。

use std::collections::{HashMap, HashSet};

use continuum_persist::{PersistError, Tx, Value};
use serde_json::json;

use crate::claim::{
    Confidence, MemoryClaim, MemoryClaimId, MemoryScope, Validity, claim_anchor, load_claim,
    validate_scope_anchor,
};
use crate::error::MemoryError;
use crate::source::{MemorySourceId, SourceKind, load_source};
use crate::summary::{mark_summaries_stale, write_summary};

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

/// 一条待落库的提案：**不能表达** `scope`／`validity`／`last_confirmed`／`anchor`
/// （设计 §8.3；照片 C4a／C4b）。
#[derive(Debug, Clone, PartialEq)]
pub struct ProposedClaim {
    pub id: MemoryClaimId,
    pub statement: String,
    pub sources: Vec<MemorySourceId>,
    pub confidence: Confidence,
    pub supersedes: Vec<MemoryClaimId>,
}

/// 一次合成的完整提案。`summary = None` 表示本次不生成摘要。
#[derive(Debug, Clone, PartialEq)]
pub struct SynthesisProposal {
    pub claims: Vec<ProposedClaim>,
    pub summary: Option<String>,
}

/// `apply_synthesis` 的结果；同时作为作业行的 report。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SynthesisReport {
    pub created: Vec<MemoryClaimId>,
    pub merged: Vec<MemoryClaimId>,
    pub superseded: Vec<MemoryClaimId>,
    pub summary_written: bool,
    pub staled_summaries: usize,
}

fn text_column(value: &Value, table: &str, column: &str) -> Result<String, MemoryError> {
    match value {
        Value::Text(s) => Ok(s.clone()),
        other => Err(MemoryError::Persist(PersistError::Database(format!(
            "{table}.{column} 应为文本，实际 {other:?}"
        )))),
    }
}

fn load_source_observed_at(
    tx: &Tx<'_>,
    id: &MemorySourceId,
) -> Result<i64, MemoryError> {
    match load_source(tx, id)? {
        None => Err(MemoryError::UnknownSource),
        Some(source) if source.removed_at().is_some() => Err(MemoryError::SourceRemoved),
        Some(source) => Ok(source.observed_at()),
    }
}

/// 应用一份 proposal：新建／合并 claim、显式退场、失效并写摘要、收尾作业。
///
/// 执行顺序照设计 §8.4：A1 → A2（逐条，按提案顺序）→ A3.1 → A3.2 → A4。
/// 全部写入在同一 `Tx` 里；**不 `commit`**。`Err` 后调用方必须丢弃句柄回滚，
/// 作业仍是 `Running`。
pub fn apply_synthesis(
    tx: &Tx<'_>,
    job: &SynthesisJobId,
    proposal: &SynthesisProposal,
    occurred_at: i64,
) -> Result<SynthesisReport, MemoryError> {
    // ── A1：作业前置 ───────────────────────────────────────────────────────
    let rows = tx.query(
        "SELECT scope, anchor_id, status FROM memory_synthesis_job WHERE job_id = ?",
        &[Value::text(job.as_str())],
    )?;
    let Some(row) = rows.into_iter().next() else {
        return Err(MemoryError::UnknownJob);
    };
    let raw_status = text_column(&row[2], "memory_synthesis_job", "status")?;
    let status = SynthesisStatus::parse(&raw_status).ok_or_else(|| {
        MemoryError::Persist(PersistError::Database(format!(
            "memory_synthesis_job.status 表外取值: {raw_status}"
        )))
    })?;
    if status != SynthesisStatus::Running {
        return Err(MemoryError::JobNotRunning);
    }
    let raw_scope = text_column(&row[0], "memory_synthesis_job", "scope")?;
    let scope = MemoryScope::parse(&raw_scope).ok_or_else(|| {
        MemoryError::Persist(PersistError::Database(format!(
            "memory_synthesis_job.scope 表外取值: {raw_scope}"
        )))
    })?;
    let anchor = match &row[1] {
        Value::Null => None,
        Value::Text(s) => Some(s.clone()),
        other => {
            return Err(MemoryError::Persist(PersistError::Database(format!(
                "memory_synthesis_job.anchor_id 应为文本或 NULL，实际 {other:?}"
            ))));
        }
    };

    let input_rows = tx.query(
        "SELECT source_id FROM memory_synthesis_input WHERE job_id = ?",
        &[Value::text(job.as_str())],
    )?;
    let job_sources: HashSet<String> = input_rows
        .iter()
        .map(|row| text_column(&row[0], "memory_synthesis_input", "source_id"))
        .collect::<Result<_, _>>()?;

    let mut created: Vec<MemoryClaimId> = Vec::new();
    let mut merged: Vec<MemoryClaimId> = Vec::new();
    let mut superseded: Vec<MemoryClaimId> = Vec::new();
    let mut touched_pairs: Vec<(MemoryScope, Option<String>)> = Vec::new();
    let mut batch_ids: HashSet<String> = HashSet::new();

    // ── A2：逐条 proposal，按顺序 ─────────────────────────────────────────
    for entry in &proposal.claims {
        // A2.1：id 不与库中已有、也不与本批前面的重复。
        if load_claim(tx, &entry.id)?.is_some() || !batch_ids.insert(entry.id.as_str().to_string())
        {
            return Err(MemoryError::DuplicateClaimId);
        }

        // A2.2：statement 去空白后非空；落库用 trim 后的值。
        let statement = entry.statement.trim().to_string();
        if statement.is_empty() {
            return Err(MemoryError::EmptyStatement);
        }

        // A2.3：sources 非空、去重（不静默去重），按 id 升序规范化。
        if entry.sources.is_empty() {
            return Err(MemoryError::EmptySourceSet);
        }
        let mut source_ids = HashSet::new();
        for source in &entry.sources {
            if !source_ids.insert(source.as_str().to_string()) {
                return Err(MemoryError::DuplicateSourceId);
            }
        }
        let mut sources = entry.sources.clone();
        sources.sort_by(|a, b| a.as_str().cmp(b.as_str()));

        // A2.4：每个 source 存在且未删；记下 observed_at。
        let mut observed: HashMap<String, i64> = HashMap::new();
        for source in &sources {
            observed.insert(source.as_str().to_string(), load_source_observed_at(tx, source)?);
        }

        // A2.5：去重键只判定、不落库。
        let dedupe_sql = if anchor.is_some() {
            "SELECT claim_id FROM memory_claim c
              WHERE c.scope = ? AND c.validity = 'active' AND c.statement = ?
                AND EXISTS (SELECT 1 FROM memory_claim_anchor a
                             WHERE a.claim_id = c.claim_id AND a.anchor_id = ?)
              ORDER BY c.claim_id ASC LIMIT 1"
        } else {
            "SELECT claim_id FROM memory_claim c
              WHERE c.scope = ? AND c.validity = 'active' AND c.statement = ?
                AND NOT EXISTS (SELECT 1 FROM memory_claim_anchor a
                                 WHERE a.claim_id = c.claim_id)
              ORDER BY c.claim_id ASC LIMIT 1"
        };
        let mut dedupe_params = vec![
            Value::text(scope.as_str()),
            Value::text(&statement),
        ];
        if let Some(anchor) = anchor.as_deref() {
            dedupe_params.push(Value::text(anchor));
        }
        let existing = tx.query(dedupe_sql, &dedupe_params)?;
        let merge_target = match existing.into_iter().next() {
            Some(row) => {
                let target = MemoryClaimId::new(text_column(&row[0], "memory_claim", "claim_id")?);
                let old = load_claim(tx, &target)?.ok_or_else(|| {
                    MemoryError::Persist(PersistError::Database(
                        "去重键命中的 claim 在事务内消失".to_string(),
                    ))
                })?;

                // A2.4 的另一半：已连到合并目标的旧 source 也要存在且未删。
                for old_source in old.sources() {
                    load_source_observed_at(tx, old_source)?;
                }
                let old_ids: HashSet<&str> =
                    old.sources().iter().map(MemorySourceId::as_str).collect();
                let new_sources: Vec<MemorySourceId> = sources
                    .iter()
                    .filter(|source| !old_ids.contains(source.as_str()))
                    .cloned()
                    .collect();

                // A2.6：合并时只有**新增的** source 受输入集合边界约束。
                for source in &new_sources {
                    if !job_sources.contains(source.as_str()) {
                        return Err(MemoryError::ForeignSource);
                    }
                }

                let proposal_max = sources
                    .iter()
                    .map(|source| observed[source.as_str()])
                    .max()
                    .expect("A2.3 已保证 sources 非空");
                let last_confirmed = old.last_confirmed().max(proposal_max);

                for source in &new_sources {
                    tx.execute(
                        "INSERT INTO memory_claim_source (claim_id, source_id) VALUES (?, ?)",
                        &[Value::text(target.as_str()), Value::text(source.as_str())],
                    )?;
                }
                tx.execute(
                    "UPDATE memory_claim SET last_confirmed = ?, confidence = ? WHERE claim_id = ?",
                    &[
                        Value::Int(last_confirmed),
                        Value::text(entry.confidence.as_value().to_string()),
                        Value::text(target.as_str()),
                    ],
                )?;
                touched_pairs.push((scope, anchor.clone()));
                merged.push(target.clone());
                Some(target)
            }
            None => {
                // A2.6：新建时提案全部 source 必须 ∈ 作业输入集合。
                for source in &sources {
                    if !job_sources.contains(source.as_str()) {
                        return Err(MemoryError::ForeignSource);
                    }
                }
                let last_confirmed = sources
                    .iter()
                    .map(|source| observed[source.as_str()])
                    .max()
                    .expect("A2.3 已保证 sources 非空");
                let claim = MemoryClaim::new(
                    entry.id.clone(),
                    scope,
                    statement,
                    sources,
                    entry.confidence.clone(),
                    last_confirmed,
                )?;
                tx.execute(
                    "INSERT INTO memory_claim
                       (claim_id, scope, statement, validity, confidence, last_confirmed)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    &[
                        Value::text(claim.id().as_str()),
                        Value::text(claim.scope().as_str()),
                        Value::text(claim.statement()),
                        Value::text(claim.validity().as_str()),
                        Value::text(claim.confidence().as_value().to_string()),
                        Value::Int(claim.last_confirmed()),
                    ],
                )?;
                for source in claim.sources() {
                    tx.execute(
                        "INSERT INTO memory_claim_source (claim_id, source_id) VALUES (?, ?)",
                        &[
                            Value::text(claim.id().as_str()),
                            Value::text(source.as_str()),
                        ],
                    )?;
                }
                if anchor.is_some() {
                    tx.execute(
                        "INSERT INTO memory_claim_anchor (claim_id, anchor_id) VALUES (?, ?)",
                        &[
                            Value::text(claim.id().as_str()),
                            Value::text(anchor.as_deref().expect("上面已判 Some")),
                        ],
                    )?;
                }
                touched_pairs.push((scope, anchor.clone()));
                created.push(claim.id().clone());
                None
            }
        };

        // A2.7：显式退场名单。
        for target in &entry.supersedes {
            let Some(target_claim) = load_claim(tx, target)? else {
                return Err(MemoryError::UnknownClaim);
            };
            if target_claim.validity() != Validity::Active {
                return Err(MemoryError::ClaimNotActive {
                    validity: target_claim.validity(),
                });
            }
            if target_claim.scope() != scope
                || claim_anchor(tx, target)?.as_deref() != anchor.as_deref()
            {
                return Err(MemoryError::ScopeMismatch);
            }
            if merge_target.as_ref() == Some(target) {
                return Err(MemoryError::SupersedeMergeConflict);
            }
            let affected = tx.execute(
                "UPDATE memory_claim SET validity = 'superseded'
                  WHERE claim_id = ? AND validity = 'active'",
                &[Value::text(target.as_str())],
            )?;
            if affected != 1 {
                return Err(MemoryError::ClaimNotActive {
                    validity: target_claim.validity(),
                });
            }
            touched_pairs.push((scope, anchor.clone()));
            superseded.push(target.clone());
        }
    }

    // ── A3.1：先失效（即使 summary == None 也做） ─────────────────────────
    let mut unique_pairs: Vec<(MemoryScope, Option<String>)> = Vec::new();
    for pair in touched_pairs {
        if !unique_pairs.contains(&pair) {
            unique_pairs.push(pair);
        }
    }
    let mut staled_summaries = 0usize;
    for (pair_scope, pair_anchor) in &unique_pairs {
        staled_summaries += mark_summaries_stale(tx, *pair_scope, pair_anchor.as_deref())?;
    }

    // ── A3.2：再写新摘要 ───────────────────────────────────────────────────
    let summary_written = match &proposal.summary {
        Some(body) => {
            if body.trim().is_empty() {
                return Err(MemoryError::EmptySummary);
            }
            write_summary(tx, job, scope, anchor.as_deref(), body, occurred_at)?;
            true
        }
        None => false,
    };

    // ── A4：作业收尾 ───────────────────────────────────────────────────────
    let report = SynthesisReport {
        created,
        merged,
        superseded,
        summary_written,
        staled_summaries,
    };
    let report_json = json!({
        "created": report.created.iter().map(MemoryClaimId::as_str).collect::<Vec<_>>(),
        "merged": report.merged.iter().map(MemoryClaimId::as_str).collect::<Vec<_>>(),
        "superseded": report.superseded.iter().map(MemoryClaimId::as_str).collect::<Vec<_>>(),
        "summary_written": report.summary_written,
        "staled_summaries": report.staled_summaries,
    })
    .to_string();
    tx.execute(
        "UPDATE memory_synthesis_job
            SET status = 'applied', finished_at = ?, report = ?
          WHERE job_id = ? AND status = 'running'",
        &[
            Value::Int(occurred_at),
            Value::text(report_json),
            Value::text(job.as_str()),
        ],
    )?;
    Ok(report)
}
