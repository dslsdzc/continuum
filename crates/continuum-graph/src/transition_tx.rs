//! 状态迁移与对应事件的同事务写入（设计 §16、§17、`§318`）。
//!
//! `crate::state::transition` 是纯函数：它判定迁移是否合法，但不落库、不发事件。
//! 本模块把「校验 → 写状态 → 写事件」绑到调用方的事务上，使 §318 的原子性
//! 在类型层面成立：本模块不持有 `Db`，只能经调用方的 `Tx` 写入，因此不存在
//! 「写完状态自己提交、事件留在另一个事务」的写法。

use crate::node::NodeState;
use crate::persist::{parse_state, state_str};
use crate::state::{transition, StateError};
use continuum_events::{Event, EventType};
use continuum_persist::{PersistError, Tx, Value};

#[derive(Debug, thiserror::Error)]
pub enum ApplyError {
    #[error("图 {graph_id} 中不存在节点 {node_id}")]
    UnknownNode { graph_id: String, node_id: String },
    #[error("迁移被拒：{0}")]
    Illegal(#[from] StateError),
    #[error("持久化失败：{0}")]
    Persist(#[from] PersistError),
}

/// 在同一个事务内：读当前状态 → 校验迁移 → 写回状态 → 写对应事件。
///
/// 只使用调用方传入的 `Tx`，不自行开启或提交事务——提交由调用方负责，
/// 这正是 `§318` 所要求的原子性：状态与事件要么一起生效，要么一起不生效。
///
/// 先判合法性再写：非法迁移在写入任何一行之前就返回 `Illegal`，不留半成品。
///
/// # 返回 `Err` 之后调用方必须回滚该事务，不得提交
///
/// 本函数先写状态、再写事件。若事件写入失败（`events.event_id` 是主键，
/// 撞上重复 id 即失败），返回 `Err` 时**前一步的状态写入已经留在事务里**。
/// 调用方若把错误收集起来、最后统一 `commit()`，就会提交出一个状态已是
/// `Running` 却没有任何 `NodeStarted` 的节点——§318 的原子性只在函数边界内
/// 成立，端到端要靠「见到 `Err` 就回滚」这条约定兜住。
///
/// 本函数不代劳回滚：`Tx` 没有「回滚到保存点」这类接口，要回滚只能把 `Tx`
/// 丢弃，而那会连带销毁调用方手里的句柄——回滚的决定只能由调用方做。
///
/// `event_id` 的唯一性由调用方保证。**P2 的执行器须用单调序号分配**
/// （如 `<graph>/<node>/<attempt>/<seq>`），不要用可由节点状态派生的可复用 id：
/// 合法路径（如 `Running → Waiting → Ready → Queued → Running`）会两次进入
/// `Running`，复用的 id 会在第二次撞上主键，事件写不进去而状态已经改了。
/// P1 内只有测试调用本函数。
pub fn apply_transition(
    tx: &Tx<'_>,
    graph_id: &str,
    node_id: &str,
    to: NodeState,
    occurred_at: i64,
    event_id: &str,
) -> Result<NodeState, ApplyError> {
    let rows = tx.query(
        "SELECT state FROM adfir_node WHERE graph_id = ?1 AND node_id = ?2",
        &[Value::text(graph_id), Value::text(node_id)],
    )?;
    let Some(row) = rows.into_iter().next() else {
        return Err(ApplyError::UnknownNode {
            graph_id: graph_id.to_owned(),
            node_id: node_id.to_owned(),
        });
    };
    // 状态列经 `parse_state` 还原，与 `load_graph` 走同一套编解码
    let from = parse_state(&text(&row[0])?)?;

    let next = transition(from, to)?;

    tx.execute(
        "UPDATE adfir_node SET state = ?3 WHERE graph_id = ?1 AND node_id = ?2",
        &[
            Value::text(graph_id),
            Value::text(node_id),
            // 取值经 `state_str`，不写字面量：该列的编码只有一个来源
            Value::text(state_str(next)),
        ],
    )?;

    // 只有 §16 列举的到达态产生事件；其余迁移只改状态。
    if let Some(event_type) = event_for(next) {
        let event = Event::new(
            event_type,
            event_id.to_owned(),
            occurred_at,
            serde_json::json!({ "graph_id": graph_id, "node_id": node_id }),
        )
        .with_node(node_id.to_owned());
        tx.append_event(&event)?;
    }

    Ok(next)
}

/// §16 的三个外发事件中与节点状态相关的两个。
///
/// 第三个是 `ArtifactCreated`，它由 Artifact 入库触发而非状态迁移触发，
/// 见 `continuum_artifact::save_artifact`。
fn event_for(state: NodeState) -> Option<EventType> {
    match state {
        NodeState::Running => Some(EventType::NodeStarted),
        NodeState::Completed => Some(EventType::NodeCompleted),
        _ => None,
    }
}

fn text(v: &Value) -> Result<String, PersistError> {
    match v {
        Value::Text(s) => Ok(s.clone()),
        other => Err(PersistError::Database(format!("列应为文本，实际 {other:?}"))),
    }
}
