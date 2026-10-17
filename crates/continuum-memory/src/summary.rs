//! Memory Summary：读面与 `pub(crate)` 的写入／失效基座（设计 §6.2／§8.4）。
//!
//! **`memory_summary` 只由 `apply_synthesis` 写**：`write_summary` 是 `pub(crate)`，
//! 本模块不提供公开写入口。

use continuum_persist::{PersistError, Tx, Value};

use crate::claim::{MemoryScope, validate_scope_anchor};
use crate::error::MemoryError;
use crate::synthesis::SynthesisJobId;

/// 摘要的 newtype 主键。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MemorySummaryId(String);

impl MemorySummaryId {
    pub fn new(id: impl Into<String>) -> MemorySummaryId {
        MemorySummaryId(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// 摘要投影（设计 §8.4）。`stale = true` 的摘要不得进入上下文。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemorySummary {
    pub id: MemorySummaryId,
    pub scope: MemoryScope,
    pub anchor: Option<String>,
    pub body: String,
    pub job: SynthesisJobId,
    pub generated_at: i64,
    pub stale: bool,
}

fn require_text(value: &Value, column: &str) -> Result<String, MemoryError> {
    match value {
        Value::Text(s) => Ok(s.clone()),
        other => Err(MemoryError::Persist(PersistError::Database(format!(
            "memory_summary.{column} 应为文本，实际 {other:?}"
        )))),
    }
}

fn require_int(value: &Value, column: &str) -> Result<i64, MemoryError> {
    match value {
        Value::Int(i) => Ok(*i),
        other => Err(MemoryError::Persist(PersistError::Database(format!(
            "memory_summary.{column} 应为整数，实际 {other:?}"
        )))),
    }
}

fn load(
    tx: &Tx<'_>,
    scope: MemoryScope,
    anchor: Option<&str>,
    current_only: bool,
) -> Result<Option<MemorySummary>, MemoryError> {
    validate_scope_anchor(&[scope], anchor)?;

    // 锚点条件分两支写：`Some("")` 与 `None` 不得被 `COALESCE` 混同。
    let sql = if anchor.is_some() {
        if current_only {
            "SELECT summary_id, scope, anchor_id, body, job_id, generated_at, stale
               FROM memory_summary
              WHERE scope = ? AND anchor_id = ? AND stale = 0
              ORDER BY generated_at DESC, summary_id ASC LIMIT 1"
        } else {
            "SELECT summary_id, scope, anchor_id, body, job_id, generated_at, stale
               FROM memory_summary
              WHERE scope = ? AND anchor_id = ?
              ORDER BY generated_at DESC, summary_id ASC LIMIT 1"
        }
    } else if current_only {
        "SELECT summary_id, scope, anchor_id, body, job_id, generated_at, stale
           FROM memory_summary
          WHERE scope = ? AND anchor_id IS NULL AND stale = 0
          ORDER BY generated_at DESC, summary_id ASC LIMIT 1"
    } else {
        "SELECT summary_id, scope, anchor_id, body, job_id, generated_at, stale
           FROM memory_summary
          WHERE scope = ? AND anchor_id IS NULL
          ORDER BY generated_at DESC, summary_id ASC LIMIT 1"
    };

    let mut params = vec![Value::text(scope.as_str())];
    if let Some(anchor) = anchor {
        params.push(Value::text(anchor));
    }
    let rows = tx.query(sql, &params)?;
    let Some(row) = rows.into_iter().next() else {
        return Ok(None);
    };

    let raw_scope = require_text(&row[1], "scope")?;
    let scope = MemoryScope::parse(&raw_scope).ok_or_else(|| {
        MemoryError::Persist(PersistError::Database(format!(
            "memory_summary.scope 表外取值: {raw_scope}"
        )))
    })?;
    let anchor = match &row[2] {
        Value::Null => None,
        Value::Text(s) => Some(s.clone()),
        other => {
            return Err(MemoryError::Persist(PersistError::Database(format!(
                "memory_summary.anchor_id 应为文本或 NULL，实际 {other:?}"
            ))));
        }
    };
    let stale = match &row[6] {
        Value::Int(0) => false,
        Value::Int(_) => true,
        other => {
            return Err(MemoryError::Persist(PersistError::Database(format!(
                "memory_summary.stale 应为整数，实际 {other:?}"
            ))));
        }
    };

    Ok(Some(MemorySummary {
        id: MemorySummaryId::new(require_text(&row[0], "summary_id")?),
        scope,
        anchor,
        body: require_text(&row[3], "body")?,
        job: SynthesisJobId::new(require_text(&row[4], "job_id")?),
        generated_at: require_int(&row[5], "generated_at")?,
        stale,
    }))
}

/// 返回最新一条摘要（可 stale），按 `generated_at DESC, summary_id ASC`。
pub fn load_summary(
    tx: &Tx<'_>,
    scope: MemoryScope,
    anchor: Option<&str>,
) -> Result<Option<MemorySummary>, MemoryError> {
    load(tx, scope, anchor, false)
}

/// 只返回最新的 `stale = false` 摘要；没有则 `None`。
pub fn load_current_summary(
    tx: &Tx<'_>,
    scope: MemoryScope,
    anchor: Option<&str>,
) -> Result<Option<MemorySummary>, MemoryError> {
    load(tx, scope, anchor, true)
}

/// 写一条 fresh 摘要。**只许 `apply_synthesis` 调**。
///
/// `summary_id = "{job}:{generated_at}"`；正文原样存（空正文由调用方挡在调用前）。
#[allow(dead_code)] // Task 8 起由 apply_synthesis 使用
pub(crate) fn write_summary(
    tx: &Tx<'_>,
    job: &SynthesisJobId,
    scope: MemoryScope,
    anchor: Option<&str>,
    body: &str,
    generated_at: i64,
) -> Result<MemorySummaryId, MemoryError> {
    let id = MemorySummaryId::new(format!("{}:{}", job.as_str(), generated_at));
    tx.execute(
        "INSERT INTO memory_summary
           (summary_id, scope, anchor_id, body, job_id, generated_at, stale)
         VALUES (?, ?, ?, ?, ?, ?, 0)",
        &[
            Value::text(id.as_str()),
            Value::text(scope.as_str()),
            match anchor {
                Some(anchor) => Value::text(anchor),
                None => Value::Null,
            },
            Value::text(body),
            Value::text(job.as_str()),
            Value::Int(generated_at),
        ],
    )?;
    Ok(id)
}

/// 把某 `(scope, anchor)` 下**仍 fresh** 的摘要全部置 stale，返回受影响行数。
/// 幂等：第二次调用返回 0。
#[allow(dead_code)] // Task 8 起由 apply_synthesis 使用
pub(crate) fn mark_summaries_stale(
    tx: &Tx<'_>,
    scope: MemoryScope,
    anchor: Option<&str>,
) -> Result<usize, MemoryError> {
    let sql = if anchor.is_some() {
        "UPDATE memory_summary SET stale = 1
          WHERE scope = ? AND anchor_id = ? AND stale = 0"
    } else {
        "UPDATE memory_summary SET stale = 1
          WHERE scope = ? AND anchor_id IS NULL AND stale = 0"
    };
    let mut params = vec![Value::text(scope.as_str())];
    if let Some(anchor) = anchor {
        params.push(Value::text(anchor));
    }
    let affected = tx.execute(sql, &params)?;
    Ok(affected as usize)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::p6c_memory_migrations;
    use continuum_persist::Db;

    fn new_db() -> (tempfile::TempDir, Db) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("summary.db");
        let db = Db::open_with(&path, p6c_memory_migrations()).unwrap();
        db.migrate().unwrap();
        (dir, db)
    }

    fn insert_job(tx: &Tx<'_>, id: &str) {
        tx.execute(
            "INSERT INTO memory_synthesis_job
               (job_id, scope, anchor_id, trigger_kind, trigger_ref, status,
                started_at, finished_at, report)
             VALUES (?, 'personal', NULL, 'user_requested', 'null', 'running', 0, NULL, NULL)",
            &[Value::text(id)],
        )
        .unwrap();
    }

    #[test]
    fn mark_summaries_stale_touches_only_the_pair() {
        let (_dir, db) = new_db();
        let tx = db.begin().unwrap();
        insert_job(&tx, "j1");
        insert_job(&tx, "j2");
        insert_job(&tx, "j3");
        write_summary(
            &tx,
            &SynthesisJobId::new("j1"),
            MemoryScope::Project,
            Some("p1"),
            "p1 摘要",
            100,
        )
        .unwrap();
        write_summary(
            &tx,
            &SynthesisJobId::new("j2"),
            MemoryScope::Project,
            Some("p2"),
            "p2 摘要",
            100,
        )
        .unwrap();
        write_summary(
            &tx,
            &SynthesisJobId::new("j3"),
            MemoryScope::Personal,
            None,
            "personal 摘要",
            100,
        )
        .unwrap();

        assert_eq!(
            mark_summaries_stale(&tx, MemoryScope::Project, Some("p1")).unwrap(),
            1
        );
        assert!(load_summary(&tx, MemoryScope::Project, Some("p1"))
            .unwrap()
            .unwrap()
            .stale);
        assert!(
            !load_summary(&tx, MemoryScope::Project, Some("p2"))
                .unwrap()
                .unwrap()
                .stale
        );
        assert!(
            !load_summary(&tx, MemoryScope::Personal, None)
                .unwrap()
                .unwrap()
                .stale
        );
        assert_eq!(
            mark_summaries_stale(&tx, MemoryScope::Project, Some("p1")).unwrap(),
            0,
            "第二次调用应幂等"
        );
    }

    #[test]
    fn write_summary_is_fresh_and_uses_the_job() {
        let (_dir, db) = new_db();
        let tx = db.begin().unwrap();
        insert_job(&tx, "j1");
        let job = SynthesisJobId::new("j1");
        let id = write_summary(
            &tx,
            &job,
            MemoryScope::Project,
            Some("p1"),
            "摘要",
            300,
        )
        .unwrap();
        assert_eq!(id.as_str(), "j1:300");

        let current = load_current_summary(&tx, MemoryScope::Project, Some("p1"))
            .unwrap()
            .expect("应读到刚写的摘要");
        assert_eq!(current.id, id);
        assert!(!current.stale);
        assert_eq!(current.job, job);
        assert_eq!(current.body, "摘要");
        assert_eq!(current.generated_at, 300);
    }
}
