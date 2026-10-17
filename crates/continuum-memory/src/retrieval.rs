//! 确定性检索面：`RetrievalQuery`、`MemoryRetrieval`、`MemoryStore` 与
//! `retrieve_claims`（设计 §6）。
//!
//! 只返回 claim 行；`sources[]` 里的 source 是否已删由调用方按
//! `MemorySource.removed_at` 过滤（§6.2 第 5 条）。

use continuum_persist::Tx;

use crate::claim::{MemoryClaim, MemoryScope, load_claim, validate_scope_anchor};
use crate::error::MemoryError;

/// 一次按需取 claim 的查询（设计 §6.1）。五个字段全私有，唯一构造点是 [`new`]。
///
/// [`new`]: RetrievalQuery::new
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetrievalQuery {
    scopes: Vec<MemoryScope>,
    anchor: Option<String>,
    text: Option<String>,
    limit: usize,
    include_terminal: bool,
}

impl RetrievalQuery {
    /// 唯一构造点：在这里执行 §5.4 的 scope/anchor 规则。
    ///
    /// 重复 scope 按集合去重（保留首次出现顺序）；`text = Some("")` 归一为 `None`；
    /// `limit = 0` 合法（检索时返回空集）。
    pub fn new(
        scopes: Vec<MemoryScope>,
        anchor: Option<String>,
        text: Option<String>,
        limit: usize,
        include_terminal: bool,
    ) -> Result<RetrievalQuery, MemoryError> {
        let mut unique: Vec<MemoryScope> = Vec::with_capacity(scopes.len());
        for scope in scopes {
            if !unique.contains(&scope) {
                unique.push(scope);
            }
        }
        validate_scope_anchor(&unique, anchor.as_deref())?;
        let text = match text {
            Some(s) if s.is_empty() => None,
            other => other,
        };
        Ok(RetrievalQuery {
            scopes: unique,
            anchor,
            text,
            limit,
            include_terminal,
        })
    }

    pub fn scopes(&self) -> &[MemoryScope] {
        &self.scopes
    }

    pub fn anchor(&self) -> Option<&str> {
        self.anchor.as_deref()
    }

    pub fn text(&self) -> Option<&str> {
        self.text.as_deref()
    }

    pub fn limit(&self) -> usize {
        self.limit
    }

    pub fn include_terminal(&self) -> bool {
        self.include_terminal
    }
}

/// P6A 消费的**对象安全**检索面；`MemoryStore` 是它在 `&Tx` 上的实现。
pub trait MemoryRetrieval {
    fn retrieve(&self, query: &RetrievalQuery) -> Result<Vec<MemoryClaim>, MemoryError>;
}

/// 持有 `&Tx` 的检索实现。
pub struct MemoryStore<'a> {
    tx: &'a Tx<'a>,
}

impl<'a> MemoryStore<'a> {
    pub fn new(tx: &'a Tx<'a>) -> MemoryStore<'a> {
        MemoryStore { tx }
    }
}

impl MemoryRetrieval for MemoryStore<'_> {
    fn retrieve(&self, query: &RetrievalQuery) -> Result<Vec<MemoryClaim>, MemoryError> {
        retrieve_claims(self.tx, query)
    }
}

/// 自由函数原语：`MemoryStore::retrieve` 与它共用同一实现。
pub fn retrieve_claims(
    tx: &Tx<'_>,
    query: &RetrievalQuery,
) -> Result<Vec<MemoryClaim>, MemoryError> {
    if query.limit() == 0 {
        return Ok(vec![]);
    }

    let mut sql = String::from("SELECT c.claim_id FROM memory_claim c WHERE 1 = 1");
    if !query.include_terminal() {
        sql.push_str(" AND c.validity = 'active'");
    }
    sql.push_str(" AND c.scope IN (");
    let placeholders = vec!["?"; query.scopes().len()].join(", ");
    sql.push_str(&placeholders);
    sql.push(')');

    let mut params: Vec<continuum_persist::Value> = query
        .scopes()
        .iter()
        .map(|scope| continuum_persist::Value::text(scope.as_str()))
        .collect();

    if let Some(text) = query.text() {
        sql.push_str(" AND instr(c.statement, ?) > 0");
        params.push(continuum_persist::Value::text(text));
    }
    if let Some(anchor) = query.anchor() {
        sql.push_str(
            " AND EXISTS (SELECT 1 FROM memory_claim_anchor a
                           WHERE a.claim_id = c.claim_id AND a.anchor_id = ?)",
        );
        params.push(continuum_persist::Value::text(anchor));
    }
    sql.push_str(" ORDER BY c.last_confirmed DESC, c.claim_id ASC LIMIT ?");
    params.push(continuum_persist::Value::Int(query.limit() as i64));

    let rows = tx.query(&sql, &params)?;
    let mut claims = Vec::with_capacity(rows.len());
    for row in rows {
        let raw_id = match &row[0] {
            continuum_persist::Value::Text(s) => s.clone(),
            other => {
                return Err(MemoryError::Persist(
                    continuum_persist::PersistError::Database(format!(
                        "memory_claim.claim_id 应为文本，实际 {other:?}"
                    )),
                ));
            }
        };
        let id = crate::claim::MemoryClaimId::new(raw_id);
        if let Some(claim) = load_claim(tx, &id)? {
            claims.push(claim);
        }
    }
    Ok(claims)
}

