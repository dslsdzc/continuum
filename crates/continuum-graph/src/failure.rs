//! 失败分类与重试判定（§307、§309）。

use continuum_operator::{Operator, SideEffectClass};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FailureClass {
    Transient,
    Permanent,
    Constraint,
    Authorization,
    Resource,
    Verification,
    Unknown,
}

impl FailureClass {
    pub const ALL: [FailureClass; 7] = [
        FailureClass::Transient,
        FailureClass::Permanent,
        FailureClass::Constraint,
        FailureClass::Authorization,
        FailureClass::Resource,
        FailureClass::Verification,
        FailureClass::Unknown,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EscalationPolicy {
    None,
    Decision,
    Manual,
}

/// 退避策略。设计 `§13.2` 的 `RetryPolicy` 含 `backoff`，`§13.1` 给 RESOURCE
/// 的固有策略是「可重试，退避后仍失败则升级」——缺该字段则接口无法表达退避。
/// （`§307` 管的是另一条：非幂等 Effect 不进入自动重试。）
///
/// 本子项目的 `decide_retry` 不使用它：退避是执行方等待时的事，此处只让类型
/// 能表达该策略。消费方在 P3 之后的调度路径。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Backoff {
    /// 固定间隔。
    Fixed { interval_ms: u64 },
    /// 指数退避：首次 `initial_ms`，每次乘 `factor`，单次不超过 `max_ms`。
    Exponential {
        initial_ms: u64,
        factor: u32,
        max_ms: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub backoff: Backoff,
    pub retryable_errors: Vec<FailureClass>,
    pub escalation_policy: EscalationPolicy,
}

impl Default for RetryPolicy {
    /// 默认策略为「单次尝试、不重试」。三道闸各自独立，缺一不可：
    ///
    /// - **空白名单是走到判定的那一闸**。设计 `§13.2` 规定 `retryable_errors`
    ///   是在 `§13.1` 固有归属之上**收窄**的白名单，空白名单使所有类别都在
    ///   `decide_retry` 第 3 步被拒，第 5 步的升级分支根本不会被走到。
    /// - `max_attempts = 1` 独立地把总尝试数限为一次：`attempt` 自 1 起计，
    ///   仅当 `attempt < max_attempts` 才重试，故首次尝试失败时已无余量。
    ///   将来若为默认策略填入 `retryable_errors`，这一条仍然拦得住。
    /// - `EscalationPolicy::None` 决定走到第 5 步时的动作：升级退化为失败。
    fn default() -> Self {
        Self {
            max_attempts: 1,
            backoff: Backoff::Fixed { interval_ms: 1_000 },
            retryable_errors: Vec::new(),
            escalation_policy: EscalationPolicy::None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryDecision {
    Retry { next_attempt: u32 },
    Escalate { to: EscalationPolicy },
    Fail,
}

/// 判定一次失败后的动作。
///
/// 判定顺序：
///
/// 1. 非幂等副作用直接升级（`§307` 的 MUST），白名单不改变它
/// 2. 按类别取 `§309` 的固有归属：
///    `CONSTRAINT` / `AUTHORIZATION` 升级；`PERMANENT` / `VERIFICATION` /
///    `UNKNOWN` 失败；`TRANSIENT` / `RESOURCE` 继续往下判
/// 3. 类别不在 `retryable_errors` 内时失败
/// 4. 仍有尝试余量时重试
/// 5. 否则升级
///
/// `retryable_errors` 只能**收窄**固有归属，不能放宽：把一个 `PERMANENT`
/// 类别加进白名单不会让它变成可重试。第 2 步先于第 3 步，故这一点由结构保证。
pub fn decide_retry(
    operator: &Operator,
    policy: &RetryPolicy,
    class: FailureClass,
    attempt: u32,
) -> RetryDecision {
    if operator.side_effect_class == SideEffectClass::NonIdempotent {
        return escalate(policy);
    }

    match class {
        FailureClass::Constraint | FailureClass::Authorization => return escalate(policy),
        FailureClass::Permanent | FailureClass::Verification | FailureClass::Unknown => {
            return RetryDecision::Fail
        }
        FailureClass::Transient | FailureClass::Resource => {}
    }

    if !policy.retryable_errors.contains(&class) {
        return RetryDecision::Fail;
    }
    if attempt < policy.max_attempts {
        return RetryDecision::Retry {
            next_attempt: attempt + 1,
        };
    }
    escalate(policy)
}

fn escalate(policy: &RetryPolicy) -> RetryDecision {
    match policy.escalation_policy {
        EscalationPolicy::None => RetryDecision::Fail,
        other => RetryDecision::Escalate { to: other },
    }
}
