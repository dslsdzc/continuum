//! 事务 API。
//!
//! 状态更新与对应事件必须在同一事务内提交（§318）。
//! 由于数据库连接是 `Db` 的私有字段，外部 crate 只能经本类型写入。

use crate::value::Value;
use crate::{Db, PersistError};
use continuum_events::audit::{record_hash, AuditKind, AuditRecord, GENESIS_HASH};
use continuum_events::{DecodedEvent, Event, EventCodecChain, EventType};
use serde_json::Value as JsonValue;

pub struct Tx<'a> {
    pub(crate) guard: std::sync::MutexGuard<'a, rusqlite::Connection>,
    done: bool,
}

impl<'a> Tx<'a> {
    pub(crate) fn begin(db: &'a Db) -> Result<Tx<'a>, PersistError> {
        let guard = db.conn.lock().unwrap_or_else(|e| e.into_inner());
        guard
            .execute_batch("BEGIN IMMEDIATE")
            .map_err(crate::error::db)?;
        Ok(Tx { guard, done: false })
    }

    pub fn query(&self, sql: &str, params: &[Value]) -> Result<Vec<Vec<Value>>, PersistError> {
        crate::db::query_rows(&self.guard, sql, params)
    }

    pub fn execute(&self, sql: &str, params: &[Value]) -> Result<u64, PersistError> {
        let bound = crate::value::params(params);
        let refs: Vec<&dyn rusqlite::ToSql> =
            bound.iter().map(|v| v as &dyn rusqlite::ToSql).collect();
        self.guard
            .execute(sql, refs.as_slice())
            .map(|n| n as u64)
            .map_err(crate::error::db)
    }

    /// 提交事务。消费 `self`，之后不能再用。
    pub fn commit(mut self) -> Result<(), PersistError> {
        self.guard
            .execute_batch("COMMIT")
            .map_err(crate::error::db)?;
        self.done = true;
        Ok(())
    }

    pub fn append_event(&self, event: &Event) -> Result<(), PersistError> {
        let payload =
            serde_json::to_string(&event.payload).map_err(|e| PersistError::Database(e.to_string()))?;
        self.execute(
            "INSERT INTO events
               (event_id, event_type, schema_version, occurred_at, intent_id, node_id, ignorable, payload)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            &[
                Value::text(event.event_id.clone()),
                Value::text(event.event_type.as_str()),
                Value::Int(event.schema_version as i64),
                Value::Int(event.occurred_at),
                match &event.intent_id {
                    Some(s) => Value::text(s.clone()),
                    None => Value::Null,
                },
                match &event.node_id {
                    Some(s) => Value::text(s.clone()),
                    None => Value::Null,
                },
                // 必须显式写入，否则 with_ignorable(true) 会被静默丢弃
                Value::Int(i64::from(event.ignorable)),
                Value::text(payload),
            ],
        )?;
        Ok(())
    }

    /// 追加一条审计记录。seq 与 prev_hash 由本方法从当前链尾推出，
    /// 调用方不指定，避免链断裂。
    pub fn append_audit(
        &self,
        kind: AuditKind,
        occurred_at: i64,
        payload: Value,
    ) -> Result<AuditRecord, PersistError> {
        let prev_hash = self.last_audit_hash()?;
        let seq: i64 = self
            .query("SELECT COALESCE(MAX(seq), 0) + 1 FROM audit_log", &[])?
            .into_iter()
            .next()
            .and_then(|r| match r.into_iter().next() {
                Some(Value::Int(i)) => Some(i),
                _ => None,
            })
            .unwrap_or(1);
        let payload_json = match &payload {
            Value::Text(s) => {
                serde_json::from_str::<JsonValue>(s).unwrap_or_else(|_| JsonValue::String(s.clone()))
            }
            Value::Int(i) => JsonValue::from(*i),
            Value::Real(f) => JsonValue::from(*f),
            Value::Null => JsonValue::Null,
            Value::Blob(_) => {
                return Err(PersistError::ParamType {
                    index: 2,
                    actual: "blob",
                })
            }
        };
        let hash = record_hash(&prev_hash, seq, kind, occurred_at, &payload_json);
        self.execute(
            "INSERT INTO audit_log (seq, kind, occurred_at, payload, prev_hash, record_hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            &[
                Value::Int(seq),
                Value::text(kind.as_str()),
                Value::Int(occurred_at),
                Value::text(serde_json::to_string(&payload_json).expect("payload 可序列化")),
                Value::text(prev_hash.clone()),
                Value::text(hash.clone()),
            ],
        )?;
        Ok(AuditRecord {
            seq,
            kind,
            occurred_at,
            payload: payload_json,
            prev_hash,
            record_hash: hash,
        })
    }

