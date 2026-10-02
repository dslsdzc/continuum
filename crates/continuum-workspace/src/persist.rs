//! Workspace 记录的落库（§317）。
//!
//! 「这个 Intent 用了哪个后端、工作区落在哪」写在 `workspace` 表里，重启后由
//! [`load_workspace`] 取回，不必重新探测后端——探测的判据（Base 是否为 Git 仓库）
//! 在重启之间可能已经改变（例如用户后来才 `git init`），按新判据猜出的后端与
//! 磁盘上实际存在的那个工作区对不上。

use crate::backend::WorkspaceBackend;
use crate::ids::IntentId;
use continuum_persist::{Migration, PersistError, Tx, Value};
use std::path::{Path, PathBuf};

/// P2 的 workspace 表迁移。
///
/// 编号取 30：1、2 是 `continuum-persist` 的内建迁移，10 与 20 分别是 P1 的
/// `p1_artifact` 与 `p1_graph`。`schema_migrations` 按 version 记录已应用项，
/// 编号重复会让后一条被当作「已应用」而跳过——两条迁移互相覆盖，且不报错。
pub fn p2_workspace_migrations() -> Vec<Migration> {
    vec![Migration::new(
        30,
        "p2_workspace",
        "CREATE TABLE workspace (
            intent_id  TEXT PRIMARY KEY,
            backend    TEXT NOT NULL,
            path       TEXT NOT NULL,
            base_path  TEXT NOT NULL,
            created_at INTEGER NOT NULL
        );",
    )]
}

/// 一条 Workspace 记录：某个 Intent 的工作区落在哪、用的哪个后端。
///
/// `path` 是 Task Workspace 的根，`base_path` 是创建时的 Base 根。两者都记，
/// 是因为放弃这个工作区要同时用到：worktree 后端经 `-C <base>` 调 git，
/// 而 Task 根的父目录 `<base>/.ai/worktrees` 由二者推出。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceRecord {
    pub intent_id: IntentId,
    pub backend: WorkspaceBackend,
    pub path: PathBuf,
    pub base_path: PathBuf,
    /// 创建时刻，Unix 毫秒。
    pub created_at: i64,
}

/// 写入一条 Workspace 记录。
///
/// 同 `intent_id` 已存在时返回数据库错误，**不覆盖**（与 `save_artifact` 的
/// §241 同一条规矩）：记录描述的是磁盘上实际存在的那个工作区，静默改写会让
/// 「记录」与「实物」分叉而无从察觉。同一 Intent 重新创建工作区须先删掉旧记录
/// ——本函数不提供该能力，删除不是本层的职责。
pub fn save_workspace(tx: &Tx<'_>, record: &WorkspaceRecord) -> Result<(), PersistError> {
    tx.execute(
        "INSERT INTO workspace (intent_id, backend, path, base_path, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        &[
            Value::text(record.intent_id.as_str()),
            Value::text(backend_str(record.backend)),
            Value::text(path_str(&record.path)?),
            Value::text(path_str(&record.base_path)?),
            Value::Int(record.created_at),
        ],
    )?;
    Ok(())
}

/// 读回一个 Intent 的 Workspace 记录。没有记录时返回 `Ok(None)`。
///
/// `backend` 列的取值必须恰是 [`backend_str`] 给出的编码之一，否则返回 `Err`：
/// 回退成某个默认后端会让放弃时走错回收路径，留下另一后端的残留而不报错。
pub fn load_workspace(
    tx: &Tx<'_>,
    intent_id: &IntentId,
) -> Result<Option<WorkspaceRecord>, PersistError> {
    let rows = tx.query(
        "SELECT intent_id, backend, path, base_path, created_at
         FROM workspace WHERE intent_id = ?1",
        &[Value::text(intent_id.as_str())],
    )?;
    let Some(row) = rows.into_iter().next() else {
        return Ok(None);
    };
    Ok(Some(WorkspaceRecord {
        intent_id: IntentId::new(text(&row[0])?),
        backend: parse_backend(&text(&row[1])?)?,
        path: PathBuf::from(text(&row[2])?),
        base_path: PathBuf::from(text(&row[3])?),
        created_at: int(&row[4])?,
    }))
}

/// `Path` → 落库文本。
///
/// 非 UTF-8 的路径**返回 `Err`**，不经 `to_string_lossy` 落库：替换字符写进库后
/// 再读回就指向另一个路径，而错误要在读回时才显形，那时已经无从判断原值是什么。
fn path_str(path: &Path) -> Result<String, PersistError> {
    path.to_str().map(str::to_owned).ok_or_else(|| {
        PersistError::Database(format!("路径不是合法 UTF-8，无法落库：{}", path.display()))
    })
}

/// `workspace.backend` 列的唯一编码来源。
///
/// 规则：小写，多词以 `_` 连接。**显式给出，不由 [`WorkspaceBackend`] 的 serde
/// 表示或 `Debug` 推出**——落库编码与类型的 serde 表示是两件事，同一事实两份
/// 写法即会分叉。调用方不得自行拼该列的取值字面量。
///
/// 与 `continuum_graph::persist` 的 `state_str` 同形：本 crate 内该列的读写
/// 都经本函数与其逆 [`parse_backend`]。
fn backend_str(backend: WorkspaceBackend) -> &'static str {
    match backend {
        WorkspaceBackend::Worktree => "worktree",
        WorkspaceBackend::Overlay => "overlay",
    }
}

/// [`backend_str`] 的逆。未知取值返回 `Err`，不落回默认后端。
fn parse_backend(s: &str) -> Result<WorkspaceBackend, PersistError> {
    Ok(match s {
        "worktree" => WorkspaceBackend::Worktree,
        "overlay" => WorkspaceBackend::Overlay,
        other => {
            return Err(PersistError::Database(format!(
                "未知 WorkspaceBackend: {other}"
            )))
        }
    })
}

fn text(v: &Value) -> Result<String, PersistError> {
    match v {
        Value::Text(s) => Ok(s.clone()),
        other => Err(PersistError::Database(format!("列应为文本，实际 {other:?}"))),
    }
}

fn int(v: &Value) -> Result<i64, PersistError> {
    match v {
        Value::Int(i) => Ok(*i),
        other => Err(PersistError::Database(format!("列应为整数，实际 {other:?}"))),
    }
}
