use continuum_artifact::ArtifactType;
use continuum_graph::{
    decide_retry, Backoff, EscalationPolicy, FailureClass, RetryDecision, RetryPolicy,
};
use continuum_operator::{
    BackendId, Determinism, Operator, OperatorId, OperatorVersion, SideEffectClass,
};

fn op(side_effect: SideEffectClass) -> Operator {
    Operator {
        id: OperatorId::new("op"),
        version: OperatorVersion::new(1),
        input_schema: vec![ArtifactType::Text],
        output_schema: vec![ArtifactType::Text],
        determinism: Determinism::Deterministic,
        side_effect_class: side_effect,
        backend_candidates: vec![BackendId::new("builtin")],
    }
}

fn policy(max_attempts: u32, retryable: Vec<FailureClass>) -> RetryPolicy {
    RetryPolicy {
        max_attempts,
        backoff: Backoff::Fixed { interval_ms: 1_000 },
        retryable_errors: retryable,
        escalation_policy: EscalationPolicy::Manual,
    }
}

#[test]
fn backoff_expresses_both_strategies() {
    // 该字段本 task 不消费，但必须能表达 §13.1 给 RESOURCE 的「退避后升级」
    let fixed = Backoff::Fixed { interval_ms: 500 };
    let exponential = Backoff::Exponential {
        initial_ms: 100,
        factor: 2,
        max_ms: 30_000,
    };
    assert_ne!(fixed, exponential);
    assert_eq!(
        serde_json::from_str::<Backoff>(&serde_json::to_string(&exponential).expect("可序列化"))
            .expect("可反序列化"),
        exponential
    );
}

#[test]
fn transient_error_retries_within_the_budget() {
    let p = policy(3, vec![FailureClass::Transient]);
    assert_eq!(
        decide_retry(&op(SideEffectClass::Pure), &p, FailureClass::Transient, 1),
        RetryDecision::Retry { next_attempt: 2 }
    );
}

#[test]
fn exhausting_attempts_escalates() {
    let p = policy(3, vec![FailureClass::Transient]);
    assert_eq!(
        decide_retry(&op(SideEffectClass::Pure), &p, FailureClass::Transient, 3),
        RetryDecision::Escalate {
            to: EscalationPolicy::Manual
        }
    );
}

#[test]
fn non_idempotent_side_effect_never_retries() {
    // §307 的 MUST，也是 02 §3.4 判据 4
    let p = policy(5, vec![FailureClass::Transient]);
    assert_eq!(
        decide_retry(
            &op(SideEffectClass::NonIdempotent),
            &p,
            FailureClass::Transient,
            1
        ),
        RetryDecision::Escalate {
            to: EscalationPolicy::Manual
        },
        "非幂等 Effect 即使类别可重试也不得自动重试"
    );
}

#[test]
fn idempotent_side_effect_may_retry() {
    let p = policy(3, vec![FailureClass::Transient]);
    assert!(matches!(
        decide_retry(&op(SideEffectClass::Idempotent), &p, FailureClass::Transient, 1),
        RetryDecision::Retry { .. }
    ));
}

#[test]
fn constraint_and_authorization_escalate_without_retry() {
    let p = policy(5, vec![FailureClass::Constraint, FailureClass::Authorization]);
    for class in [FailureClass::Constraint, FailureClass::Authorization] {
        assert_eq!(
            decide_retry(&op(SideEffectClass::Pure), &p, class, 1),
            RetryDecision::Escalate {
                to: EscalationPolicy::Manual
            },
            "{class:?} 应升级而非重试"
        );
    }
}

#[test]
fn permanent_verification_and_unknown_fail_without_retry() {
    let p = policy(5, vec![
        FailureClass::Permanent,
        FailureClass::Verification,
        FailureClass::Unknown,
    ]);
    for class in [
        FailureClass::Permanent,
        FailureClass::Verification,
        FailureClass::Unknown,
    ] {
        assert_eq!(
            decide_retry(&op(SideEffectClass::Pure), &p, class, 1),
            RetryDecision::Fail,
            "{class:?} 应直接失败"
        );
    }
}

#[test]
fn resource_retries_when_listed_and_budget_allows() {
    let p = policy(3, vec![FailureClass::Resource]);
    assert_eq!(
        decide_retry(&op(SideEffectClass::Pure), &p, FailureClass::Resource, 1),
        RetryDecision::Retry { next_attempt: 2 }
    );
}

#[test]
fn whitelisting_a_permanent_class_does_not_make_it_retryable() {
    // retryable_errors 只能收窄 §13.1 的固有归属，不能放宽
    let p = policy(5, vec![FailureClass::Permanent]);
    assert_eq!(
        decide_retry(&op(SideEffectClass::Pure), &p, FailureClass::Permanent, 1),
        RetryDecision::Fail
    );
}

#[test]
fn a_class_outside_retryable_errors_fails() {
    let p = policy(5, vec![FailureClass::Transient]);
    assert_eq!(
        decide_retry(&op(SideEffectClass::Pure), &p, FailureClass::Resource, 1),
        RetryDecision::Fail
    );
}
