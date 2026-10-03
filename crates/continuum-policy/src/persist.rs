//! `policy` 表的迁移、枚举列编码与行级读写（设计下篇第 8 节）。
//!
//! 表定义与行级读写放在同一文件（设计下篇第 8 节：每张表的读写函数与该表的定义
//! 放在同一 crate）。`tests/persist.rs` 以字面量断言落库取值，改编码会让那些字面量变红。
//!
//! 三个枚举列**不依赖 serde**：`Level` / `Decision` / `Scope` 都不派生 serde
//! （见它们各自的类型注释）。落库编码一律小写、多词以 `_` 连接，**编码本体在类型
//! 自己身上**（[`Level::as_str`] / [`Level::parse`] 等），本文件只做委托与
//! [`PersistError`] 适配——与 `continuum-effect` 的 `EffectType` 同形。编码放在
//! 定义判别值的那几行旁边是有理由的：`Level` 的判别值是 1–6 的序号，而落库取名字
//! 而非数字，这条判断只有在判别值跟前才看得见。
//!
//! 本 crate 不含其它 I/O：数据库连接在 `continuum-persist` 内私有，此处只经 [`Tx`] 访问。

use continuum_persist::{Migration, PersistError, Tx, Value, value::kind_name};
use serde_json::Value as JsonValue;

use crate::rule::{Condition, Decision, Level, Policy, Scope};

/// 本 crate 在 `policy` 表上注册的迁移。
///
/// 编号 41：1、2 是 `continuum-persist` 的内建（事件、审计），10 是 P1 的 artifact，
/// 20 是 P1 的图，30 是 P2 上篇的工作区，40 是本子项目 `continuum-effect` 的 `effect`
/// 表——本表与它同属一个子项目，取紧邻的 41。50/60/100 只出现在测试与 bin 里，
/// 不是已占用的业务编号。
///
/// `level` 是带数字序的枚举，落库仍取名字：数字编码会让「加一级」变成破坏性变更
/// （在中间插入一层时既有行的数字要全部改写），理由详见 [`Level::as_str`]。
pub fn p2_policy_migrations() -> Vec<Migration> {
    vec![Migration::new(
        41,
        "p2_policy",
        "CREATE TABLE policy (
            id        TEXT PRIMARY KEY,
            level     TEXT NOT NULL,
            scope     TEXT NOT NULL,
            condition TEXT NOT NULL,
            decision  TEXT NOT NULL
        );",
    )]
}

/// 插入一条规则。
///
/// 裸 `INSERT`，不 `OR REPLACE`：同 id 的第二次写入由主键拒绝，判据在库层而非
/// 调用方自查（与 `continuum-effect` 的 `insert_row` 同一条成法）。这里比那边更重
/// ——`OR REPLACE` 会静默覆盖用户自己定的持久策略，而策略是被安全语义依赖的输入。
pub fn save_policy(tx: &Tx<'_>, id: &str, policy: &Policy) -> Result<(), PersistError> {
    let condition = serde_json::to_string(&policy.condition.to_json())
        .map_err(|e| PersistError::Database(format!("condition 不可序列化为 JSON: {e}")))?;
    tx.execute(
        "INSERT INTO policy (id, level, scope, condition, decision)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        &[
            Value::text(id),
            Value::text(policy.level.as_str()),
            Value::text(policy.scope.as_str()),
            Value::text(condition),
            Value::text(policy.decision.as_str()),
        ],
    )?;
    Ok(())
}

/// 读出全部规则，按 id 升序（次序稳定，不依赖表的物理顺序）。
///
/// 条件列**重新经 [`Condition::parse`]**：库里被手工改坏的条件不该静默变成
/// 「永不匹配」——那会让人以为策略生效了而实际没有（设计下篇第 5.1 节）。
/// 解析失败即返回 `Err`，且错误信息指名是哪个 id 的哪一类写错。
///
/// 三个枚举列的解码走枚举自己的 `parse`（编码的唯一来源）；表外取值在此转成库层
/// 错误，不取默认——把未知层级猜成第 5 级会让裁决判错（与 `EffectType::parse`
/// 的文档同一条理由）。
pub fn load_policies(tx: &Tx<'_>) -> Result<Vec<Policy>, PersistError> {
    tx.query(
        "SELECT id, level, scope, condition, decision FROM policy ORDER BY id",
        &[],
    )?
    .iter()
    .map(|row| row_to_policy(row))
    .collect()
}

fn row_to_policy(row: &[Value]) -> Result<Policy, PersistError> {
    let id = text_at(row, 0)?;

    let raw_level = text_at(row, 1)?;
    let level = Level::parse(&raw_level)
        .ok_or_else(|| PersistError::Database(format!("未知 Level: {raw_level}")))?;

    let raw_scope = text_at(row, 2)?;
    let scope = Scope::parse(&raw_scope)
        .ok_or_else(|| PersistError::Database(format!("未知 Scope: {raw_scope}")))?;

    let condition = parse_condition(&id, &text_at(row, 3)?)?;

    let raw_decision = text_at(row, 4)?;
    let decision = Decision::parse(&raw_decision)
        .ok_or_else(|| PersistError::Database(format!("未知 Decision: {raw_decision}")))?;

    Ok(Policy {
        level,
        condition,
        decision,
        scope,
    })
}

/// 读回一列条件：先解 JSON，再经 [`Condition::parse`]。
///
/// 两步分开报错，不把 JSON 解析失败降级成 `Value::Null`（那会连同「结构不符」一起
/// 报出，错误来源就分不清了）。`PersistError` 没有独立的条件解析变体，故两类都落
/// 在 `Database` 内，由消息指名 id 与 [`crate::PolicyError`] 的具体变体。
fn parse_condition(id: &str, raw: &str) -> Result<Condition, PersistError> {
    let value: JsonValue = serde_json::from_str(raw).map_err(|e| {
        PersistError::Database(format!("policy {id} 的 condition 列不是合法 JSON: {e}"))
    })?;
    Condition::parse(&value).map_err(|e| {
        PersistError::Database(format!("policy {id} 的 condition 列不是合法谓词: {e}"))
    })
}

/// 按下标取文本列。列类型不符报 [`PersistError::ColumnType`]，
/// 形态与 `continuum-effect` 的同名函数一致。列缺失（`None`）也算类型不符，
/// `actual` 记 `"missing"`。
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
