//! `effect` 表的迁移、枚举列编码与行级读写（设计下篇第 8 节）。
//!
//! 表定义、两个枚举列的编码辅助函数与行级读写放在同一文件：两个枚举列的编码
//! 在库侧只有一个产生点（[`effect_type_str`] / [`state_str`]），解码也只有一处。
//! `tests/persist.rs` 另以字面量断言落库取值，改编码会让那些字面量变红。
//! 语义层（写入、推进、幂等键查询）在 [`crate::journal`]，该文件内没有 SQL 字面量。
//!
//! 两个枚举列**不依赖 serde**：`EffectType` / `EffectState` 的 serde 表示
//! （`SCREAMING_SNAKE_CASE`）与落库编码（小写）是两件事，落库只走本文件的
//! `effect_type_str` / `state_str` 及其逆。

use crate::effect::{Effect, EffectId, EffectState, EffectType};
use continuum_persist::{Migration, PersistError, Tx, Value};
use serde_json::Value as JsonValue;

/// 本 crate 在 `effect` 表上注册的迁移。
///
/// 编号 40：1、2 是 `continuum-persist` 的内建（事件、审计），20 是 P1 的图，
/// 30 是 P2 上篇的工作区。50/60/100 只出现在测试与 bin 里，不是已占用的业务编号。
pub fn p2_effect_migrations() -> Vec<Migration> {
    vec![Migration::new(
        40,
        "p2_effect",
        "CREATE TABLE effect (
            id              TEXT PRIMARY KEY,
            effect_type     TEXT NOT NULL,
            target          TEXT NOT NULL,
            parameters      TEXT NOT NULL,
            authorization   TEXT NOT NULL,
            idempotency_key TEXT NOT NULL,
            state           TEXT NOT NULL,
            planned_at      INTEGER NOT NULL,
            updated_at      INTEGER NOT NULL
        );
        CREATE UNIQUE INDEX idx_effect_idempotency ON effect(idempotency_key);",
    )]
}

/// `effect.effect_type` 列的唯一编码来源。
///
/// 枚举是封闭的且本 match 穷尽无通配臂：给 `EffectType` 加变体时本函数编译不过，
/// 编码不会漏掉分支。**解码侧没有这层保护**——[`parse_effect_type`] 是同一个枚举
/// 的逆，加变体时它照旧编译，须由作者一并补上；漏补的后果是该变体写得进、读不回。
fn effect_type_str(t: EffectType) -> &'static str {
    match t {
        EffectType::SendEmail => "send_email",
        EffectType::PushBranch => "push_branch",
        EffectType::Publish => "publish",
        EffectType::DeleteRemote => "delete_remote",
        EffectType::Charge => "charge",
        EffectType::Deploy => "deploy",
    }
}

/// `effect.effect_type` 列的唯一解码来源，与 [`effect_type_str`] 互逆。
///
/// 取值不在表内即报错，不取默认类型：效应类型是策略裁决的依据，取默认会让
/// 策略表漏判。`tests/persist.rs` 的 `an_unknown_enum_column_value_is_rejected`
/// 直接写入表外取值来钉住这条。
fn parse_effect_type(s: &str) -> Result<EffectType, PersistError> {
    Ok(match s {
        "send_email" => EffectType::SendEmail,
        "push_branch" => EffectType::PushBranch,
        "publish" => EffectType::Publish,
        "delete_remote" => EffectType::DeleteRemote,
        "charge" => EffectType::Charge,
        "deploy" => EffectType::Deploy,
        other => {
            return Err(PersistError::Database(format!("未知 EffectType: {other}")))
        }
    })
}

/// `effect.state` 列的唯一编码来源。穷尽无通配臂，理由同 [`effect_type_str`]。
fn state_str(s: EffectState) -> &'static str {
    match s {
        EffectState::Planned => "planned",
        EffectState::Authorized => "authorized",
        EffectState::Executing => "executing",
        EffectState::Committed => "committed",
        EffectState::Failed => "failed",
        EffectState::RolledBack => "rolled_back",
        EffectState::Unknown => "unknown",
    }
}

/// `effect.state` 列的唯一解码来源，与 [`state_str`] 互逆。
///
/// 与 [`parse_effect_type`] 同理：表外取值报错，不取默认状态——把 `UNKNOWN`
/// 猜成 `COMMITTED`/`FAILED` 正是设计上篇第 7.4 节「不猜」所禁止的。用例同上。
fn parse_state(s: &str) -> Result<EffectState, PersistError> {
    Ok(match s {
        "planned" => EffectState::Planned,
        "authorized" => EffectState::Authorized,
        "executing" => EffectState::Executing,
        "committed" => EffectState::Committed,
        "failed" => EffectState::Failed,
        "rolled_back" => EffectState::RolledBack,
        "unknown" => EffectState::Unknown,
        other => {
            return Err(PersistError::Database(format!("未知 EffectState: {other}")))
        }
    })
}

