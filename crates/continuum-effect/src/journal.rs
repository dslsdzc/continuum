//! Effect Journal 的语义层：写入、推进、幂等键查询，以及随之落下的审计记录。
//!
//! 本模块内没有 SQL 字面量，也不自行编码枚举列——表的定义、列编码与行级读写
//! 都在 [`crate::persist`]。本模块只把它们按 journal 的语义串起来：
//! 两次读各是一次转发，写入与 [`advance`] 还各写一条审计（[`audit`]）。
//!
//! 与 P1 的 `continuum-graph` 对应物同形：纯函数的状态机在 [`crate::effect`]，
//! 「校验 → 写状态」的次序在这里（那边是 `transition_tx::apply_transition`）。

use crate::effect::{transition, Effect, EffectId, EffectState};
use crate::persist;
use continuum_events::AuditKind;
use continuum_persist::{PersistError, Tx, Value};
use serde_json::json;

/// 记一条审计。**与状态变更同一个 `Tx`**：设计上篇第 7.3 节要求二者在同一事务内
/// 提交。本 crate 的两个写入函数（[`record_planned`] / [`advance`]）是 `effect`
/// 表在 crate 外的仅有两个写入入口（`crate::persist` 的行级函数都是 `pub(crate)`），
/// 审计落在它们内部即满足该要求——调用方想改状态就只能经这两个函数，绕不开。
/// crate 内若新增第三个写入路径，须一并落审计：这一层由 `tests/persist.rs` 的
/// 审计计数断言逐函数钉住，新增路径不会自动被覆盖。
///
/// `kind` 固定取 `ExternalEffects`：上篇的归类里，不可回滚的外部操作归它，效应
/// 正是这一类。**不新增 `AuditKind` 变体**（`AuditKind::ALL` 是长度写死的数组，
/// 扩它会牵动 P0 的黄金向量与 serde rename 用例）。
///
/// `occurred_at` 由调用方给出（登记取 `planned_at`、推进取 `now`），本 crate 不取
/// 时钟——与工作区的 `Gate::audit` 同一条理由。
///
/// payload 用 JSON 对象而非工作区那样的一行文本：那里的理由是「该 crate 至今没有
/// serde_json」，本条不适用，而效应审计的字段（效应 id、类型、目标、迁移两端）要
/// 供后续对账取用，结构化比文本稳。取值仍经 [`crate::persist`] 的编码函数，
/// 与 `effect` 表里的列同源。
fn audit(tx: &Tx<'_>, occurred_at: i64, payload: serde_json::Value) -> Result<(), PersistError> {
    tx.append_audit(
        AuditKind::ExternalEffects,
        occurred_at,
        Value::text(serde_json::to_string(&payload).expect("payload 由 json! 构造，必可序列化")),
    )?;
    Ok(())
}

/// 写入一条记录（设计上篇第 7.3 节写入顺序的第一步）。
///
/// 记录按 `effect` 所携带的状态整条落库；`PLANNED` 由调用方构造（该态含
/// `authorization` 与 `idempotency_key`，二者在第一步就要有）。后续的
/// `AUTHORIZED` / `EXECUTING` 经 [`advance`] 推进，不重写整条记录。
///
/// 本函数不做状态校验：它原样写入 `effect.state`，状态机的把关在 [`advance`]。
///
/// 同键的第二次写入被库层的唯一索引拒绝并返回 `Err`（设计上篇第 7.6 节）。
/// 该拒绝的具体形态：`PersistError` 没有独立的冲突变体，而 `Tx::execute` 把
/// rusqlite 的错误码抹成了字符串，故调用方只能在 `PersistError::Database` 内
/// 按消息识别冲突。
///
/// 登记与对应的审计记录在**同一个 `Tx`** 内写（设计上篇第 7.3 节），次序是先
/// 插入后记审计。被唯一索引拒绝时审计不写，且本函数已写入的内容由调用方回滚
/// ——契约同 [`advance`]：**返回 `Err` 之后不得提交**。
pub fn record_planned(tx: &Tx<'_>, effect: &Effect) -> Result<(), PersistError> {
    // 先插入后记审计：写入被唯一索引拒绝时不留下「登记了却没这条效应」的审计。
    persist::insert_row(tx, effect)?;
    audit(
        tx,
        effect.planned_at,
        json!({
            "effect_id": effect.id.as_str(),
            // 类型与状态取枚举自己的编码（`EffectType::as_str` / `state_str`），
            // 与 `effect` 表的列同一个串：此处手写字面量会让同一事实有了第二份表示。
            "effect_type": effect.effect_type.as_str(),
            "target": effect.target,
            "idempotency_key": effect.idempotency_key,
            "state": persist::state_str(effect.state),
        }),
    )
}

