//! `MemoryClaim`、五个 scope、`Validity`、`Confidence` 与 scope/anchor 规则
//! （设计 §5.1–§5.5）。
//!
//! `remove_source` 在 Task 9 落在这里：它要在 source 与 claim 之间动状态，
//! 放进 `source.rs` 会让该文件引用 claim 侧类型，违反 S3。

use std::collections::HashSet;

use continuum_persist::{PersistError, Tx, Value};

use crate::error::MemoryError;
use crate::source::MemorySourceId;

/// §284 的五个 scope，逐字。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryScope {
    Personal,
    Project,
    Product,
    Task,
    Runtime,
}

impl MemoryScope {
    pub const ALL: [MemoryScope; 5] = [
        MemoryScope::Personal,
        MemoryScope::Project,
        MemoryScope::Product,
        MemoryScope::Task,
        MemoryScope::Runtime,
    ];

    /// 落库编码的唯一产生点。
    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryScope::Personal => "personal",
            MemoryScope::Project => "project",
            MemoryScope::Product => "product",
            MemoryScope::Task => "task",
            MemoryScope::Runtime => "runtime",
        }
    }

    /// 表外串返回 `None`，不取默认臂。
    pub fn parse(raw: &str) -> Option<MemoryScope> {
        match raw {
            "personal" => Some(MemoryScope::Personal),
            "project" => Some(MemoryScope::Project),
            "product" => Some(MemoryScope::Product),
            "task" => Some(MemoryScope::Task),
            "runtime" => Some(MemoryScope::Runtime),
            _ => None,
        }
    }

    /// `Project`／`Product`／`Task` 必须带锚点；`Personal`／`Runtime` 不得带。
    pub(crate) fn is_anchored(&self) -> bool {
        matches!(
            self,
            MemoryScope::Project | MemoryScope::Product | MemoryScope::Task
        )
    }
}

/// §283 的 `validity`：三臂，终态不可复活（设计 §5.3.1）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Validity {
    Active,
    Superseded,
    Invalidated,
}

impl Validity {
    pub const ALL: [Validity; 3] = [
        Validity::Active,
        Validity::Superseded,
        Validity::Invalidated,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Validity::Active => "active",
            Validity::Superseded => "superseded",
            Validity::Invalidated => "invalidated",
        }
    }

    pub fn parse(raw: &str) -> Option<Validity> {
        match raw {
            "active" => Some(Validity::Active),
            "superseded" => Some(Validity::Superseded),
            "invalidated" => Some(Validity::Invalidated),
            _ => None,
        }
    }
}

/// §283 的 `confidence`：只存不判；没有默认值、没有序（设计 §5.3.2）。
#[derive(Debug, Clone, PartialEq)]
pub struct Confidence(serde_json::Value);

impl Confidence {
    pub fn from_value(value: serde_json::Value) -> Confidence {
        Confidence(value)
    }

    pub fn as_value(&self) -> &serde_json::Value {
        &self.0
    }
}

/// Claim 的 newtype 主键。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MemoryClaimId(String);