/// 插入一条记录。调用方（[`crate::journal::record_planned`]）已确保 `effect`
/// 是完整的一条记录；本函数按其所携带的状态整条写入。
///
/// 裸 `INSERT`，不 `OR REPLACE`：同键的第二次写入由唯一索引
/// `idx_effect_idempotency` 拒绝，判据在库层而非调用方自查（设计上篇第 7.6 节）。
pub(crate) fn insert_row(tx: &Tx<'_>, effect: &Effect) -> Result<(), PersistError> {
    let parameters = serde_json::to_string(&effect.parameters)
        .map_err(|e| PersistError::Database(format!("parameters 不可序列化为 JSON: {e}")))?;
    tx.execute(
        "INSERT INTO effect
           (id, effect_type, target, parameters, authorization,
            idempotency_key, state, planned_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        &[
            Value::text(effect.id.as_str()),
            Value::text(effect_type_str(effect.effect_type)),
            Value::text(effect.target.clone()),
            Value::text(parameters),
            Value::text(effect.authorization.clone()),
            Value::text(effect.idempotency_key.clone()),
            Value::text(state_str(effect.state)),
            Value::Int(effect.planned_at),
            Value::Int(effect.updated_at),
        ],
    )?;
    Ok(())
}

/// 只读当前状态。记录不存在返回 `None`。
///
/// 与 [`set_state`] 配对使用：`journal::advance` 先经本函数取到当前状态、
/// 判定迁移合法后才写，故这里与那里的状态列编解码同源。
pub(crate) fn state_of(tx: &Tx<'_>, id: &EffectId) -> Result<Option<EffectState>, PersistError> {
    let rows = tx.query(
        "SELECT state FROM effect WHERE id = ?1",
        &[Value::text(id.as_str())],
    )?;
    let Some(row) = rows.into_iter().next() else {
        return Ok(None);
    };
    Ok(Some(parse_state(&text(&row[0])?)?))
}

/// 改写状态列并把 `updated_at` 置为 `now`。其它列不动。
///
/// 本函数只写，不判定迁移是否合法：判定是 [`crate::journal::advance`] 的职责，
/// 且必须发生在调用本函数**之前**，否则表外迁移会落库。库侧对本函数的调用点
/// 在 [`crate::journal::advance`]。
pub(crate) fn set_state(
    tx: &Tx<'_>,
    id: &EffectId,
    to: EffectState,
    now: i64,
) -> Result<(), PersistError> {
    tx.execute(
        "UPDATE effect SET state = ?2, updated_at = ?3 WHERE id = ?1",
        &[
            Value::text(id.as_str()),
            Value::text(state_str(to)),
            Value::Int(now),
        ],
    )?;
    Ok(())
}

/// 按 id 读整条记录。不存在返回 `None`。
pub(crate) fn load_row_by_id(
    tx: &Tx<'_>,
    id: &EffectId,
) -> Result<Option<Effect>, PersistError> {
    rows_to_effect(tx.query(
        "SELECT id, effect_type, target, parameters, authorization,
                idempotency_key, state, planned_at, updated_at
         FROM effect WHERE id = ?1",
        &[Value::text(id.as_str())],
    )?)
}

/// 按幂等键读整条记录。不存在返回 `None`。
///
/// 列清单与 [`load_row_by_id`] 逐字相同，行映射共用 [`rows_to_effect`]：
/// 两条查询路径不会因列序不同而各错一处。
pub(crate) fn load_row_by_key(tx: &Tx<'_>, key: &str) -> Result<Option<Effect>, PersistError> {
    rows_to_effect(tx.query(
        "SELECT id, effect_type, target, parameters, authorization,
                idempotency_key, state, planned_at, updated_at
         FROM effect WHERE idempotency_key = ?1",
        &[Value::text(key)],
    )?)
}

fn rows_to_effect(rows: Vec<Vec<Value>>) -> Result<Option<Effect>, PersistError> {
    let Some(row) = rows.into_iter().next() else {
        return Ok(None);
    };
    let parameters: JsonValue =
        serde_json::from_str(&text(&row[3])?).map_err(|e| PersistError::Database(e.to_string()))?;
    Ok(Some(Effect {
        id: EffectId::new(text(&row[0])?),
        effect_type: parse_effect_type(&text(&row[1])?)?,
        target: text(&row[2])?,
        parameters,
        authorization: text(&row[4])?,
        idempotency_key: text(&row[5])?,
        state: parse_state(&text(&row[6])?)?,
        planned_at: int(&row[7])?,
        updated_at: int(&row[8])?,
    }))
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
