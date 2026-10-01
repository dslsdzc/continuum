use continuum_artifact::{Artifact, ArtifactId, ArtifactType, ContentHash, PrivacyClass};
use continuum_graph::{
    is_candidate_backend, ArtifactRef, ExecutionProfile, NodeContext, NodeId, OperatorImpl,
};
use continuum_operator::{
    BackendId, Determinism, Operator, OperatorId, OperatorVersion, SideEffectClass,
};
use serde_json::json;

fn op(backends: &[&str]) -> Operator {
    Operator {
        id: OperatorId::new("op"),
        version: OperatorVersion::new(1),
        input_schema: vec![ArtifactType::Text],
        output_schema: vec![ArtifactType::Text],
        determinism: Determinism::Deterministic,
        side_effect_class: SideEffectClass::Pure,
        backend_candidates: backends.iter().map(|b| BackendId::new(*b)).collect(),
    }
}

#[test]
fn execution_profile_starts_empty() {
    // 本子项目内四个资源字段恒为 None：P3 与 P7 之前无对应资源
    let p = ExecutionProfile::default();
    assert!(p.model.is_none());
    assert!(p.provider.is_none());
    assert!(p.tool.is_none());
    assert!(p.backend.is_none());
    assert!(p.compute_node.is_none());
    assert!(p.reasoning_effort.is_none());
    assert!(p.parallelism.is_none());
    assert!(p.timeout_ms.is_none());
    assert!(p.cost_budget.is_none());
    // retry_policy 非可选：默认策略是「单次尝试、不重试」
    assert_eq!(p.retry_policy.max_attempts, 1);
    assert!(p.retry_policy.retryable_errors.is_empty());
}

#[test]
fn context_starts_uncancelled_and_can_be_cancelled() {
    let ctx = NodeContext::new(NodeId::new("n1"), ExecutionProfile::default());
    assert!(!ctx.is_cancelled());
    ctx.cancel();
    assert!(ctx.is_cancelled(), "取消后应可观察到");
}

#[test]
fn a_second_handle_observes_cancellation() {
    // 取消信号必须能被另一个持有者看到，否则执行中的算子无法中止
    let ctx = NodeContext::new(NodeId::new("n1"), ExecutionProfile::default());
    let ctx2 = ctx.clone();
    ctx.cancel();
    assert!(ctx2.is_cancelled(), "取消信号应跨句柄可见");
}

#[test]
fn declared_backend_is_a_candidate() {
    let operator = op(&["builtin", "remote"]);
    assert!(is_candidate_backend(&operator, &BackendId::new("builtin")));
    assert!(is_candidate_backend(&operator, &BackendId::new("remote")));
}

#[test]
fn undeclared_backend_is_not_a_candidate() {
    let operator = op(&["builtin"]);
    assert!(!is_candidate_backend(&operator, &BackendId::new("remote")));
}

/// 一个确定性算子实现。仅用于证明 OperatorImpl 可被实现，
/// 并证明 ArtifactRef 能被读取。
struct UpperCase;

impl OperatorImpl for UpperCase {
    fn execute(
        &self,
        inputs: &[ArtifactRef],
        _ctx: &NodeContext,
    ) -> Result<Vec<Artifact>, continuum_operator::OperatorError> {
        let first = inputs.first().ok_or_else(|| {
            continuum_operator::OperatorError::NotFound {
                id: OperatorId::new("input"),
                version: OperatorVersion::new(0),
            }
        })?;
        Ok(vec![Artifact {
            id: ArtifactId::new("out"),
            artifact_type: ArtifactType::Text,
            content_hash: ContentHash::of(first.id.as_str().as_bytes()),
            size: 0,
            producer_node: None,
            input_artifacts: vec![first.id.clone()],
            metadata: json!({}),
            provenance: json!({}),
            privacy_class: PrivacyClass::Personal,
            version: 1,
        }])
    }
}

#[test]
fn operator_impl_is_implementable_and_reads_its_inputs() {
    let implementation = UpperCase;
    let input = ArtifactRef {
        id: ArtifactId::new("a1"),
        content_hash: ContentHash::of(b"a1"),
    };
    let ctx = NodeContext::new(NodeId::new("n1"), ExecutionProfile::default());

    let out = implementation.execute(&[input.clone()], &ctx).expect("应能执行");
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].input_artifacts, vec![ArtifactId::new("a1")]);
    assert_eq!(out[0].content_hash, ContentHash::of(b"a1"));
}

#[test]
fn operator_impl_can_report_failure() {
    let implementation = UpperCase;
    let ctx = NodeContext::new(NodeId::new("n1"), ExecutionProfile::default());
    assert!(
        implementation.execute(&[], &ctx).is_err(),
        "无输入时应返回错误而非 panic"
    );
}
