//! 与 rusqlite 解耦的参数与结果值。

use crate::error::PersistError;
use rusqlite::types::{Value as SqlValue, ValueRef};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Int(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
}

impl Value {
    pub fn text(s: impl Into<String>) -> Self {
        Value::Text(s.into())
    }

    pub(crate) fn to_sql(&self) -> SqlValue {
        match self {
            Value::Null => SqlValue::Null,
            Value::Int(i) => SqlValue::Integer(*i),
            Value::Real(f) => SqlValue::Real(*f),
            Value::Text(s) => SqlValue::Text(s.clone()),
            Value::Blob(b) => SqlValue::Blob(b.clone()),
        }
    }

    pub(crate) fn from_ref(r: ValueRef<'_>) -> Self {
        match r {
            ValueRef::Null => Value::Null,
            ValueRef::Integer(i) => Value::Int(i),
            ValueRef::Real(f) => Value::Real(f),
            ValueRef::Text(t) => Value::Text(String::from_utf8_lossy(t).into_owned()),
            ValueRef::Blob(b) => Value::Blob(b.to_vec()),
        }
    }
}

pub(crate) fn params(values: &[Value]) -> Vec<SqlValue> {
    values.iter().map(Value::to_sql).collect()
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::Text(s.to_owned())
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::Text(s)
    }
}

impl From<i64> for Value {
    fn from(i: i64) -> Self {
        Value::Int(i)
    }
}

/// 供调用方在测试与断言中使用。
pub fn as_text(v: &Value) -> Result<&str, PersistError> {
    match v {
        Value::Text(s) => Ok(s),
        other => Err(PersistError::ColumnType {
            index: 0,
            actual: kind_name(other),
        }),
    }
}

pub fn kind_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Int(_) => "int",
        Value::Real(_) => "real",
        Value::Text(_) => "text",
        Value::Blob(_) => "blob",
    }
}
