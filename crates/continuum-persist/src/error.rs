//! 持久化错误。错误信息用字符串承载，不外泄 rusqlite 类型，
//! 这样 `continuum-persist` 的使用者不需要依赖 rusqlite。

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PersistError {
    #[error("数据库错误: {0}")]
    Database(String),
    #[error("迁移 {version} 失败: {message}")]
    Migration { version: i64, message: String },
    #[error("参数类型不匹配: 第 {index} 个参数为 {actual}")]
    ParamType { index: usize, actual: &'static str },
    #[error("结果类型不匹配: 第 {index} 列为 {actual}")]
    ColumnType { index: usize, actual: &'static str },
    #[error("事务未提交即被丢弃")]
    TransactionDropped,
}

pub(crate) fn db(e: rusqlite::Error) -> PersistError {
    PersistError::Database(e.to_string())
}
