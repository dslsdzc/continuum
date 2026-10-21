//! Task 3：`AuthorizedTask` 的四个构造器与校验顺序。

mod support;

use continuum_semantics::{
    ContractId, ContractRef, Intent, IntentKind, IntentStatus, ProductIdRef,
};
use continuum_supervisor::{
    AuthorizedTask, MaintenanceOccurrenceRef, PolicyGrantRef, TaskError, TaskSource,
};

use support::{code_only, intent, product_intent, product_task, task};

#[test]
fn product_drive_accepts_only_ready_across_the_thirteen_states() {
    let statuses = [
        IntentStatus::Proposed,
        IntentStatus::Specifying,
        IntentStatus::Planning,
        IntentStatus::Review,
        IntentStatus::Ready,
        IntentStatus::Running,
        IntentStatus::Waiting,
        IntentStatus::Suspended,
        IntentStatus::Verifying,
        IntentStatus::Completed,
        IntentStatus::Evolving,
        IntentStatus::Failed,
        IntentStatus::Cancelled,
    ];
    for status in statuses {
        let result = AuthorizedTask::product_drive(product_intent(status));
        if status == IntentStatus::Ready {
            assert!(result.is_ok(), "READY 的 Product Intent 必须通过");
        } else {
            assert_eq!(
                result.unwrap_err(),
                TaskError::NotReady { status },
                "非 READY 的 Product Intent 必须落 NotReady 且保留输入状态"
            );
        }
    }
}

#[test]
fn product_drive_rejects_running_constructed_directly() {
    let running = Intent {
        status: IntentStatus::Running,
        ..product_intent(IntentStatus::Ready)
    };
    assert_eq!(
        AuthorizedTask::product_drive(running).unwrap_err(),
        TaskError::NotReady {
            status: IntentStatus::Running
        }
    );
}

#[test]
fn authorized_intent_rejects_product_and_product_drive_rejects_non_product() {
    assert_eq!(
        AuthorizedTask::authorized_intent(product_intent(IntentStatus::Ready)).unwrap_err(),
        TaskError::WrongKind {
            kind: IntentKind::Product
        }
    );
    assert_eq!(
        AuthorizedTask::product_drive(intent(IntentKind::Task, IntentStatus::Ready)).unwrap_err(),
        TaskError::WrongKind {
            kind: IntentKind::Task
        }
    );
    let task = AuthorizedTask::authorized_intent(intent(IntentKind::Task, IntentStatus::Ready));
    assert!(task.is_ok(), "非 Product 的 READY Intent 必须通过");
}

#[test]
fn contract_and_product_id_are_validated_in_order() {
    let empty_id = Intent {
        contract_id: ContractRef {
            id: ContractId::from_str(""),
            version: 1,
        },
        ..product_intent(IntentStatus::Ready)
    };
    assert_eq!(
        AuthorizedTask::product_drive(empty_id).unwrap_err(),
        TaskError::InvalidContract {
            id: String::new(),
            version: 1,
        }
    );

    let zero_version = Intent {
        contract_id: ContractRef {
            id: ContractId::from_str("contract-1"),
            version: 0,
        },
        ..product_intent(IntentStatus::Ready)
    };
    assert_eq!(
        AuthorizedTask::product_drive(zero_version).unwrap_err(),
        TaskError::InvalidContract {
            id: "contract-1".to_owned(),
            version: 0,
        }
    );

    let missing_product = Intent {
        product_id: None,
        ..product_intent(IntentStatus::Ready)
    };
    assert_eq!(
        AuthorizedTask::product_drive(missing_product).unwrap_err(),
        TaskError::MissingProductId
    );

    let empty_product = Intent {
        product_id: Some(ProductIdRef::from_str("")),
        ..product_intent(IntentStatus::Ready)
    };
    assert_eq!(
        AuthorizedTask::product_drive(empty_product).unwrap_err(),
        TaskError::MissingProductId
    );

    let empty_contract = Intent {
        contract_id: ContractRef {
            id: ContractId::from_str(""),
            version: 1,
        },
        ..intent(IntentKind::Task, IntentStatus::Ready)
    };
    assert!(matches!(
        AuthorizedTask::authorized_intent(empty_contract).unwrap_err(),
        TaskError::InvalidContract { .. }
    ));
    let zero_contract = Intent {
        contract_id: ContractRef {
            id: ContractId::from_str("contract-1"),
            version: 0,
        },
        ..intent(IntentKind::Task, IntentStatus::Ready)
    };
    assert!(matches!(
        AuthorizedTask::authorized_intent(zero_contract).unwrap_err(),
        TaskError::InvalidContract { .. }
    ));

    let running_empty_contract = Intent {
        status: IntentStatus::Running,
        contract_id: ContractRef {
            id: ContractId::from_str(""),
            version: 0,
        },
        ..product_intent(IntentStatus::Ready)
    };
    assert_eq!(
        AuthorizedTask::product_drive(running_empty_contract).unwrap_err(),
        TaskError::NotReady {
            status: IntentStatus::Running
        },
        "kind 与 product_id 先于 status，status 先于 contract"
    );
}

