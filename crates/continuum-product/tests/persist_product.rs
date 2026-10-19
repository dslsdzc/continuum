//! J2 正例与 J6 的库行读回：`save_product`／`load_product`／`load_product_drive`。
//!
//! 十四条用例：写路径重查锚 Intent 四条不变量；读回逐项复核 baseline／readiness／
//! predicate，六种坏行一律 `CorruptRow`（不取默认值）。

mod support;

use continuum_canonical::Opaque;
use continuum_persist::{Tx, Value};
use continuum_product::{
    BaselineRequest, ProductError, ReadinessDimensionName, load_product, load_product_drive,
    save_product,
};
use continuum_semantics::{IntentError, IntentStatus, save_intent};
use support::{
    FixedBaselineGenerator, anchor_intent, predicate, product_with_intent, save_anchor_and_product,
    saved_product, saved_product_with_intent,
};

fn request() -> BaselineRequest {
    BaselineRequest {
        idea: Opaque::from_str("an idea"),
        constraints: vec![],
    }
}

fn count(tx: &Tx<'_>, table: &str) -> i64 {
    let rows = tx
        .query(&format!("SELECT COUNT(*) FROM {table}"), &[])
        .unwrap();
    match rows[0][0] {
        Value::Int(n) => n,
        ref other => panic!("COUNT 不是整数: {other:?}"),
    }
}

fn insert_milestone(tx: &Tx<'_>, id: &str, product_id: &str, seq: i64, state: &str) {
    tx.execute(
        "INSERT INTO milestone
           (id, product_id, seq, objective, dependencies, entry_conditions,
            exit_conditions, state, created_at, updated_at)
         VALUES (?1, ?2, ?3, 'm', '[]', '[]', '[]', ?4, 100, 100)",
        &[
            Value::text(id),
            Value::text(product_id),
            Value::Int(seq),
            Value::text(state),
        ],
    )
    .unwrap();
}

#[tokio::test]
async fn save_product_reloads_the_anchor_and_requires_ready() {
    let (_dir, db) = support::new_db();
    let (intent, product) = product_with_intent("p1").await;
    let tx = db.begin().unwrap();
    save_anchor_and_product(&tx, &intent, &product, 100);
    assert!(load_product(&tx, product.id()).unwrap().is_some());

    let mut proposed = anchor_intent("p2", IntentStatus::Proposed);
    proposed.updated_at = 120;
    let (_, second) = product_with_intent("p2").await;
    save_intent(&tx, &proposed).unwrap();
    let err = save_product(&tx, &second, 120).unwrap_err();
    assert!(matches!(
        err,
        ProductError::AnchorIntentNotReady { id, status }
            if id == "intent/root" && status == "proposed"
    ));
    assert_eq!(count(&tx, "product"), 1);
}

#[tokio::test]
async fn save_product_rejects_a_missing_anchor_intent() {
    let (_dir, db) = support::new_db();
    let (_, product) = product_with_intent("p1").await;
    let tx = db.begin().unwrap();
    let err = save_product(&tx, &product, 100).unwrap_err();
    assert!(matches!(
        err,
        ProductError::Intent(IntentError::UnknownIntent { id }) if id == "intent/root"
    ));
    assert_eq!(count(&tx, "product"), 0);
}

#[tokio::test]
async fn save_product_rejects_an_anchor_whose_contract_became_invalid() {
    let (_dir, db) = support::new_db();
    let (mut intent, product) = product_with_intent("p1").await;
    let tx = db.begin().unwrap();
    save_intent(&tx, &intent).unwrap();
    intent.contract_id.version = 0;
    save_intent(&tx, &intent).unwrap();
    let err = save_product(&tx, &product, 100).unwrap_err();
    assert!(matches!(
        err,
        ProductError::InvalidContract { intent } if intent == "intent/root"
    ));
    assert_eq!(count(&tx, "product"), 0);
}

#[tokio::test]
async fn a_generator_error_leaves_zero_product_rows() {
    let (_dir, db) = support::new_db();
    let intent = anchor_intent("p1", IntentStatus::Ready);
    let result = continuum_product::establish_baseline(
        &intent,
        &request(),
        predicate(),
        &FixedBaselineGenerator {
            result: Err(continuum_product::PortError::new("boom")),
        },
    )
    .await;
    assert!(result.is_err());
    let tx = db.begin().unwrap();
    save_intent(&tx, &intent).unwrap();
    assert_eq!(count(&tx, "product"), 0);
}

