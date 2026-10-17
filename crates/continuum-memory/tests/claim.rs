//! Task 4：公开编码与 `Confidence`（设计 §5.1–§5.3）。
//!
//! 私有构造、`validate_scope_anchor` 与 `load_claim` 的用例在 `src/claim.rs` 的
//! 单元测试里（集成测试 crate 看不见 `pub(crate)`）。
//!
//! 照片：R22（`Confidence` 的任意 JSON 往返）、S6（五 scope）、S8（三臂 validity）。

use continuum_memory::{Confidence, MemoryScope, Validity};

#[cfg(feature = "test-support")]
use continuum_memory::{MemoryClaim, MemoryClaimId, MemorySourceId};

#[test]
fn memory_scope_has_the_five_arms_of_284() {
    assert_eq!(MemoryScope::ALL.len(), 5);
    let expected = [
        (MemoryScope::Personal, "personal"),
        (MemoryScope::Project, "project"),
        (MemoryScope::Product, "product"),
        (MemoryScope::Task, "task"),
        (MemoryScope::Runtime, "runtime"),
    ];
    for (scope, code) in expected {
        assert_eq!(scope.as_str(), code);
        assert_eq!(MemoryScope::parse(scope.as_str()), Some(scope));
    }
}

#[test]
fn memory_scope_parse_does_not_default() {
    assert_eq!(MemoryScope::parse("session"), None);
    assert_eq!(MemoryScope::parse("PERSONAL"), None);
    assert_eq!(MemoryScope::parse(""), None);
}

#[test]
fn validity_has_three_arms_and_round_trips() {
    assert_eq!(
        Validity::ALL,
        [Validity::Active, Validity::Superseded, Validity::Invalidated]
    );
    for validity in Validity::ALL {
        assert_eq!(Validity::parse(validity.as_str()), Some(validity));
    }
    assert_eq!(Validity::Superseded.as_str(), "superseded");
    assert_eq!(Validity::parse("unknown"), None);
    assert_eq!(Validity::parse("ACTIVE"), None);
    assert_eq!(Validity::parse(""), None);
}

#[test]
fn confidence_round_trips_arbitrary_json() {
    for value in [
        serde_json::json!(null),
        serde_json::json!("high"),
        serde_json::json!({"grade": "medium", "score": 2}),
        serde_json::json!([1, 2, 3]),
    ] {
        let confidence = Confidence::from_value(value.clone());
        assert_eq!(confidence.as_value(), &value, "JSON 值往返必须相等");
    }
}

#[test]
fn confidence_has_no_default_and_no_order() {
    // 「没有 Ord／没有默认值」是编译期事实，运行期只能钉住「任意 JSON 都能构造」。
    // 排序的负例在 Task 11 的 C6。
    let confidence = Confidence::from_value(serde_json::json!(null));
    assert_eq!(confidence.as_value(), &serde_json::json!(null));
}

#[cfg(feature = "test-support")]
#[test]
fn for_test_is_available_under_the_test_support_feature() {
    let claim = MemoryClaim::for_test(
        MemoryClaimId::new("c-test"),
        MemoryScope::Personal,
        "test-only",
        vec![MemorySourceId::new("s-test")],
        Validity::Superseded,
        Confidence::from_value(serde_json::json!(1.0)),
        42,
    );
    assert_eq!(claim.id().as_str(), "c-test");
    assert_eq!(claim.scope(), MemoryScope::Personal);
    assert_eq!(claim.statement(), "test-only");
    assert_eq!(claim.sources().len(), 1);
    assert_eq!(claim.sources()[0].as_str(), "s-test");
    assert_eq!(claim.validity(), Validity::Superseded);
    assert_eq!(
        claim.confidence(),
        &Confidence::from_value(serde_json::json!(1.0))
    );
    assert_eq!(claim.last_confirmed(), 42);
}
