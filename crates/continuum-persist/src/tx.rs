//! 事务 API。Task 7 补全。

use crate::{Db, PersistError, Value};

// Task 7 会加入 `done: bool` 字段与 Drop 实现。本 task 不含它们：
// 该字段在此无人读取，提前声明会产生 dead_code 警告。
pub struct Tx<'a> {
    pub(crate) guard: std::sync::MutexGuard<'a, rusqlite::Connection>,
}

impl<'a> Tx<'a> {
    pub(crate) fn begin(db: &'a Db) -> Result<Tx<'a>, PersistError> {
        let guard = db.conn.lock().unwrap_or_else(|e| e.into_inner());
        guard
            .execute_batch("BEGIN IMMEDIATE")
            .map_err(crate::error::db)?;
        Ok(Tx { guard })
    }

    pub fn query(&self, sql: &str, params: &[Value]) -> Result<Vec<Vec<Value>>, PersistError> {
        crate::db::query_rows(&self.guard, sql, params)
    }

    pub fn execute(&self, sql: &str, params: &[Value]) -> Result<u64, PersistError> {
        use crate::value::params as to_sql;
        let bound = to_sql(params);
        let refs: Vec<&dyn rusqlite::ToSql> =
            bound.iter().map(|v| v as &dyn rusqlite::ToSql).collect();
        self.guard
            .execute(sql, refs.as_slice())
            .map(|n| n as u64)
            .map_err(crate::error::db)
    }
}
