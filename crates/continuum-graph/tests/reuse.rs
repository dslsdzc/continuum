use continuum_artifact::{ArtifactType, ContentHash};
use continuum_graph::{cache_key, can_reuse, CacheKey};
use continuum_operator::{
    BackendId, Determinism, Operator, OperatorId, OperatorVersion, SideEffectClass,
};

fn op(determinism: Determinism) -> Operator {
    Operator {
        id: OperatorId::new("op"),
        version: OperatorVersion::new(1),
        input_schema: vec![ArtifactType::Text],
        output_schema: vec![ArtifactType::Text],
        determinism,
        side_effect_class: SideEffectClass::Pure,
        backend_candidates: vec![BackendId::new("builtin")],
    }
}

fn hash(s: &str) -> ContentHash {
    ContentHash::of(s.as_bytes())
}

#[test]
fn deterministic_operator_produces_a_cache_key() {
    let key = cache_key(&op(Determinism::Deterministic), hash("in")).expect("应产生缓存键");
    assert_eq!(key.operator_version, OperatorVersion::new(1));
}

#[test]
fn non_deterministic_operator_produces_no_cache_key() {
    // ENG-002 裁定 + 02 §3.4 判据 5 的验证点
    assert!(
        cache_key(&op(Determinism::NonDeterministic), hash("in")).is_none(),
        "非确定 Operator 不得产生缓存键"
    );
}

#[test]
fn reuse_requires_all_four_conditions() {
    let operator = op(Determinism::Deterministic);
    let current = hash("in");
    let key = CacheKey {
        input_hash: current.clone(),
        operator_version: OperatorVersion::new(1),
    };

    assert!(can_reuse(&operator, Some(&key), &current, false), "四条件全满足应复用");
}

#[test]
fn changed_input_hash_prevents_reuse() {
    let operator = op(Determinism::Deterministic);
    let key = CacheKey {
        input_hash: hash("old"),
        operator_version: OperatorVersion::new(1),
    };
    assert!(!can_reuse(&operator, Some(&key), &hash("new"), false));
}

#[test]
fn changed_operator_version_prevents_reuse() {
    let operator = op(Determinism::Deterministic);
    let key = CacheKey {
        input_hash: hash("in"),
        operator_version: OperatorVersion::new(2),
    };
    assert!(!can_reuse(&operator, Some(&key), &hash("in"), false));
}

#[test]
fn affected_contract_prevents_reuse() {
    let operator = op(Determinism::Deterministic);
    let key = CacheKey {
        input_hash: hash("in"),
        operator_version: OperatorVersion::new(1),
    };
    assert!(!can_reuse(&operator, Some(&key), &hash("in"), true));
}

#[test]
fn non_deterministic_operator_never_reuses() {
    let operator = op(Determinism::NonDeterministic);
    let key = CacheKey {
        input_hash: hash("in"),
        operator_version: OperatorVersion::new(1),
    };
    assert!(!can_reuse(&operator, Some(&key), &hash("in"), false));
}

#[test]
fn missing_cache_entry_prevents_reuse() {
    let operator = op(Determinism::Deterministic);
    assert!(!can_reuse(&operator, None, &hash("in"), false));
}
