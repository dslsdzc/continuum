//! §223 状态迁移的落库版本（§5.4）：状态迁移与对应 `Event` 在同一事务内写入
//! （`§318`）。
//!
//! 纯判定（§5.3 的迁移表）在 `intent.rs` 的 [`IntentStatus::transition`]；本模块把
//! 「读当前状态 → 判定 → 写回 → 写事件」绑到调用方的 `Tx` 上。形状部分参照 P1 的
//! `apply_transition`（`crates/continuum-graph/src/transition_tx.rs`，下称 P1 的
//! 迁移函数）：
//! 本模块不持有 `Db`，只能经调用方的 `Tx` 写入，因此不存在「写完状态自己提交、
//! 事件留在另一个事务」的写法。
//!
//! # 事件面为什么经 `continuum_persist` 取
//!
//! [`Event`] / [`EventType`] / [`default_chain`] 是 `continuum-events` 的类型，
//! 而本 crate 不登记指向它的依赖边（设计 §2.2：本层经 `continuum-persist` 的
//! `Tx::append_event` 写事件，「故不新增边」），故经 persist 的转出取
//! （`crates/continuum-persist/src/lib.rs` 的转出注释）。

use continuum_persist::{Event, EventType, Tx, Value, default_chain};

use crate::error::IntentError;
use crate::ids::IntentId;
use crate::intent::IntentStatus;
use crate::persist::load_intent;

/// 状态迁移与对应 `Event` **在同一事务**内写入（照 P1 的 `apply_transition` 形状）。
/// **本函数不自己 `commit`**（P1 的纪律：返回 `Err` 之后调用方必须回滚、不得提交）。
///
/// # `occurred_at` 与 `event_id`：签名比任务简报的速写多两个参数（**申报**）
///
/// 简报 Step 3 的速写是 `(tx, id, to) -> Result<(), IntentError>`。落地时补上这两个，
/// 理由三条，缺一条都做不出正确实现：
///
/// 1. **时间没有第二个来源**：`updated_at` 要更新为迁移时刻（§5.2：「`updated_at` 与
///    `version` 是两件事：状态迁移改前者」），事件的 `occurred_at` 也要它。而本层
///    **不取系统时钟**（Task 21 的源码面复核 `SystemTime::now` 零命中；Task 8 的
///    `created_at` / `updated_at` 同一条纪律），故只能由调用方给。先例两条：
///    P1 的 `apply_transition(tx, …, occurred_at, event_id)` 与 P3 的
///    `authorize(tx, …, now)`（`continuum-capability/src/registry.rs`）。
/// 2. **事件 id 不能由本函数派生**：`events.event_id` 是主键，唯一性由调用方保证
///    （`docs/superpowers/p1-followups.md` §一.2：「事件 id 由调用方分配且必须唯一」）。
///    「一个 Intent 恰一条 `IntentCompleted`」**不成立**——Product Intent 的合法路径
///    `COMPLETED → EVOLVING → RUNNING → … → COMPLETED`（§5.3 的表）会**两次到达**
///    `COMPLETED`；派生 id 的第二次会撞上第一次的主键，把一个合法迁移变成 `Err`。
///    可复用派生 id 的豁免（P1 只给了 `artifact/{id}` 那一种）不覆盖本条。
/// 3. **设计 §5.4 指名这里照 P1 的形状**：「写路径的形状照 P1 的 `apply_transition`」——
///    P1 的签名就带这两个参数；简报速写的三参数形与 P3D 的
///    `transition_in_tx(tx, id, to)` 同形，而那个函数**不写事件**。
///
/// **参数名与顺序按 P1**：`to` 后接 `occurred_at`、`event_id`。
///
/// # 写入次序与失败路径
///
/// 先判合法性（[`IntentStatus::transition`]），**再**写 `intent` 行（`status` 与
/// `updated_at`），最后写事件——非法对在写入任何一行之前就返回
/// [`IntentError::Transition`]，不留半成品；`to = COMPLETED` 时事件写完后还要复扫
/// 事件日志（见下）。任一步失败都返回 `Err`，而此时前一步的写入**已经留在事务里**——
/// 调用方必须回滚（丢弃 `Tx`），不得提交。本函数不代劳回滚（理由同 P1：`Tx` 没有
/// 保存点接口，回滚只能由调用方做）。照片：`tests/intent_state.rs` 的
/// `a_failed_event_write_leaves_no_half_written_row`（`events` 表缺失的库：行改过、
/// 事件写不进、回滚后值复原）与 `unlisted_pairs_…` 的「行没动」断言。
///
/// # `IntentCompleted` 的收口自检（§5.4 第 2 条）
///
/// `IntentCompleted` 已有一个带 MUST 的消费者：`Tx::scan_event_log` 在「**可跳过
/// （不可解码／版本未知）的事件之后存在 `intent.completed`**」时报 `Err`
/// （`continuum-persist/src/tx.rs` 的收口判定；原因见 P0 设计第 4.1 节：前驱事件不可读时
/// 收口依据不成立）。本层无法控制别层在此之前写了什么，但自己的写入点总在链尾——
/// 故写完 `IntentCompleted` 后**立即复扫整链**，不满足即返回 `Err`（经
/// [`IntentError::Persist`] 转出）。因为不 `commit`，调用方回滚后库里既不留下这次
/// 状态迁移、也不留下那条事件。照片：`tests/intent_state.rs` 的
/// `writing_intent_completed_behind_a_skippable_event_is_rejected`（两侧：有可跳过
/// 事件时被拒且回滚、无可跳过事件时得 `Ok`）。
///
/// # 只有 `COMPLETED` 产生事件
///
/// §5.4 的六项写入点里，与状态迁移挂钩的只有「迁移到 `COMPLETED` → `IntentCompleted`」
/// 一项；其余迁移只改状态（同 P1 的 `event_for`）。
pub fn apply_intent_transition(
    tx: &Tx<'_>,
    id: &IntentId,
    to: IntentStatus,
    occurred_at: i64,
    event_id: &str,
) -> Result<(), IntentError> {
    let Some(intent) = load_intent(tx, id)? else {
        // 不静默创建——登记是 `save_intent` 的活（同 P3D 的 `UnknownModel`）。
        return Err(IntentError::UnknownIntent {
            id: id.as_str().to_owned(),
        });
    };

    // 先判合法（含 `COMPLETED → EVOLVING` 的 kind 条件），再写任何一行。
    IntentStatus::transition(intent.status, to, intent.kind)?;

    // 只改 `status` 与 `updated_at`：`version` 不动（§5.2：状态迁移不 +1）。
    tx.execute(
        "UPDATE intent SET status = ?2, updated_at = ?3 WHERE id = ?1",
        &[
            Value::text(id.as_str()),
            Value::text(to.as_str()),
            Value::Int(occurred_at),
        ],
    )?;

    if to == IntentStatus::Completed {
        let event = Event::new(
            EventType::IntentCompleted,
            event_id.to_owned(),
            occurred_at,
            serde_json::json!({ "intent_id": id.as_str() }),
        )
        .with_intent(id.as_str());
        tx.append_event(&event)?;

        // 收口自检：写完在事务内复扫整链（判定见 `scan_event_log` 的文档）。
        tx.scan_event_log(&default_chain())?;
    }

    Ok(())
}
