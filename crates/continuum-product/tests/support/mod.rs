#![allow(dead_code)]

//! P7A 测试夹具：锚 Intent、假 baseline 生成器、可用基线与谓词。

use continuum_canonical::{EntityId, Opaque};
use continuum_product::{
    BaselineRequest, EngineeringBaseline, EngineeringBaselineGenerator, PortError,
    ReadinessDimensionName, ShippableClause, ShippableClauseId, ShippablePredicate,
};
use continuum_semantics::{
    ContractId, ContractRef, Intent, IntentId, IntentKind, IntentStatus, Priority, ProductIdRef,
};

pub fn anchor_intent(product_id: &str, status: IntentStatus) -> Intent {
    Intent {
        id: IntentId::from_str("intent/root"),
        version: 1,
        kind: IntentKind::Product,
        goal: EntityId::from_str("goal"),
        status,
        priority: Priority::from_str("normal"),
        created_at: 10,
        updated_at: 20,
        parent_intent: None,
        contract_id: ContractRef {
            id: ContractId::from_str("c1"),
            version: 1,
        },
        active_plan_id: None,
        graph_id: None,
        product_id: Some(ProductIdRef::from_str(product_id)),
        completion_predicate: Opaque::from_str(""),
    }
}

pub fn usable_baseline() -> EngineeringBaseline {
    EngineeringBaseline {
        product_goal: Opaque::from_str("ship a product"),
        target_users: vec![Opaque::from_str("developers")],
        required_capabilities: vec![Opaque::from_str("build")],
        hard_constraints: vec![Opaque::from_str("offline")],
        architecture: Opaque::from_str("library"),
        technology_choices: vec![Opaque::from_str("rust")],
        minimum_complete_system: vec![Opaque::from_str("cli")],
        known_unknowns: vec![Opaque::from_str("demand")],
        engineering_risks: vec![Opaque::from_str("scope")],
        acceptance_criteria: vec![Opaque::from_str("tests pass")],
    }
}

pub fn predicate() -> ShippablePredicate {
    ShippablePredicate::new(
        vec![ReadinessDimensionName::Functional],
        vec![
            ShippableClause::new(
                ShippableClauseId::from_str("c1"),
                Opaque::from_str("works"),
            )
            .unwrap(),
        ],
    )
    .unwrap()
}

pub struct FixedBaselineGenerator {
    pub result: Result<EngineeringBaseline, PortError>,
}

#[async_trait::async_trait]
impl EngineeringBaselineGenerator for FixedBaselineGenerator {
    async fn generate(&self, _request: &BaselineRequest) -> Result<EngineeringBaseline, PortError> {
        self.result.clone()
    }
}
