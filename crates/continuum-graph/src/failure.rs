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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub retryable_errors: Vec<FailureClass>,
    pub escalation_policy: EscalationPolicy,
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
