//! Effect Journal 的语义层：写入、推进、幂等键查询。
//!
//! 本模块内没有 SQL 字面量，也不自行编码枚举列——表的定义、列编码与行级读写
//! 都在 [`crate::persist`]。本模块只把它们按 journal 的语义串起来：
//! 写入与两次读各是一次转发，只有 [`advance`] 带判定次序（读→校验→写）。
//!
//! 与 P1 的 `continuum-graph` 对应物同形：纯函数的状态机在 [`crate::effect`]，
//! 「校验 → 写状态」的次序在这里（那边是 `transition_tx::apply_transition`）。

use crate::effect::{transition, Effect, EffectId, EffectState};
use crate::persist;
use continuum_persist::{PersistError, Tx};

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
pub fn record_planned(tx: &Tx<'_>, effect: &Effect) -> Result<(), PersistError> {
    persist::insert_row(tx, effect)
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
/// 次序固定为「读当前状态 → 经 [`transition`] 校验 → 写回」，与 P1 的
/// `apply_transition` 同形：表外迁移在写出任何一行之前就被拒，不留半成品。
///
/// 只使用调用方传入的 `Tx`，不自行开启或提交事务——提交由调用方负责。
///
/// # 返回 `Err` 之后调用方必须回滚该事务，不得提交
///
/// 本函数在校验通过后写状态列。若调用方把错误收集起来、最后统一 `commit()`，
/// 就会提交出一个状态不对的记录；同理，`effect.id` 不存在时本函数不写任何行，
/// 但调用方若把该错误与别的写入一并提交，那些写入仍然生效。原子性只在函数
/// 边界内成立，端到端要靠「见到 `Err` 就回滚」这条约定兜住。
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
    persist::set_state(tx, id, next, now)
}