/// 读一条记录。不存在返回 `None`。
pub fn load_effect(tx: &Tx<'_>, id: &EffectId) -> Result<Option<Effect>, PersistError> {
    persist::load_row_by_id(tx, id)
}

/// 按幂等键读一条记录。不存在返回 `None`。
///
/// 同键的第二次 `PLANNED` 被 [`record_planned`] 拒绝后，调用方经本函数取到
/// 既有记录（设计上篇第 7.6 节）。
pub fn find_by_idempotency_key(tx: &Tx<'_>, key: &str) -> Result<Option<Effect>, PersistError> {
    persist::load_row_by_key(tx, key)
}

/// 推进一条记录的状态，并把 `updated_at` 置为 `now`。
///
/// 次序固定为「读当前状态 → 经 [`transition`] 校验 → 写回 → 记审计」，与 P1 的
/// `apply_transition` 同形：表外迁移在写出任何一行之前就被拒，既不改状态也不
/// 留审计。状态变更与对应审计在**同一个 `Tx`** 内写（设计上篇第 7.3 节）。
///
/// 只使用调用方传入的 `Tx`，不自行开启或提交事务——提交由调用方负责。
///
/// # 返回 `Err` 之后调用方必须回滚该事务，不得提交
///
/// 本函数在校验通过后写状态列、再记审计。审计写入失败（`events`/`audit_log`
/// 写不进去）时返回 `Err`，而**前一步的状态写入已经留在事务里**。调用方若把
/// 错误收集起来、最后统一 `commit()`，就会提交出一个状态已变却无审计的记录；
/// 同理，`effect.id` 不存在时本函数不写任何行，但调用方若把该错误与别的写入
/// 一并提交，那些写入仍然生效。原子性只在函数边界内成立，端到端要靠「见到
/// `Err` 就回滚」这条约定兜住。
///
/// 本函数不代劳回滚：`Tx` 没有「回滚到保存点」这类接口，要回滚只能把 `Tx`
/// 丢弃，而那会连带销毁调用方手里的句柄——回滚的决定只能由调用方做。
///
/// # 错误形态
///
/// 失败都经 `PersistError::Database` 承载：id 不存在、迁移被 [`transition`]
/// 拒绝，这两种在 `tests/persist.rs` 各有用例；第三种是库层写入失败，由 `?`
/// 直接透传。本 crate 手上只有这一个能承载业务判定的变体——其余四个各指
/// 迁移、参数、列类型与事务丢弃（见 [`record_planned`] 的同一条说明），
/// 故判定细节在消息里。
pub fn advance(
    tx: &Tx<'_>,
    id: &EffectId,
    to: EffectState,
    now: i64,
) -> Result<(), PersistError> {
    let Some(from) = persist::state_of(tx, id)? else {
        return Err(PersistError::Database(format!("不存在 id 为 {id} 的效应记录")));
    };
    let next = transition(from, to).map_err(|e| PersistError::Database(e.to_string()))?;
    // 先写状态后记审计：校验不通过时不留下审计（表外迁移既不落库也不入审计）。
    persist::set_state(tx, id, next, now)?;
    audit(
        tx,
        now,
        json!({
            "effect_id": id.as_str(),
            "from": persist::state_str(from),
            "to": persist::state_str(next),
        }),
    )
}