    /// 当前链尾的 record_hash。空链返回 `GENESIS_HASH`。
    pub fn last_audit_hash(&self) -> Result<String, PersistError> {
        let rows = self.query(
            "SELECT record_hash FROM audit_log ORDER BY seq DESC LIMIT 1",
            &[],
        )?;
        match rows.into_iter().next().and_then(|r| r.into_iter().next()) {
            Some(Value::Text(s)) => Ok(s),
            Some(other) => Err(PersistError::ColumnType {
                index: 0,
                actual: crate::value::kind_name(&other),
            }),
            None => Ok(GENESIS_HASH.to_owned()),
        }
    }

    pub fn audit_records(&self) -> Result<Vec<AuditRecord>, PersistError> {
        let rows = self.query(
            "SELECT seq, kind, occurred_at, payload, prev_hash, record_hash
             FROM audit_log ORDER BY seq ASC",
            &[],
        )?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let mut it = row.into_iter();
            let seq = match it.next() {
                Some(Value::Int(i)) => i,
                other => {
                    return Err(PersistError::ColumnType {
                        index: 0,
                        actual: other.as_ref().map(crate::value::kind_name).unwrap_or("missing"),
                    })
                }
            };
            let kind_text = match it.next() {
                Some(Value::Text(s)) => s,
                other => {
                    return Err(PersistError::ColumnType {
                        index: 1,
                        actual: other.as_ref().map(crate::value::kind_name).unwrap_or("missing"),
                    })
                }
            };
            let occurred_at = match it.next() {
                Some(Value::Int(i)) => i,
                other => {
                    return Err(PersistError::ColumnType {
                        index: 2,
                        actual: other.as_ref().map(crate::value::kind_name).unwrap_or("missing"),
                    })
                }
            };
            let payload_text = match it.next() {
                Some(Value::Text(s)) => s,
                other => {
                    return Err(PersistError::ColumnType {
                        index: 3,
                        actual: other.as_ref().map(crate::value::kind_name).unwrap_or("missing"),
                    })
                }
            };
            let prev_hash = match it.next() {
                Some(Value::Text(s)) => s,
                other => {
                    return Err(PersistError::ColumnType {
                        index: 4,
                        actual: other.as_ref().map(crate::value::kind_name).unwrap_or("missing"),
                    })
                }
            };
            let record_hash = match it.next() {
                Some(Value::Text(s)) => s,
                other => {
                    return Err(PersistError::ColumnType {
                        index: 5,
                        actual: other.as_ref().map(crate::value::kind_name).unwrap_or("missing"),
                    })
                }
            };
            let kind: AuditKind = serde_json::from_value(JsonValue::String(kind_text.clone()))
                .map_err(|_| PersistError::Database(format!("未知审计类型: {kind_text}")))?;
            let payload: JsonValue = serde_json::from_str(&payload_text)
                .map_err(|e| PersistError::Database(e.to_string()))?;
            out.push(AuditRecord {
                seq,
                kind,
                occurred_at,
                payload,
                prev_hash,
                record_hash,
            });
        }
        Ok(out)
    }

    /// 重算整条链。改写任一记录后本条或其后任一条的哈希不匹配。
    pub fn verify_audit_chain(&self) -> Result<(), PersistError> {
        let records = self.audit_records()?;
        continuum_events::audit::verify_chain(&records)
            .map_err(|e| PersistError::Database(e.to_string()))
    }
}

impl Drop for Tx<'_> {
    fn drop(&mut self) {
        if !self.done {
            let _ = self.guard.execute_batch("ROLLBACK");
        }
    }
}

/// 事件日志扫描结果（P0 设计第 4.1 节）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventLogScan {
    pub decoded: usize,
    pub skipped_records: usize,
}

