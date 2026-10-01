use continuum_artifact::ArtifactType;
use continuum_operator::{
    BackendId, Determinism, Operator, OperatorError, OperatorId, OperatorRegistry,
    OperatorVersion, SideEffectClass,
};

fn op(id: &str, version: u32, determinism: Determinism) -> Operator {
    Operator {
        id: OperatorId::new(id),
        version: OperatorVersion::new(version),
        input_schema: vec![ArtifactType::SourceTree],
        output_schema: vec![ArtifactType::Patch],
        determinism,
        side_effect_class: SideEffectClass::Pure,
        backend_candidates: vec![BackendId::new("builtin")],
    }
}

#[test]
fn registered_operator_is_resolvable_by_id_and_version() {
    let mut registry = OperatorRegistry::new();
    registry.register(op("patch.apply", 1, Determinism::Deterministic)).unwrap();

    let got = registry
        .resolve(&OperatorId::new("patch.apply"), &OperatorVersion::new(1))
        .expect("应能解析");
    assert_eq!(got.output_schema, vec![ArtifactType::Patch]);
}

#[test]
fn unknown_operator_is_rejected() {
    let registry = OperatorRegistry::new();
    match registry.resolve(&OperatorId::new("nope"), &OperatorVersion::new(1)) {
        Err(OperatorError::NotFound { id, version }) => {
            assert_eq!(id.as_str(), "nope");
            assert_eq!(version.as_u32(), 1);
        }
        other => panic!("未注册的 Operator 必须被拒绝，实际 {other:?}"),
    }
}

#[test]
fn same_id_with_different_versions_coexist() {
    let mut registry = OperatorRegistry::new();
    registry.register(op("patch.apply", 1, Determinism::Deterministic)).unwrap();
    registry.register(op("patch.apply", 2, Determinism::NonDeterministic)).unwrap();

    let v2 = registry
        .resolve(&OperatorId::new("patch.apply"), &OperatorVersion::new(2))
        .expect("v2 应能解析");
    assert_eq!(v2.determinism, Determinism::NonDeterministic);
}

#[test]
fn error_messages_render_the_version_as_a_plain_number() {
    // OperatorVersion 缺 Display 时错误文案渲染为 `OperatorVersion(1)`，
    // 与同 crate 的 OperatorId 风格不一。
    let registry = OperatorRegistry::new();
    let not_found = registry
        .resolve(&OperatorId::new("patch.apply"), &OperatorVersion::new(1))
        .expect_err("未注册必须被拒绝");
    assert_eq!(not_found.to_string(), "Operator patch.apply 版本 1 未注册");

    let mut registry = OperatorRegistry::new();
    registry
        .register(op("patch.apply", 1, Determinism::Deterministic))
        .unwrap();
    let duplicate = registry
        .register(op("patch.apply", 1, Determinism::Deterministic))
        .expect_err("重复注册必须被拒绝");
    assert_eq!(duplicate.to_string(), "Operator patch.apply 版本 1 已注册");
}

#[test]
fn duplicate_id_and_version_is_rejected() {
    let mut registry = OperatorRegistry::new();
    registry.register(op("patch.apply", 1, Determinism::Deterministic)).unwrap();
    match registry.register(op("patch.apply", 1, Determinism::NonDeterministic)) {
        Err(OperatorError::Duplicate { id, version }) => {
            assert_eq!(id.as_str(), "patch.apply");
            assert_eq!(version.as_u32(), 1);
        }
        other => panic!("同 id 同版本重复注册必须被拒绝，实际 {other:?}"),
    }
}