#[tokio::test]
async fn save_product_writes_the_drive_row_disabled_in_pre_shippable() {
    let (_dir, db) = support::new_db();
    let product = saved_product(&db, "p1").await;
    let tx = db.begin().unwrap();
    let drive = load_product_drive(&tx, product.id()).unwrap().unwrap();
    assert!(!drive.continuous_product_drive.enabled);
    assert_eq!(drive.phase, continuum_product::ProductPhase::PreShippable);
}

#[tokio::test]
async fn save_product_is_outer_insert_only() {
    let (_dir, db) = support::new_db();
    let (intent, product) = product_with_intent("p1").await;
    let tx = db.begin().unwrap();
    save_anchor_and_product(&tx, &intent, &product, 100);
    let err = save_product(&tx, &product, 999).unwrap_err();
    assert!(matches!(err, ProductError::ProductExists { id } if id == "p1"));
    let rows = tx
        .query("SELECT created_at, updated_at FROM product WHERE id = 'p1'", &[])
        .unwrap();
    assert_eq!(rows[0][0], Value::Int(100));
    assert_eq!(rows[0][1], Value::Int(100));
}

#[tokio::test]
async fn a_saved_product_round_trips_its_baseline_readiness_and_predicate() {
    let (_dir, db) = support::new_db();
    let product = saved_product(&db, "p1").await;
    let tx = db.begin().unwrap();
    let loaded = load_product(&tx, product.id()).unwrap().unwrap();

    assert_eq!(loaded.id().as_str(), "p1");
    assert_eq!(loaded.intent_id().as_str(), "intent/root");
    assert_eq!(loaded.baseline().product_goal.as_str(), "ship a product");
    assert_eq!(loaded.baseline().target_users[0].as_str(), "developers");
    assert_eq!(loaded.baseline().required_capabilities[0].as_str(), "build");
    assert_eq!(loaded.baseline().hard_constraints[0].as_str(), "offline");
    assert_eq!(loaded.baseline().architecture.as_str(), "library");
    assert_eq!(loaded.baseline().technology_choices[0].as_str(), "rust");
    assert_eq!(
        loaded.baseline().minimum_complete_system[0].as_str(),
        "cli"
    );
    assert_eq!(loaded.baseline().known_unknowns[0].as_str(), "demand");
    assert_eq!(loaded.baseline().engineering_risks[0].as_str(), "scope");
    assert_eq!(
        loaded.baseline().acceptance_criteria[0].as_str(),
        "tests pass"
    );

    for (_, dimension) in loaded.readiness().iter() {
        assert!(dimension.value().is_unassessed());
        assert!(dimension.evidence().is_empty());
        assert_eq!(dimension.last_verified(), None);
    }
    assert_eq!(
        loaded.shippable_predicate().required_dimensions(),
        [ReadinessDimensionName::Functional]
    );
    assert_eq!(loaded.shippable_predicate().clauses()[0].id().as_str(), "c1");
    assert_eq!(
        loaded.shippable_predicate().clauses()[0].statement().as_str(),
        "works"
    );
    assert_eq!(loaded.current_milestone(), None);
    assert!(loaded.milestone_history().is_empty());
}

#[tokio::test]
async fn load_product_returns_none_for_an_unknown_id() {
    let (_dir, db) = support::new_db();
    let tx = db.begin().unwrap();
    let none = load_product(&tx, &continuum_semantics::ProductIdRef::from_str("nope")).unwrap();
    assert!(none.is_none());
}

#[tokio::test]
async fn load_product_rejects_an_empty_baseline_row() {
    let (_dir, db) = support::new_db();
    let product = saved_product(&db, "p1").await;
    let tx = db.begin().unwrap();
    let baseline = serde_json::json!({
        "product_goal": "",
        "target_users": ["developers"],
        "required_capabilities": ["build"],
        "hard_constraints": ["offline"],
        "architecture": "library",
        "technology_choices": ["rust"],
        "minimum_complete_system": ["cli"],
        "known_unknowns": ["demand"],
        "engineering_risks": ["scope"],
        "acceptance_criteria": ["tests pass"]
    });
    tx.execute(
        "UPDATE product SET baseline = ?2 WHERE id = ?1",
        &[
            Value::text(product.id().as_str()),
            Value::text(baseline.to_string()),
        ],
    )
    .unwrap();
    let err = load_product(&tx, product.id()).unwrap_err();
    assert!(matches!(
        err,
        ProductError::CorruptRow { table: "product", .. }
    ));
}

