//! `effect` 表的迁移、枚举列编码与行级读写（设计下篇第 8 节）。
//!
//! 表定义、枚举列的编码辅助函数与行级读写放在同一文件。`tests/persist.rs` 以
//! 字面量断言落库取值，改编码会让那些字面量变红。语义层（写入、推进、幂等键查询）
//! 在 [`crate::journal`]，该文件内没有 SQL 字面量。
//!
//! 两个枚举列**不依赖 serde**：`EffectType` 与 `EffectState` 都不派生 serde
//! （Task 1 按裁定去掉，理由各处记在 `effect.rs` 的类型注释里）。落库编码一律
//! 小写、多词以 `_` 连接。
//!
//! 编码的产生点分两处，各是**唯一**的：
//! - `effect_type`：在类型自己身上——[`EffectType::as_str`] / [`EffectType::parse`]。
//!   本 crate 外的两个消费方（`continuum-policy` 的条件取值，以及计划里 Task 12 的
//!   驱动 `--effect` 解析）取用同一对函数，本文件不再自建表。
//! - `state`：在本文件的 `state_str` / `parse_state`，因为 `EffectState` 至今只有
//!   库列与审计 payload 两个消费方，两者同 crate。

use crate::effect::{Effect, EffectId, EffectState, EffectType};
use continuum_persist::{Migration, PersistError, Tx, Value, value::kind_name};
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

/// `effect.state` 列的唯一编码来源。穷尽无通配臂：给 `EffectState` 加变体时本函数
/// 编译不过，编码不会漏掉分支。
///
/// `pub(crate)`：`crate::journal` 的审计 payload 要写同一个串，故它必须取用本函数
/// 而非另抄一份字面量。crate 外不可见。
pub(crate) fn state_str(s: EffectState) -> &'static str {
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
/// 与 [`EffectType::parse`] 同理：表外取值报错，不取默认状态——把 `UNKNOWN`
/// 猜成 `COMMITTED`/`FAILED` 正是设计上篇第 7.4 节「不猜」所禁止的。
/// `tests/persist.rs` 的 `an_unknown_enum_column_value_is_rejected` 直接写入表外
/// 取值来钉住这条（两个枚举列各一行，故两条解码路径都有对照片）。
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
            Value::text(effect.effect_type.as_str()),
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
    Ok(Some(parse_state(&text_at(&row, 0)?)?))
}

/// 查出全部处于 `state` 的记录 id 及其 `updated_at`，按 id 升序。
///
/// 筛选条件经 [`state_str`] 取值，与写入侧的编码同源：在此手写字面量会让
/// 「写进去的串」与「查得中的串」成为两个产生点，改一处不会让另一处失败。
/// 列名与 SQL 也只出现在本文件（[`crate::journal`] 与 [`crate::recovery`]
/// 都不含 SQL 字面量）。
///
/// `updated_at` 一并取出：调用方（[`crate::recovery`]）要拿它作为推进时的
/// `now`，同一次查询取到可以少读一次。
pub(crate) fn rows_in_state(
    tx: &Tx<'_>,
    state: EffectState,
) -> Result<Vec<(EffectId, i64)>, PersistError> {
    tx.query(
        "SELECT id, updated_at FROM effect WHERE state = ?1 ORDER BY id",
        &[Value::text(state_str(state))],
    )?
    .iter()
    .map(|row| Ok((EffectId::new(text_at(row, 0)?), int_at(row, 1)?)))
    .collect()
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
        serde_json::from_str(&text_at(&row, 3)?).map_err(|e| PersistError::Database(e.to_string()))?;
    // 解码走枚举自己的 `parse`（编码的唯一来源）；表外取值在此转成库层错误，
    // 不取默认类型——理由见 `EffectType::parse` 的文档。
    let raw_type = text_at(&row, 1)?;
    let effect_type = EffectType::parse(&raw_type)
        .ok_or_else(|| PersistError::Database(format!("未知 EffectType: {raw_type}")))?;
    Ok(Some(Effect {
        id: EffectId::new(text_at(&row, 0)?),
        effect_type,
        target: text_at(&row, 2)?,
        parameters,
        authorization: text_at(&row, 4)?,
        idempotency_key: text_at(&row, 5)?,
        state: parse_state(&text_at(&row, 6)?)?,
        planned_at: int_at(&row, 7)?,
        updated_at: int_at(&row, 8)?,
    }))
}

/// 按下标取文本列。列类型不符报 [`PersistError::ColumnType`]，
/// 形态与 `continuum-persist` 的 `Tx::audit_records` 同（那里的 `text_at`）。
/// 列缺失（`None`）也算类型不符，`actual` 记 `"missing"`。
fn text_at(row: &[Value], index: usize) -> Result<String, PersistError> {
    match row.get(index) {
        Some(Value::Text(s)) => Ok(s.clone()),
        Some(other) => Err(PersistError::ColumnType {
            index,
            actual: kind_name(other),
        }),
        None => Err(PersistError::ColumnType {
            index,
            actual: "missing",
        }),
    }
}

/// 按下标取整数列，报错形态同 [`text_at`]。
fn int_at(row: &[Value], index: usize) -> Result<i64, PersistError> {
    match row.get(index) {
        Some(Value::Int(i)) => Ok(*i),
        Some(other) => Err(PersistError::ColumnType {
            index,
            actual: kind_name(other),
        }),
        None => Err(PersistError::ColumnType {
            index,
            actual: "missing",
        }),
    }
}
