//! Effect 的记录体（设计上篇第 7.1 节）与状态机（第 7.2 节）。

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 设计上篇第 7.5 节的六个效应类型。封闭枚举：策略要按类型裁决，
/// 开放类型会让策略表漏判。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectType {
    SendEmail,
    PushBranch,
    Publish,
    DeleteRemote,
    Charge,
    Deploy,
}

/// 设计上篇第 7.2 节的七个状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EffectState {
    Planned,
    Authorized,
    Executing,
    Committed,
    Failed,
    RolledBack,
    Unknown,
}

impl EffectState {
    /// 全部七个状态，供全量遍历的用例使用（与 P1 的 `NodeState::ALL` 同形）。
    pub const ALL: [EffectState; 7] = [
        EffectState::Planned,
        EffectState::Authorized,
        EffectState::Executing,
        EffectState::Committed,
        EffectState::Failed,
        EffectState::RolledBack,
        EffectState::Unknown,
    ];
}

/// 效应记录的身份。稳定，不随状态变化。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EffectId(String);

impl EffectId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for EffectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StateError {
    #[error("迁移 {from:?} → {to:?} 不在允许的迁移表内")]
    Illegal { from: EffectState, to: EffectState },
}

/// 判定一次状态迁移是否合法。设计上篇第 7.2 节的迁移表，表外一律拒绝。
///
/// `COMMITTED` / `FAILED` / `ROLLED_BACK` 是终态，在表中没有任何出边；
/// `UNKNOWN` **不是**终态（它表示「不知道外部发生了什么」），故它既有入边也有出边，
/// 并允许 `UNKNOWN → UNKNOWN` 的自环（对账后仍不确定）。
pub fn transition(from: EffectState, to: EffectState) -> Result<EffectState, StateError> {
    use EffectState::*;

    let legal = matches!(
        (from, to),
        (Planned, Authorized)
            | (Planned, Failed)
            | (Planned, RolledBack)
            | (Authorized, Executing)
            | (Authorized, Failed)
            | (Authorized, RolledBack)
            | (Executing, Committed)
            | (Executing, Failed)
            | (Executing, Unknown)
            | (Unknown, Committed)
            | (Unknown, Failed)
            | (Unknown, RolledBack)
            | (Unknown, Unknown)
    );

    if legal {
        Ok(to)
    } else {
        Err(StateError::Illegal { from, to })
    }
}

/// 设计上篇第 7.1 节的记录体。字段与类型照该节。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Effect {
    pub id: EffectId,
    pub effect_type: EffectType,
    /// 作用对象（URL / 分支名 / 收件人 / 资源标识），文本。
    pub target: String,
    pub parameters: Value,
    /// 不透明凭据串；本子项目只记录，不校验（设计上篇第 7.7 节）。
    pub authorization: String,
    /// 幂等键，唯一。
    pub idempotency_key: String,
    pub state: EffectState,
    /// Unix 毫秒。
    pub planned_at: i64,
    /// Unix 毫秒。
    pub updated_at: i64,
}
