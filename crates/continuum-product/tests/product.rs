//! §231 `Product` 的七字段、只读访问器与 `establish_baseline` 的唯一创建路径。

mod support;

use continuum_canonical::Opaque;
use continuum_product::{
    BaselineRequest, EngineeringBaseline, Product, ProductError, ReadinessDimensionName,
    ShippableClauseId, ShippablePredicate,
};
use continuum_semantics::{IntentKind, IntentStatus};
use support::{FixedBaselineGenerator, anchor_intent, predicate, usable_baseline};

fn request() -> BaselineRequest {
    BaselineRequest {
        idea: Opaque::from_str("an idea"),
        constraints: vec![Opaque::from_str("offline")],
    }
}

fn generator(result: Result<EngineeringBaseline, continuum_product::PortError>) -> FixedBaselineGenerator {
    FixedBaselineGenerator { result }
}

async fn establish(status: IntentStatus) -> Result<Product, (ProductError, continuum_semantics::Intent)> {
    let intent = anchor_intent("p1", status);
    let result = continuum_product::establish_baseline(
        &intent,
        &request(),
        predicate(),
        &generator(Ok(usable_baseline())),
    )
    .await;
    result.map_err(|e| (e, intent))
}

#[tokio::test]
async fn establish_baseline_derives_the_product_id_from_the_anchor_intent() {
    let intent = anchor_intent("p1", IntentStatus::Ready);
    let product = continuum_product::establish_baseline(
        &intent,
        &request(),
        predicate(),
        &generator(Ok(usable_baseline())),
    )
    .await
    .unwrap();

    assert_eq!(product.id().as_str(), "p1");
    assert_eq!(product.intent_id().as_str(), "intent/root");
    assert_eq!(product.baseline().product_goal.as_str(), "ship a product");
    assert_eq!(product.baseline().minimum_complete_system.len(), 1);
    for (name, d) in product.readiness().iter() {
        assert!(d.value().is_unassessed(), "{name:?} 应为 Unassessed");
    }
    assert_eq!(product.current_milestone(), None);
    assert!(product.milestone_history().is_empty());
    assert_eq!(
        product.shippable_predicate().required_dimensions(),
        [ReadinessDimensionName::Functional]
    );
    assert_eq!(product.shippable_predicate().clauses().len(), 1);
}

#[tokio::test]
async fn establish_baseline_rejects_a_task_intent() {
    let mut intent = anchor_intent("p1", IntentStatus::Ready);
    intent.kind = IntentKind::Task;
    let err = continuum_product::establish_baseline(
        &intent,
        &request(),
        predicate(),
        &generator(Ok(usable_baseline())),
    )
    .await
    .unwrap_err();
    assert!(matches!(err, ProductError::IntentNotProduct { id } if id == "intent/root"));
}

#[tokio::test]
async fn establish_baseline_rejects_a_missing_product_id() {
    let mut intent = anchor_intent("p1", IntentStatus::Ready);
    intent.product_id = None;
    let err = continuum_product::establish_baseline(
        &intent,
        &request(),
        predicate(),
        &generator(Ok(usable_baseline())),
    )
    .await
    .unwrap_err();
    assert!(matches!(err, ProductError::MissingProductId { intent } if intent == "intent/root"));
}

#[tokio::test]
async fn establish_baseline_rejects_an_empty_product_id() {
    let err = establish_with(anchor_intent("", IntentStatus::Ready)).await;
    assert!(matches!(err, ProductError::MissingProductId { .. }));
}

#[tokio::test]
async fn establish_baseline_rejects_an_invalid_contract() {
    let mut intent = anchor_intent("p1", IntentStatus::Ready);
    intent.contract_id.version = 0;
    let err = establish_with(intent).await;
    assert!(matches!(err, ProductError::InvalidContract { intent } if intent == "intent/root"));

    let mut intent = anchor_intent("p1", IntentStatus::Ready);
    intent.contract_id.id = continuum_semantics::ContractId::from_str("");
    let err = establish_with(intent).await;
    assert!(matches!(err, ProductError::InvalidContract { .. }));
}

#[tokio::test]
async fn establish_baseline_rejects_a_proposed_anchor() {
    let err = establish_with(anchor_intent("p1", IntentStatus::Proposed)).await;
    assert!(matches!(
        err,
        ProductError::AnchorIntentNotReady { id, status } if id == "intent/root" && status == "proposed"
    ));
}

#[tokio::test]
async fn establish_baseline_rejects_a_running_anchor() {
    let err = establish_with(anchor_intent("p1", IntentStatus::Running)).await;
    assert!(matches!(
        err,
        ProductError::AnchorIntentNotReady { status, .. } if status == "running"
    ));
}

