//! 数据库打开、PRAGMA 与迁移框架。
//!
//! 连接对象是本 crate 私有字段。外部 crate 只能经 `Tx` 访问数据库，
//! 因此 §318 的事务约束在编译期成立。见 P0 设计第 4 节。

use crate::value::{params, Value};
use crate::{PersistError, Tx};
use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;

/// 一次迁移。`version` 递增且唯一，`sql` 允许包含多条语句。
#[derive(Debug, Clone)]
pub struct Migration {
    pub version: i64,
    pub name: &'static str,
    pub sql: &'static str,
}

impl Migration {
    pub const fn new(version: i64, name: &'static str, sql: &'static str) -> Self {
        Self { version, name, sql }
    }
}

/// P0 内置迁移：事件表与审计表。业务表由各层通过 `open_with` 追加。
pub fn builtin_migrations() -> Vec<Migration> {
    vec![
        Migration::new(
            1,
            "events",
            "CREATE TABLE events (
                event_id TEXT PRIMARY KEY,
                event_type TEXT NOT NULL,
                schema_version INTEGER NOT NULL,
                occurred_at INTEGER NOT NULL,
                intent_id TEXT,
                node_id TEXT,
                ignorable INTEGER NOT NULL DEFAULT 0,
                payload TEXT NOT NULL
            );
            CREATE INDEX idx_events_type ON events(event_type);
            CREATE INDEX idx_events_intent ON events(intent_id);
            CREATE INDEX idx_events_occurred ON events(occurred_at);",
        ),
        Migration::new(
            2,
            "audit_log",
            "CREATE TABLE audit_log (
                seq INTEGER PRIMARY KEY AUTOINCREMENT,
                kind TEXT NOT NULL,
                occurred_at INTEGER NOT NULL,
                payload TEXT NOT NULL,
                prev_hash TEXT NOT NULL,
                record_hash TEXT NOT NULL
            );
            CREATE INDEX idx_audit_kind ON audit_log(kind);",
        ),
    ]
}

pub struct Db {
    pub(crate) conn: Mutex<Connection>,
    migrations: Vec<Migration>,
}

impl Db {
    pub fn open(path: &Path) -> Result<Db, PersistError> {
        Db::open_with(path, builtin_migrations())
    }

    pub fn open_with(path: &Path, migrations: Vec<Migration>) -> Result<Db, PersistError> {
        let conn = Connection::open(path).map_err(crate::error::db)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=FULL;
             PRAGMA foreign_keys=ON;",
        )
        .map_err(crate::error::db)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                applied_at INTEGER NOT NULL
            );",
        )
        .map_err(crate::error::db)?;
        Ok(Db {
            conn: Mutex::new(conn),
            migrations,
        })
    }

    /// 按 version 升序应用未记录的迁移。每个迁移单独一个事务。
    /// 返回本次应用的迁移数。任一迁移失败时其后的迁移不应用。
    pub fn migrate(&self) -> Result<u32, PersistError> {
        let mut ordered = self.migrations.clone();
        ordered.sort_by_key(|m| m.version);

        let mut applied = 0u32;
        for m in ordered {
            let guard = self.conn.lock().unwrap_or_else(|e| e.into_inner());
            let done: i64 = guard
                .query_row(
                    "SELECT COUNT(*) FROM schema_migrations WHERE version = ?1",
                    [m.version],
                    |r| r.get(0),
                )
                .map_err(crate::error::db)?;
            if done > 0 {
                continue;
            }
            guard
                .execute_batch("BEGIN IMMEDIATE")
                .map_err(crate::error::db)?;
            let result = guard
                .execute_batch(m.sql)
                .and_then(|_| {
                    guard.execute(
                        "INSERT INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, ?3)",
                        rusqlite::params![m.version, m.name, now_millis()],
                    )
                });
            match result {
                Ok(_) => {
                    guard.execute_batch("COMMIT").map_err(crate::error::db)?;
                    applied += 1;
                }
                Err(e) => {
                    let _ = guard.execute_batch("ROLLBACK");
                    return Err(PersistError::Migration {
                        version: m.version,
                        message: e.to_string(),
                    });
                }
            }
        }
        Ok(applied)
    }

    /// 开启写事务。同一时刻只有一个事务，第二个未提交时调用会阻塞。
    pub fn begin(&self) -> Result<Tx<'_>, PersistError> {
        Tx::begin(self)
    }
}

pub(crate) fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub(crate) fn query_rows(conn: &Connection, sql: &str, p: &[Value]) -> Result<Vec<Vec<Value>>, PersistError> {
    let mut stmt = conn.prepare(sql).map_err(crate::error::db)?;
    let bound = params(p);
    let refs: Vec<&dyn rusqlite::ToSql> = bound.iter().map(|v| v as &dyn rusqlite::ToSql).collect();
    let mut rows = stmt.query(refs.as_slice()).map_err(crate::error::db)?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().map_err(crate::error::db)? {
        let n = row.as_ref().column_count();
        let mut one = Vec::with_capacity(n);
        for i in 0..n {
            let v = row.get_ref(i).map_err(crate::error::db)?;
            one.push(Value::from_ref(v));
        }
        out.push(one);
    }
    Ok(out)
}
