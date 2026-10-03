//! Effect 的记录体（设计上篇第 7.1 节）与状态机（第 7.2 节）。

use serde_json::Value;

/// 设计上篇第 7.5 节的六个效应类型。封闭枚举：策略要按类型裁决，
/// 开放类型会让策略表漏判。
///
/// 不派生 serde：设计下篇第 8 节规定枚举列的落库编码「经显式辅助函数读写，
/// 不依赖 serde」。派生 `snake_case` 的表示恰与落库编码**相同**，故问题不是
/// 「两套表示」而是「两条读写路径」——同一条编码有两个产生点，改一处不会让
/// 另一处失败。本 crate 的 serde 表示另无消费方（不落库、不传网络、不入 JSON）。
///
/// 编码本体在 [`EffectType::as_str`] / [`EffectType::parse`] 上：落库列、审计
/// payload（`crate::journal`）与策略条件的取值都取用它，没有第二份表。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EffectType {
    SendEmail,
    PushBranch,
    Publish,
    DeleteRemote,
    Charge,
    Deploy,
}

impl EffectType {
    /// 全部六个类型。供全量遍历的用例使用（与 [`EffectState::ALL`] 同形，
    /// 那里的说明同样适用，含它挡不住的那一种情形）。
    ///
    /// 完整性与数目由 `tests/persist.rs` 的
    /// `every_effect_type_round_trips_through_its_encoding` 把关。
    pub const ALL: [EffectType; 6] = [
        EffectType::SendEmail,
        EffectType::PushBranch,
        EffectType::Publish,
        EffectType::DeleteRemote,
        EffectType::Charge,
        EffectType::Deploy,
    ];

    /// 设计下篇第 8 节的落库编码：小写、多词以 `_` 连接。
    ///
    /// **本函数是该编码唯一的产生点**：库列的写入、审计 payload 与策略条件的取值
    /// 都取用它。match 穷尽且无通配臂：给枚举加变体时本函数编译不过，编码不会漏分支。
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SendEmail => "send_email",
            Self::PushBranch => "push_branch",
            Self::Publish => "publish",
            Self::DeleteRemote => "delete_remote",
            Self::Charge => "charge",
            Self::Deploy => "deploy",
        }
    }

    /// [`EffectType::as_str`] 的**严格逆**：对 [`EffectType::ALL`] 里的每个变体都有
    /// `EffectType::parse(t.as_str()) == Some(t)`，表外字符串一律 `None`。
    ///
    /// `None` 而不是默认类型：效应类型是策略裁决的依据，取默认会让策略表漏判
    /// （与 `crate::persist` 的解码同一条理由）。
    ///
    /// 解码侧没有穷尽 match 的保护——来源是 `&str` 而非枚举，编译器点不出漏掉的变体，
    /// 故由 `tests/persist.rs` 的 `every_effect_type_round_trips_through_its_encoding`
    /// 遍历 [`EffectType::ALL`] 兜住。
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "send_email" => Self::SendEmail,
            "push_branch" => Self::PushBranch,
            "publish" => Self::Publish,
            "delete_remote" => Self::DeleteRemote,
            "charge" => Self::Charge,
            "deploy" => Self::Deploy,
            _ => return None,
        })
    }
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
    /// 与其中的 `ordinal` 共同把关。**能挡住的情形**：给 `EffectState` 加变体而
    /// 不同步更新它们——`ordinal` 的 match 穷尽且无通配臂，此时整个测试目标编译
    /// 失败（`E0004`），作者被迫回到用例处，紧邻便是本条 `ALL` 的数目与覆盖断言。
    ///
    /// **挡不住的情形**（据实记录）：作者补上了 `ordinal` 的臂，却仍不把新变体
    /// 加进本条 `ALL`。此时 `ALL` 的长度不变，数目与覆盖两条断言都照过，遍历也
    /// 走不到新变体。单靠断言关不掉它：判定变体总数必须先能枚举变体，而枚举的
    /// 来源只有手写名单本身（循环）或 `std::mem::variant_count`，后者在 rustc
    /// 1.95 上仍是 unstable（`E0658`，issue #73662）。真正关掉它要让 `ALL` 与
    /// 变体清单同源（由宏一并展开），那是跨 crate 的惯用法变更，不在本 crate
    /// 单独做——`continuum-graph` 的 `NodeState::ALL` 形态相同。
    ///
    /// 维护本条 `ALL` 时请一并核 `tests/journal.rs` 的 `ordinal`。
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