impl MemoryClaimId {
    pub fn new(id: impl Into<String>) -> MemoryClaimId {
        MemoryClaimId(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// §283 的七字段，逐字（设计 §5.1）。
#[derive(Debug, Clone, PartialEq)]
pub struct MemoryClaim {
    id: MemoryClaimId,
    scope: MemoryScope,
    statement: String,
    sources: Vec<MemorySourceId>,
    validity: Validity,
    confidence: Confidence,
    last_confirmed: i64,
}

impl MemoryClaim {
    /// 唯一的生产构造点（`pub(crate)`）。只造 `Active`；`statement` 去首尾空白；
    /// `sources` 非空、去重、按 id 升序规范化。
    #[allow(dead_code)] // Task 8 起由 apply_synthesis 使用
    pub(crate) fn new(
        id: MemoryClaimId,
        scope: MemoryScope,
        statement: String,
        sources: Vec<MemorySourceId>,
        confidence: Confidence,
        last_confirmed: i64,
    ) -> Result<MemoryClaim, MemoryError> {
        let statement = statement.trim().to_string();
        if statement.is_empty() {
            return Err(MemoryError::EmptyStatement);
        }
        if sources.is_empty() {
            return Err(MemoryError::EmptySourceSet);
        }
        let mut seen = HashSet::new();
        for source in &sources {
            if !seen.insert(source.as_str().to_string()) {
                return Err(MemoryError::DuplicateSourceId);
            }
        }
        let mut sources = sources;
        sources.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        Ok(MemoryClaim {
            id,
            scope,
            statement,
            sources,
            validity: Validity::Active,
            confidence,
            last_confirmed,
        })
    }

    /// 从库行还原：**不校验、不归一**（库里的数据已过写入口）。
    #[allow(dead_code)] // Task 7 起经 load_claim 使用
    pub(crate) fn rehydrate(
        id: MemoryClaimId,
        scope: MemoryScope,
        statement: String,
        sources: Vec<MemorySourceId>,
        validity: Validity,
        confidence: Confidence,
        last_confirmed: i64,
    ) -> MemoryClaim {
        MemoryClaim {
            id,
            scope,
            statement,
            sources,
            validity,
            confidence,
            last_confirmed,
        }
    }

    pub fn id(&self) -> &MemoryClaimId {
        &self.id
    }

    pub fn scope(&self) -> MemoryScope {
        self.scope
    }

    pub fn statement(&self) -> &str {
        &self.statement
    }

    pub fn sources(&self) -> &[MemorySourceId] {
        &self.sources
    }

    pub fn validity(&self) -> Validity {
        self.validity
    }

    pub fn confidence(&self) -> &Confidence {
        &self.confidence
    }

    pub fn last_confirmed(&self) -> i64 {
        self.last_confirmed
    }
}

impl MemoryClaim {
    /// 仅供跨 crate 测试装配；未开 `test-support` 时本方法不存在。
    /// 不校验、不归一（与 `rehydrate` 同形），参数由调用方保证。
    #[cfg(feature = "test-support")]
    #[doc(hidden)]
    pub fn for_test(
        id: MemoryClaimId,
        scope: MemoryScope,
        statement: impl Into<String>,
        sources: Vec<MemorySourceId>,
        validity: Validity,
        confidence: Confidence,
        last_confirmed: i64,
    ) -> MemoryClaim {
        MemoryClaim::rehydrate(
            id,
            scope,
            statement.into(),
            sources,
            validity,
            confidence,
            last_confirmed,
        )
    }
}

/// 全块唯一的锚点校验点（设计 §5.4）。
///
/// 顺序：空 `scopes` ⇒ `EmptyScopeSet`；`anchor` 为 `Some` 时要求 `scopes` 恰好
/// 一个 anchored scope；`anchor` 为 `None` 时任一 anchored scope ⇒ `AnchorRequired`。
pub(crate) fn validate_scope_anchor(
    scopes: &[MemoryScope],
    anchor: Option<&str>,
) -> Result<(), MemoryError> {
    if scopes.is_empty() {
        return Err(MemoryError::EmptyScopeSet);
    }
    if anchor.is_some() {
        if scopes.len() != 1 || !scopes[0].is_anchored() {
            return Err(MemoryError::AnchorNotAllowed);
        }
    } else if scopes.iter().any(MemoryScope::is_anchored) {
        return Err(MemoryError::AnchorRequired);
    }
    Ok(())
}

#[allow(dead_code)] // Task 5 起经 load_claim 使用
fn require_text(value: &Value, column: &str) -> Result<String, MemoryError> {
    match value {
        Value::Text(s) => Ok(s.clone()),
        other => Err(MemoryError::Persist(PersistError::Database(format!(
            "memory_claim.{column} 应为文本，实际 {other:?}"
        )))),
    }
}

#[allow(dead_code)] // Task 5 起经 load_claim 使用
fn require_int(value: &Value, column: &str) -> Result<i64, MemoryError> {
    match value {
        Value::Int(i) => Ok(*i),
        other => Err(MemoryError::Persist(PersistError::Database(format!(
            "memory_claim.{column} 应为整数，实际 {other:?}"
        )))),
    }
}

/// 按 id 读一条 claim；未知 id 返回 `Ok(None)`。`sources[]` 按 id 升序还原。
pub(crate) fn load_claim(
    tx: &Tx<'_>,
    id: &MemoryClaimId,
) -> Result<Option<MemoryClaim>, MemoryError> {
    let rows = tx.query(
        "SELECT claim_id, scope, statement, validity, confidence, last_confirmed
           FROM memory_claim WHERE claim_id = ?",
        &[Value::text(id.as_str())],
    )?;
    let Some(row) = rows.into_iter().next() else {
        return Ok(None);
    };

    let raw_scope = require_text(&row[1], "scope")?;
    let scope = MemoryScope::parse(&raw_scope).ok_or_else(|| {
        MemoryError::Persist(PersistError::Database(format!(
            "memory_claim.scope 表外取值: {raw_scope}"
        )))
    })?;
    let raw_validity = require_text(&row[3], "validity")?;
    let validity = Validity::parse(&raw_validity).ok_or_else(|| {
        MemoryError::Persist(PersistError::Database(format!(
            "memory_claim.validity 表外取值: {raw_validity}"
        )))
    })?;
    let raw_confidence = require_text(&row[4], "confidence")?;
    let confidence = serde_json::from_str::<serde_json::Value>(&raw_confidence)
        .map(Confidence::from_value)
        .map_err(|e| {
        MemoryError::Persist(PersistError::Database(format!(
            "memory_claim.confidence 不是合法 JSON: {e}"
        )))
    })?;

    let source_rows = tx.query(
        "SELECT source_id FROM memory_claim_source WHERE claim_id = ? ORDER BY source_id ASC",
        &[Value::text(id.as_str())],
    )?;
    let mut sources = Vec::with_capacity(source_rows.len());
    for row in source_rows {
        sources.push(MemorySourceId::new(require_text(&row[0], "source_id")?));
    }

    Ok(Some(MemoryClaim::rehydrate(
        MemoryClaimId::new(require_text(&row[0], "claim_id")?),
        scope,
        require_text(&row[2], "statement")?,
        sources,
        validity,
        confidence,
        require_int(&row[5], "last_confirmed")?,
    )))
}

/// 读 claim 的锚点；无侧表行 ⇒ `None`。
#[allow(dead_code)] // Task 8 起由 apply_synthesis 使用
pub(crate) fn claim_anchor(
    tx: &Tx<'_>,
    id: &MemoryClaimId,
) -> Result<Option<String>, MemoryError> {
    let rows = tx.query(
        "SELECT anchor_id FROM memory_claim_anchor WHERE claim_id = ?",
        &[Value::text(id.as_str())],
    )?;
    match rows.into_iter().next() {
        None => Ok(None),
        Some(row) => Ok(Some(require_text(&row[0], "anchor_id")?)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{SourceKind, insert_source};
    use crate::{MemorySource, p6c_memory_migrations};
    use continuum_persist::Db;

    fn new_db() -> (tempfile::TempDir, Db) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("claim.db");
        let db = Db::open_with(&path, p6c_memory_migrations()).unwrap();
        db.migrate().unwrap();
        (dir, db)
    }

    fn conf() -> Confidence {
        Confidence::from_value(serde_json::json!({"grade": "high"}))
    }

    fn src(id: &str) -> MemorySourceId {
        MemorySourceId::new(id)
    }

    #[test]
    fn a_claim_is_constructed_only_active_with_normalized_sources() {
        let claim = MemoryClaim::new(
            MemoryClaimId::new("c1"),
            MemoryScope::Personal,
            "  用户喜欢中文  ".to_string(),
            vec![src("s2"), src("s1")],
            conf(),
            10,
        )
        .unwrap();
        assert_eq!(claim.validity(), Validity::Active);
        assert_eq!(claim.statement(), "用户喜欢中文");
        let ids: Vec<&str> = claim.sources().iter().map(MemorySourceId::as_str).collect();
        assert_eq!(ids, ["s1", "s2"]);
    }

    #[test]
    fn a_claim_constructor_rejects_empty_statement_and_empty_sources() {
        assert_eq!(
            MemoryClaim::new(
                MemoryClaimId::new("c1"),
                MemoryScope::Personal,
                "  ".to_string(),
                vec![src("s1")],
                conf(),
                0,
            ),
            Err(MemoryError::EmptyStatement)
        );
        assert_eq!(
            MemoryClaim::new(
                MemoryClaimId::new("c1"),
                MemoryScope::Personal,
                "陈述".to_string(),
                vec![],
                conf(),
                0,
            ),
            Err(MemoryError::EmptySourceSet)
        );
    }

    #[test]
    fn a_claim_constructor_rejects_duplicate_sources() {
        assert_eq!(
            MemoryClaim::new(
                MemoryClaimId::new("c1"),
                MemoryScope::Personal,
                "陈述".to_string(),
                vec![src("s2"), src("s1"), src("s2")],
                conf(),
                0,
            ),
            Err(MemoryError::DuplicateSourceId)
        );
    }

    #[test]
    fn validate_scope_anchor_applies_the_single_shape() {
        assert_eq!(
            validate_scope_anchor(&[MemoryScope::Project], None),
            Err(MemoryError::AnchorRequired)
        );
        assert_eq!(
            validate_scope_anchor(&[MemoryScope::Personal], Some("p")),
            Err(MemoryError::AnchorNotAllowed)
        );
        assert_eq!(
            validate_scope_anchor(&[MemoryScope::Personal, MemoryScope::Project], None),
            Err(MemoryError::AnchorRequired)
        );
        assert_eq!(
            validate_scope_anchor(
                &[MemoryScope::Personal, MemoryScope::Project],
                Some("p")
            ),
            Err(MemoryError::AnchorNotAllowed)
        );
        assert_eq!(
            validate_scope_anchor(&[MemoryScope::Project], Some("p")),
            Ok(())
        );
        assert_eq!(
            validate_scope_anchor(&[MemoryScope::Personal], None),
            Ok(())
        );
        assert_eq!(
            validate_scope_anchor(&[], None),
            Err(MemoryError::EmptyScopeSet)
        );
    }

    #[test]
    fn load_claim_round_trips_a_seeded_row() {
        let (_dir, db) = new_db();
        let tx = db.begin().unwrap();
        for id in ["s1", "s2"] {
            let source = MemorySource::new(src(id), SourceKind::Files, id, 1).unwrap();
            insert_source(&tx, &source).unwrap();
        }
        tx.execute(
            "INSERT INTO memory_claim
               (claim_id, scope, statement, validity, confidence, last_confirmed)
             VALUES (?, ?, ?, ?, ?, ?)",
            &[
                Value::text("c1"),
                Value::text("project"),
                Value::text("Core 不再使用 move"),
                Value::text("active"),
                Value::text(serde_json::json!({"grade": "high"}).to_string()),
                Value::Int(300),
            ],
        )
        .unwrap();
        tx.execute(
            "INSERT INTO memory_claim_source (claim_id, source_id) VALUES (?, ?)",
            &[Value::text("c1"), Value::text("s1")],
        )
        .unwrap();
        tx.execute(
            "INSERT INTO memory_claim_source (claim_id, source_id) VALUES (?, ?)",
            &[Value::text("c1"), Value::text("s2")],
        )
        .unwrap();
        tx.execute(
            "INSERT INTO memory_claim_anchor (claim_id, anchor_id) VALUES (?, ?)",
            &[Value::text("c1"), Value::text("p1")],
        )
        .unwrap();

        let claim = load_claim(&tx, &MemoryClaimId::new("c1"))
            .unwrap()
            .expect("应读到 claim");
        assert_eq!(claim.id().as_str(), "c1");
        assert_eq!(claim.scope(), MemoryScope::Project);
        assert_eq!(claim.statement(), "Core 不再使用 move");
        let ids: Vec<&str> = claim.sources().iter().map(MemorySourceId::as_str).collect();
        assert_eq!(ids, ["s1", "s2"]);
        assert_eq!(claim.validity(), Validity::Active);
        assert_eq!(claim.confidence(), &conf());
        assert_eq!(claim.last_confirmed(), 300);
        assert_eq!(
            claim_anchor(&tx, &MemoryClaimId::new("c1")).unwrap(),
            Some("p1".to_string())
        );

        // 另一条 unanchored claim。
        tx.execute(
            "INSERT INTO memory_claim
               (claim_id, scope, statement, validity, confidence, last_confirmed)
             VALUES (?, ?, ?, ?, ?, ?)",
            &[
                Value::text("c2"),
                Value::text("personal"),
                Value::text("用户喜欢中文"),
                Value::text("active"),
                Value::text("null"),
                Value::Int(1),
            ],
        )
        .unwrap();
        assert_eq!(claim_anchor(&tx, &MemoryClaimId::new("c2")).unwrap(), None);
    }

    #[test]
    fn rehydrate_does_not_validate() {
        let claim = MemoryClaim::rehydrate(
            MemoryClaimId::new("c1"),
            MemoryScope::Personal,
            String::new(),
            vec![],
            Validity::Superseded,
            Confidence::from_value(serde_json::json!(null)),
            0,
        );
        assert_eq!(claim.statement(), "");
        assert!(claim.sources().is_empty());
        assert_eq!(claim.validity(), Validity::Superseded);
    }
}
