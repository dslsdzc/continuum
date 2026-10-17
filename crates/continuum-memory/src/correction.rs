//! Memory Correction（设计 §7）：一笔事务里产生新的 Correction Source 并把旧 claim
//! 置为 `Superseded`。**不创建 claim、不改旧 claim 的 statement／sources[]**。
//!
//! 纠正后的新说法存在 `memory_correction.statement`，供下一次合成读取（§7.2）。

use continuum_persist::{PersistError, Tx, Value};

use crate::claim::{
    MemoryClaimId, Validity, claim_anchor, load_claim,
};
use crate::error::MemoryError;
use crate::source::{MemorySource, MemorySourceId, SourceKind, insert_source};
use crate::summary::mark_summaries_stale;

/// 一次纠正：被纠正的 claim、新说法、新的 Correction Source 与发生时间。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Correction {
    pub claim: MemoryClaimId,
    pub statement: String,
    pub source: MemorySource,
    pub occurred_at: i64,
}

/// `correct` 的结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorrectionOutcome {
    pub correction_source: MemorySourceId,
    pub superseded: MemoryClaimId,
    pub staled_summaries: usize,
}

/// 纠正行的读回投影；它是下一次合成的输入，不是 claim。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorrectionRecord {
    pub correction_source: MemorySourceId,
    pub claim: MemoryClaimId,
    pub statement: String,
    pub occurred_at: i64,
}

fn text_column(value: &Value, column: &str) -> Result<String, MemoryError> {
    match value {
        Value::Text(s) => Ok(s.clone()),
        other => Err(MemoryError::Persist(PersistError::Database(format!(
            "memory_correction.{column} 应为文本，实际 {other:?}"
        )))),
    }
}

fn int_column(value: &Value, column: &str) -> Result<i64, MemoryError> {
    match value {
        Value::Int(i) => Ok(*i),
        other => Err(MemoryError::Persist(PersistError::Database(format!(
            "memory_correction.{column} 应为整数，实际 {other:?}"
        )))),
    }
}

/// 纠正一条 `Active` claim：九步顺序照设计 §7.2；**不 `commit`**。
pub fn correct(
    tx: &Tx<'_>,
    correction: &Correction,
) -> Result<CorrectionOutcome, MemoryError> {
    // 1. claim 必须存在。
    let Some(claim) = load_claim(tx, &correction.claim)? else {
        return Err(MemoryError::UnknownClaim);
    };
    // 2. 只接受 Active（终态不可再纠正）。
    if claim.validity() != Validity::Active {
        return Err(MemoryError::ClaimNotActive {
            validity: claim.validity(),
        });
    }
    // 3. 来源类型必须是显式用户纠正。
    if correction.source.kind() != SourceKind::ExplicitUserCorrections {
        return Err(MemoryError::WrongCorrectionSourceKind {
            kind: correction.source.kind(),
        });
    }
    // 4. 新说法去空白后非空（落库仍原样，不 trim）。
    if correction.statement.trim().is_empty() {
        return Err(MemoryError::EmptyStatement);
    }
    // 5. 插新 source（内部已做 DuplicateSourceId 预检，不覆盖）。
    insert_source(tx, &correction.source)?;
    // 6. 插纠正行（claim_id UNIQUE 是数据库级防线）。
    tx.execute(
        "INSERT INTO memory_correction (source_id, claim_id, statement, occurred_at)
         VALUES (?, ?, ?, ?)",
        &[
            Value::text(correction.source.id().as_str()),
            Value::text(correction.claim.as_str()),
            Value::text(&correction.statement),
            Value::Int(correction.occurred_at),
        ],
    )?;
    // 7. 旧 claim 进入 SUPERSEDED；受影响行数必须是 1。
    let affected = tx.execute(
        "UPDATE memory_claim SET validity = 'superseded'
          WHERE claim_id = ? AND validity = 'active'",
        &[Value::text(correction.claim.as_str())],
    )?;
    if affected != 1 {
        let current = load_claim(tx, &correction.claim)?;
        return match current {
            Some(current) => Err(MemoryError::ClaimNotActive {
                validity: current.validity(),
            }),
            None => Err(MemoryError::Persist(PersistError::Database(
                "纠正过程中 claim 消失".to_string(),
            ))),
        };
    }
    // 8. 该 scope/anchor 的摘要置 stale。
    let anchor = claim_anchor(tx, &correction.claim)?;
    let staled_summaries = mark_summaries_stale(tx, claim.scope(), anchor.as_deref())?;
    // 9. 返回结果。
    Ok(CorrectionOutcome {
        correction_source: correction.source.id().clone(),
        superseded: correction.claim.clone(),
        staled_summaries,
    })
}

/// 按 claim 读纠正记录；没有则 `None`。
pub fn load_correction(
    tx: &Tx<'_>,
    claim: &MemoryClaimId,
) -> Result<Option<CorrectionRecord>, MemoryError> {
    let rows = tx.query(
        "SELECT source_id, claim_id, statement, occurred_at
           FROM memory_correction WHERE claim_id = ?",
        &[Value::text(claim.as_str())],
    )?;
    let Some(row) = rows.into_iter().next() else {
        return Ok(None);
    };
    Ok(Some(CorrectionRecord {
        correction_source: MemorySourceId::new(text_column(&row[0], "source_id")?),
        claim: MemoryClaimId::new(text_column(&row[1], "claim_id")?),
        statement: text_column(&row[2], "statement")?,
        occurred_at: int_column(&row[3], "occurred_at")?,
    }))
}
