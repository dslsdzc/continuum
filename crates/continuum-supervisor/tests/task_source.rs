//! Task 1：`TaskSource` 四值闭集与两个授权引用。

mod support;

use std::error::Error;

use continuum_semantics::{IntentKind, IntentStatus};
use continuum_supervisor::{
    MaintenanceOccurrenceRef, PolicyGrantRef, TaskError, TaskSource,
};

use support::code_only;

fn exhaustive(source: TaskSource) -> &'static str {
    match source {
        TaskSource::AuthorizedIntent => "authorized_intent",
        TaskSource::Policy => "policy",
        TaskSource::MaintenanceSchedule => "maintenance_schedule",
        TaskSource::ProductDrive => "product_drive",
    }
}

#[test]
fn task_source_has_exactly_four_values_and_verbatim_labels() {
    let sources = [
        TaskSource::AuthorizedIntent,
        TaskSource::Policy,
        TaskSource::MaintenanceSchedule,
        TaskSource::ProductDrive,
    ];
    let labels = [
        "authorized_intent",
        "policy",
        "maintenance_schedule",
        "product_drive",
    ];
    for (source, label) in sources.into_iter().zip(labels) {
        assert_eq!(source.as_str(), label);
        assert_eq!(exhaustive(source), label);
    }
}

#[test]
fn task_source_has_no_parser_or_model_suggested_variant() {
    let corpus = [
        include_str!("../src/lib.rs"),
        include_str!("../src/task.rs"),
        include_str!("../src/supervisor.rs"),
        include_str!("../src/session.rs"),
        include_str!("../src/resource.rs"),
        include_str!("../src/trigger.rs"),
        include_str!("../src/error.rs"),
    ]
    .map(code_only)
    .join("\n");

    for banned in [
        "ModelSuggested",
        "TaskSource::Model",
        "UserRequested",
        "SystemIdle",
        "impl FromStr for TaskSource",
        "fn parse",
    ] {
        assert!(
            !corpus.contains(banned),
            "源码面不应出现 {banned}：{corpus}"
        );
    }

    let task = code_only(include_str!("../src/task.rs"));
    for banned in ["ModelSuggested", "UserRequested", "SystemIdle"] {
        assert!(!task.contains(banned), "task.rs 不应出现 {banned}");
    }
}

#[test]
fn policy_and_maintenance_refs_reject_empty_fields() {
    assert_eq!(
        PolicyGrantRef::new("", "g").unwrap_err(),
        TaskError::EmptyPolicyGrant
    );
    assert_eq!(
        PolicyGrantRef::new("i", "").unwrap_err(),
        TaskError::EmptyPolicyGrant
    );
    assert_eq!(
        MaintenanceOccurrenceRef::new("", "o").unwrap_err(),
        TaskError::EmptyMaintenanceRef
    );
    assert_eq!(
        MaintenanceOccurrenceRef::new("s", "").unwrap_err(),
        TaskError::EmptyMaintenanceRef
    );

    let grant = PolicyGrantRef::new("issuer", "grant").expect("非空引用应可构造");
    assert_eq!(grant.issuer(), "issuer");
    assert_eq!(grant.grant_id(), "grant");

    let occurrence =
        MaintenanceOccurrenceRef::new("schedule", "occurrence").expect("非空引用应可构造");
    assert_eq!(occurrence.schedule_id(), "schedule");
    assert_eq!(occurrence.occurrence_id(), "occurrence");
}

#[test]
fn task_error_display_covers_all_six_arms() {
    let errors = [
        TaskError::NotReady {
            status: IntentStatus::Running,
        },
        TaskError::WrongKind {
            kind: IntentKind::Product,
        },
        TaskError::InvalidContract {
            id: String::new(),
            version: 0,
        },
        TaskError::MissingProductId,
        TaskError::EmptyPolicyGrant,
        TaskError::EmptyMaintenanceRef,
    ];
    for error in errors {
        let text = error.to_string();
        assert!(!text.is_empty(), "每个错误臂都必须有可读的 Display");
        assert!(
            error.source().is_none(),
            "TaskError 没有内层错误，source() 必须是 None"
        );
    }
}
