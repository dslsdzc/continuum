//! Task 1：`SourceKind` 的九臂与 `MemorySource` 的五字段（设计 §4.1／§4.2）。
//!
//! 照片：R23（空 `external_ref`）。本文件只碰类型层，不起库。

use continuum_memory::{MemoryError, MemorySource, MemorySourceId, SourceKind};

#[test]
fn source_kind_has_the_nine_arms_of_170_and_282() {
    assert_eq!(SourceKind::ALL.len(), 9);
    let expected = [
        (SourceKind::Conversations, "conversations"),
        (SourceKind::TaskHistory, "task_history"),
        (SourceKind::ProductHistory, "product_history"),
        (SourceKind::ProjectArtifacts, "project_artifacts"),
        (SourceKind::Files, "files"),
        (SourceKind::ConnectedServices, "connected_services"),
        (SourceKind::ExplicitUserCorrections, "explicit_user_corrections"),
        (SourceKind::UserPolicies, "user_policies"),
        (SourceKind::Project, "project"),
    ];
    assert_eq!(SourceKind::ALL.len(), expected.len());
    for (kind, code) in expected {
        assert_eq!(kind.as_str(), code, "编码不符：{kind:?}");
        assert_eq!(SourceKind::parse(kind.as_str()), Some(kind), "往返失败：{code}");
    }
}

#[test]
fn source_kind_parse_does_not_default() {
    assert_eq!(SourceKind::parse("conversation"), None);
    assert_eq!(SourceKind::parse(""), None);
    assert_eq!(SourceKind::parse("PROJECT"), None);
}

#[test]
fn memory_source_constructor_rejects_a_blank_external_ref() {
    for blank in ["", "   ", "\t\n"] {
        let err = MemorySource::new(
            MemorySourceId::new("s1"),
            SourceKind::Files,
            blank,
            100,
        )
        .expect_err("空白 external_ref 必须被拒绝");
        assert_eq!(err, MemoryError::EmptyExternalRef);
    }

    let source = MemorySource::new(
        MemorySourceId::new("s1"),
        SourceKind::Files,
        "  sess-1  ",
        100,
    )
    .expect("非空白 external_ref 应可构造");
    assert_eq!(
        source.external_ref(),
        "  sess-1  ",
        "external_ref 必须原样存储，不得 trim"
    );
}

#[test]
fn memory_source_exposes_the_five_fields() {
    let source = MemorySource::new(
        MemorySourceId::new("s-1"),
        SourceKind::ConnectedServices,
        "connector://x",
        123,
    )
    .unwrap();
    assert_eq!(source.id().as_str(), "s-1");
    assert_eq!(source.kind(), SourceKind::ConnectedServices);
    assert_eq!(source.external_ref(), "connector://x");
    assert_eq!(source.observed_at(), 123);
    assert_eq!(source.removed_at(), None);
}

#[test]
fn source_id_round_trips_as_a_string() {
    assert_eq!(MemorySourceId::new("s1").as_str(), "s1");
    let owned = String::from("s2");
    assert_eq!(MemorySourceId::new(owned).as_str(), "s2");
    let borrowed: &str = "s3";
    assert_eq!(MemorySourceId::new(borrowed).as_str(), "s3");
}