#[tokio::test]
async fn a_generator_error_produces_no_product() {
    let intent = anchor_intent("p1", IntentStatus::Ready);
    let err = continuum_product::establish_baseline(
        &intent,
        &request(),
        predicate(),
        &generator(Err(continuum_product::PortError::new("boom"))),
    )
    .await
    .unwrap_err();
    assert!(matches!(err, ProductError::Port(e) if e.message() == "boom"));
}

#[tokio::test]
async fn an_unusable_baseline_is_rejected() {
    let mut goal_empty = usable_baseline();
    goal_empty.product_goal = Opaque::from_str("");
    let err = continuum_product::establish_baseline(
        &anchor_intent("p1", IntentStatus::Ready),
        &request(),
        predicate(),
        &generator(Ok(goal_empty)),
    )
    .await
    .unwrap_err();
    assert!(matches!(
        err,
        ProductError::BaselineUnusable {
            missing: "product_goal"
        }
    ));

    let mut minimum_empty = usable_baseline();
    minimum_empty.minimum_complete_system = vec![];
    let err = continuum_product::establish_baseline(
        &anchor_intent("p1", IntentStatus::Ready),
        &request(),
        predicate(),
        &generator(Ok(minimum_empty)),
    )
    .await
    .unwrap_err();
    assert!(matches!(
        err,
        ProductError::BaselineUnusable {
            missing: "minimum_complete_system"
        }
    ));
}

async fn establish_with(intent: continuum_semantics::Intent) -> ProductError {
    continuum_product::establish_baseline(
        &intent,
        &request(),
        predicate(),
        &generator(Ok(usable_baseline())),
    )
    .await
    .unwrap_err()
}

#[test]
fn the_product_has_exactly_seven_read_only_accessors() {
    let _: for<'a> fn(&'a Product) -> &'a continuum_semantics::ProductIdRef = Product::id;
    let _: for<'a> fn(&'a Product) -> &'a continuum_semantics::IntentId = Product::intent_id;
    let _: for<'a> fn(&'a Product) -> &'a EngineeringBaseline = Product::baseline;
    let _: for<'a> fn(&'a Product) -> &'a continuum_product::ProductReadiness =
        Product::readiness;
    let _: for<'a> fn(&'a Product) -> Option<&'a continuum_product::MilestoneId> =
        Product::current_milestone;
    let _: for<'a> fn(&'a Product) -> &'a [continuum_product::MilestoneId] =
        Product::milestone_history;
    let _: for<'a> fn(&'a Product) -> &'a ShippablePredicate = Product::shippable_predicate;

    let src = include_str!("../src/product.rs");
    assert!(!src.contains("&mut self"), "访问器不应有可变接收者");
    assert!(!src.contains("pub fn into_parts"));
    assert!(!src.contains("pub id:"), "Product 字段必须私有");
}

#[test]
fn the_product_fields_are_verbatim_in_order() {
    let src = include_str!("../src/product.rs");
    let start = src.find("pub struct Product {").expect("找不到 Product") + "pub struct Product {".len();
    let rest = &src[start..];
    let end = rest.find("\n}").expect("Product 没有闭合");
    let fields: Vec<String> = rest[..end]
        .lines()
        .filter_map(|line| {
            let t = line.trim();
            if t.is_empty() || t.starts_with("//") || t.starts_with("#[") {
                None
            } else {
                Some(t.split(':').next().unwrap().trim().to_owned())
            }
        })
        .collect();
    assert_eq!(
        fields,
        [
            "id",
            "intent_id",
            "baseline",
            "readiness",
            "current_milestone",
            "milestone_history",
            "shippable_predicate",
        ]
    );
}

#[test]
fn the_product_has_no_public_constructor_or_deserialize() {
    let src = include_str!("../src/product.rs");
    assert!(!src.contains("pub fn new"));
    assert!(!src.contains("pub fn from_parts"));
    assert!(!src.contains("impl Deserialize"));
    assert!(!src.contains("Serialize for Product"));
    assert!(!src.contains("Deserialize for Product"));
}

#[tokio::test]
async fn a_product_cannot_be_built_from_a_public_literal_or_default() {
    // 编译期照片由 tests/compile_fail 下三份样例钉住；这里只做一次正控制，
    // 证明 establish_baseline 是可得值的唯一公开路径。
    let product = establish(IntentStatus::Ready).await.unwrap();
    assert_eq!(product.id().as_str(), "p1");
    let _ = ShippableClauseId::from_str("c1");
}