#[tokio::test]
async fn load_product_rejects_a_predicate_with_a_duplicate_dimension() {
    let (_dir, db) = support::new_db();
    let product = saved_product(&db, "p1").await;
    let tx = db.begin().unwrap();
    let predicate = serde_json::json!({
        "required_dimensions": ["functional", "functional"],
        "clauses": [{"id": "c1", "statement": "works"}]
    });
    tx.execute(
        "UPDATE product SET shippable_predicate = ?2 WHERE id = ?1",
        &[
            Value::text(product.id().as_str()),
            Value::text(predicate.to_string()),
        ],
    )
    .unwrap();
    let err = load_product(&tx, product.id()).unwrap_err();
    assert!(matches!(
        err,
        ProductError::CorruptRow { table: "product", .. }
    ));
}

#[tokio::test]
async fn load_product_rejects_an_assessed_dimension_without_evidence() {
    let (_dir, db) = support::new_db();
    let product = saved_product(&db, "p1").await;
    let tx = db.begin().unwrap();
    let readiness = readiness_json("functional", r#"{"value":"0.8","evidence":[],"last_verified":100}"#);
    tx.execute(
        "UPDATE product SET readiness = ?2 WHERE id = ?1",
        &[
            Value::text(product.id().as_str()),
            Value::text(readiness),
        ],
    )
    .unwrap();
    let err = load_product(&tx, product.id()).unwrap_err();
    assert!(matches!(
        err,
        ProductError::CorruptRow { table: "product", .. }
    ));
}

#[tokio::test]
async fn load_product_rejects_an_assessed_dimension_without_last_verified() {
    let (_dir, db) = support::new_db();
    let product = saved_product(&db, "p1").await;
    let tx = db.begin().unwrap();
    let readiness = readiness_json(
        "functional",
        r#"{"value":"0.8","evidence":["e1"],"last_verified":null}"#,
    );
    tx.execute(
        "UPDATE product SET readiness = ?2 WHERE id = ?1",
        &[
            Value::text(product.id().as_str()),
            Value::text(readiness),
        ],
    )
    .unwrap();
    let err = load_product(&tx, product.id()).unwrap_err();
    assert!(matches!(
        err,
        ProductError::CorruptRow { table: "product", .. }
    ));
}

#[tokio::test]
async fn load_product_rejects_a_current_milestone_from_another_product() {
    let (_dir, db) = support::new_db();
    let first = saved_product(&db, "p1").await;
    let _second = saved_product_with_intent(&db, "p2", "intent/p2").await;
    let tx = db.begin().unwrap();
    insert_milestone(&tx, "milestone/p2/0", "p2", 0, "proposed");
    tx.execute(
        "UPDATE product SET current_milestone = 'milestone/p2/0' WHERE id = 'p1'",
        &[],
    )
    .unwrap();
    let err = load_product(&tx, first.id()).unwrap_err();
    assert!(matches!(
        err,
        ProductError::CorruptRow { table: "product", .. }
    ));
}

#[tokio::test]
async fn load_product_rejects_a_current_milestone_in_a_terminal_state() {
    let (_dir, db) = support::new_db();
    let product = saved_product(&db, "p1").await;
    let tx = db.begin().unwrap();
    insert_milestone(&tx, "milestone/p1/0", "p1", 0, "completed");
    tx.execute(
        "UPDATE product SET current_milestone = 'milestone/p1/0' WHERE id = 'p1'",
        &[],
    )
    .unwrap();
    let err = load_product(&tx, product.id()).unwrap_err();
    assert!(matches!(
        err,
        ProductError::CorruptRow { table: "product", .. }
    ));
}

/// 九个维度中把 `assessed` 那一维替换成给定 JSON，其余八维保持未评。
fn readiness_json(assessed: &str, dimension: &str) -> String {
    let names = [
        "functional",
        "correctness",
        "reliability",
        "performance",
        "security",
        "usability",
        "maintainability",
        "deployability",
        "documentation",
    ];
    let mut object = serde_json::Map::new();
    for name in names {
        let value: serde_json::Value = if name == assessed {
            serde_json::from_str(dimension).unwrap()
        } else {
            serde_json::json!({"value": null, "evidence": [], "last_verified": null})
        };
        object.insert(name.to_owned(), value);
    }
    serde_json::Value::Object(object).to_string()
}