#[test]
fn authorized_intent_accessors_match_the_evidence() {
    let task = task(intent(IntentKind::Task, IntentStatus::Ready));
    assert_eq!(task.source(), TaskSource::AuthorizedIntent);
    assert_eq!(
        task.intent().expect("授权 Intent 必须带意图").id.as_str(),
        "intent-1"
    );
    let contract = task.contract().expect("授权 Intent 必须带合同");
    assert_eq!(contract.id.as_str(), "contract-1");
    assert_eq!(contract.version, 1);
    assert!(task.policy_grant().is_none());
    assert!(task.maintenance_schedule().is_none());
}

#[test]
fn policy_and_maintenance_tasks_have_no_intent_or_contract() {
    let grant = PolicyGrantRef::new("issuer", "grant").expect("非空引用");
    let policy = AuthorizedTask::policy(grant.clone()).expect("policy 任务");
    assert_eq!(policy.source(), TaskSource::Policy);
    assert!(policy.intent().is_none());
    assert!(policy.contract().is_none());
    assert_eq!(policy.policy_grant(), Some(&grant));
    assert!(policy.maintenance_schedule().is_none());

    let occurrence =
        MaintenanceOccurrenceRef::new("schedule", "occurrence").expect("非空引用");
    let maintenance =
        AuthorizedTask::maintenance(occurrence.clone()).expect("maintenance 任务");
    assert_eq!(maintenance.source(), TaskSource::MaintenanceSchedule);
    assert!(maintenance.intent().is_none());
    assert!(maintenance.contract().is_none());
    assert_eq!(
        maintenance.maintenance_schedule(),
        Some(&occurrence)
    );
    assert!(maintenance.policy_grant().is_none());

    let cloned_policy = policy.clone();
    assert_eq!(cloned_policy.source(), TaskSource::Policy);
    assert_eq!(cloned_policy.policy_grant(), Some(&grant));

    let cloned_maintenance = maintenance.clone();
    assert_eq!(
        cloned_maintenance.source(),
        TaskSource::MaintenanceSchedule
    );
    assert_eq!(
        cloned_maintenance.maintenance_schedule(),
        Some(&occurrence)
    );
}

#[test]
fn product_drive_reads_product_id_only_from_intent() {
    let missing = Intent {
        product_id: None,
        ..product_intent(IntentStatus::Ready)
    };
    assert_eq!(
        AuthorizedTask::product_drive(missing).unwrap_err(),
        TaskError::MissingProductId
    );

    let empty = Intent {
        product_id: Some(ProductIdRef::from_str("")),
        ..product_intent(IntentStatus::Ready)
    };
    assert_eq!(
        AuthorizedTask::product_drive(empty).unwrap_err(),
        TaskError::MissingProductId
    );

    let valid = product_task(IntentStatus::Ready);
    assert_eq!(valid.source(), TaskSource::ProductDrive);
    assert_eq!(
        valid
            .intent()
            .and_then(|value| value.product_id.as_ref())
            .map(ProductIdRef::as_str),
        Some("product-1")
    );

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
    for banned in ["continuum_product", "Product::", "Milestone", "load_product"] {
        assert!(
            !corpus.contains(banned),
            "product drive 只能从 Intent.product_id 读值，源码面不应出现 {banned}"
        );
    }
}