fn text_at(row: &[Value], index: usize) -> Result<String, PersistError> {
    match row.get(index) {
        Some(Value::Text(s)) => Ok(s.clone()),
        Some(other) => Err(PersistError::ColumnType {
            index,
            actual: crate::value::kind_name(other),
        }),
        None => Err(PersistError::ColumnType {
            index,
            actual: "missing",
        }),
    }
}

/// ignorable 列是 INTEGER，而 `Event.ignorable` 是 bool。
/// 经 `json_at` 会得到 JSON number，serde 拒绝 number→bool，
/// 使每条正常写入的事件都被误判为可跳过。故单独还原为 JSON bool。
fn bool_at(row: &[Value], index: usize) -> bool {
    matches!(row.get(index), Some(Value::Int(i)) if *i != 0)
}

/// 宽松取值：列里存了非预期类型时按 JSON 原样交给解码器，
/// 由解码器判定可跳过还是致命。schema_version 被写成文本即走这条路径。
fn json_at(row: &[Value], index: usize) -> JsonValue {
    match row.get(index) {
        Some(Value::Int(i)) => JsonValue::from(*i),
        Some(Value::Real(f)) => JsonValue::from(*f),
        Some(Value::Text(s)) => JsonValue::from(s.clone()),
        Some(Value::Null) | Some(Value::Blob(_)) | None => JsonValue::Null,
    }
}

impl Tx<'_> {
    /// 按 rowid 顺序解码事件日志，实现 P0 设计第 4.1 节的判定规则。
    ///
    /// `chain` 由调用方装配并校验过缺口（见 `run_recovery` 第一阶段）。
    /// 每条记录按其 `schema_version` 列在该链中取解码器：版本号不在链中
    /// （含未来版本）为致命，与「未知事件类型默认读时必需」同向。
    ///
    /// 致命：版本号不在链中；未知且未标记 ignorable 的事件类型；
    ///       可跳过记录之后存在 intent.completed（含该记录本身不可读）。
    /// 可跳过：payload 列不可读或不是合法 JSON；版本号列不可判定；
    ///         信封其它字段类型错。
    pub fn scan_event_log(&self, chain: &EventCodecChain) -> Result<EventLogScan, PersistError> {
        let rows = self.query(
            "SELECT event_id, event_type, schema_version, occurred_at, intent_id, node_id,
                    ignorable, payload
             FROM events ORDER BY rowid ASC",
            &[],
        )?;

        let mut decoded = 0usize;
        let mut skipped_records = 0usize;
        let mut saw_skip = false;

        for row in &rows {
            let event_type = text_at(row, 1)?;

            // payload 列不可读、或不是合法 JSON，一律记可跳过。
            // 不要把解析失败降级成 JSON null：payload 的类型是 Value，
            // null 合法，那样这类损坏会被计为已解码，收口升级随之漏判。
            let payload = match row.get(7) {
                Some(Value::Text(s)) => serde_json::from_str::<JsonValue>(s).ok(),
                _ => None,
            };

            let version = match row.get(2) {
                Some(Value::Int(v)) if *v > 0 && *v <= i64::from(u32::MAX) => Some(*v as u32),
                _ => None,
            };

            let skipped = match (payload, version) {
                (Some(payload), Some(version)) => {
                    let envelope = serde_json::json!({
                        "event_id": text_at(row, 0)?,
                        "event_type": event_type,
                        "schema_version": json_at(row, 2),
                        "occurred_at": json_at(row, 3),
                        "intent_id": json_at(row, 4),
                        "node_id": json_at(row, 5),
                        "ignorable": bool_at(row, 6),
                        "payload": payload,
                    });
                    match chain.decode(version, &envelope.to_string()) {
                        Ok(DecodedEvent::Event(_)) => false,
                        Ok(DecodedEvent::Skippable(_)) => true,
                        Err(e) => return Err(PersistError::Database(e.to_string())),
                    }
                }
                // payload 不可读，或版本号不可判定
                _ => true,
            };

            if skipped {
                skipped_records += 1;
                saw_skip = true;
            } else {
                decoded += 1;
            }

            if saw_skip && event_type == EventType::IntentCompleted.as_str() {
                return Err(PersistError::Database(
                    "可跳过事件之后存在 intent.completed，收口依据不成立".to_owned(),
                ));
            }
        }

        Ok(EventLogScan {
            decoded,
            skipped_records,
        })
    }
}
