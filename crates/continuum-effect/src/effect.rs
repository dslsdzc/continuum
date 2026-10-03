//! Effect 的记录体（设计上篇第 7.1 节）与状态机（第 7.2 节）。

use serde_json::Value;

/// 设计上篇第 7.5 节的六个效应类型。封闭枚举：策略要按类型裁决，
/// 开放类型会让策略表漏判。
///
/// 不派生 serde：设计下篇第 8 节规定枚举列的落库编码「经显式辅助函数读写，
/// 不依赖 serde」。派生 `snake_case` 的表示恰与落库编码**相同**，故问题不是
/// 「两套表示」而是「两条读写路径」——同一条编码有两个产生点，改一处不会让
/// 另一处失败。本 crate 的 serde 表示另无消费方（不落库、不传网络、不入 JSON）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EffectType {
    SendEmail,
    PushBranch,
    Publish,
    DeleteRemote,
    Charge,
    Deploy,
}

/// 设计上篇第 7.2 节的七个状态。
///
/// 不派生 serde：设计下篇第 8 节的落库编码是小写（`rolled_back`），
/// 而派生 `SCREAMING_SNAKE_CASE` 会给出**另一套**字符串（`ROLLED_BACK`）——
/// 同一事实两份表示，正是 P1 出过 Critical 的那类。本 crate 的 serde 表示
/// 另无消费方（不落库、不传网络，`Effect.parameters` 是 `serde_json::Value`，
/// 与枚举本身无关）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    ///
    /// 完整性由 `tests/journal.rs` 的 `all_lists_every_variant_exactly_once`
    /// 钉住：用例里的穷尽 match 使「加了变体却漏加进 `ALL`」编译失败，
    /// 回到用例后由数目与覆盖断言判定 `ALL` 恰是全部变体的一个排列。
    /// 该拦截的边界见 `ordinal` 的注释。
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
///
/// 不派生 serde：与 [`EffectType`] 同理，本 crate 的 serde 表示无消费方，
/// 而落库编码由 Task 2 的显式辅助函数读写（设计下篇第 8 节）。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
///
/// 不派生 serde：本记录没有 JSON 或网络的消费方，落库由 Task 2 的列级读写完成
/// （设计下篇第 8 节）。派生会连带要求 [`EffectId`] 与两个枚举也派生，
/// 从而把上面各自的 serde 表示一并拉回来。
#[derive(Debug, Clone, PartialEq)]
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
