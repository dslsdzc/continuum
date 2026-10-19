//! §4.8 的确定 lower：Milestone → 一枚 `PROPOSED` 的 Product Intent。
//!
//! **不跑 Canonicalizer**、不读生成端口；同一 Milestone 的两次 lower 字段全同。
//! 写路径只有 [`crate::save_lowered_intent`]，它不接受调用方拼好的 `Intent`。

use continuum_canonical::{EntityId, Opaque};
use continuum_semantics::{ContractRef, Intent, IntentId, IntentKind, IntentStatus, Priority};

use crate::error::ProductError;
use crate::milestone::Milestone;
use crate::product::Product;

/// 由 Product 与 Milestone 确定地构造一枚 `PROPOSED` Product Intent。
///
/// 校验顺序：跨 Product ⇒ `MilestoneProductMismatch`；空 objective ⇒
/// `EmptyMilestoneObjective`；无效 contract ⇒ `InvalidContract`。
pub fn lower_milestone(
    product: &Product,
    milestone: &Milestone,
    contract: ContractRef,
    priority: Priority,
    created_at: i64,
    updated_at: i64,
) -> Result<Intent, ProductError> {
    if milestone.product_id().as_str() != product.id().as_str() {
        return Err(ProductError::MilestoneProductMismatch {
            milestone: milestone.id().as_str().to_owned(),
            product: product.id().as_str().to_owned(),
        });
    }
    if milestone.objective().as_str().is_empty() {
        return Err(ProductError::EmptyMilestoneObjective);
    }
    let id = IntentId::from_str(&format!("intent/{}", milestone.id().as_str()));
    if contract.id.as_str().is_empty() || contract.version < 1 {
        return Err(ProductError::InvalidContract {
            intent: id.as_str().to_owned(),
        });
    }

    let completion_predicate = Opaque::from_str(
        &milestone
            .exit_conditions()
            .iter()
            .map(Opaque::as_str)
            .collect::<Vec<_>>()
            .join("\n"),
    );

    Ok(Intent {
        id,
        version: 1,
        kind: IntentKind::Product,
        goal: EntityId::from_str(milestone.objective().as_str()),
        status: IntentStatus::Proposed,
        priority,
        created_at,
        updated_at,
        parent_intent: Some(product.intent_id().clone()),
        contract_id: contract,
        active_plan_id: None,
        graph_id: None,
        product_id: Some(product.id().clone()),
        completion_predicate,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::baseline::EngineeringBaseline;
    use crate::milestone::{MilestoneId, MilestoneState};
    use crate::predicate::{ShippableClause, ShippableClauseId, ShippablePredicate};
    use crate::readiness::{ProductReadiness, ReadinessDimensionName};
    use continuum_semantics::{ContractId, IntentId, ProductIdRef};

    fn baseline() -> EngineeringBaseline {
        EngineeringBaseline {
            product_goal: Opaque::from_str("ship"),
            target_users: vec![Opaque::from_str("devs")],
            required_capabilities: vec![Opaque::from_str("build")],
            hard_constraints: vec![Opaque::from_str("offline")],
            architecture: Opaque::from_str("library"),
            technology_choices: vec![Opaque::from_str("rust")],
            minimum_complete_system: vec![Opaque::from_str("cli")],
            known_unknowns: vec![Opaque::from_str("demand")],
            engineering_risks: vec![Opaque::from_str("scope")],
            acceptance_criteria: vec![Opaque::from_str("tests")],
        }
    }

    fn product() -> Product {
        Product::from_verified_parts(
            ProductIdRef::from_str("p"),
            IntentId::from_str("intent/p"),
            baseline(),
            ProductReadiness::unassessed(),
            None,
            vec![],
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
            .unwrap(),
        )
        .unwrap()
    }

    fn contract() -> ContractRef {
        ContractRef {
            id: ContractId::from_str("c"),
            version: 1,
        }
    }

    #[test]
    fn lower_milestone_rejects_an_empty_objective() {
        let milestone = Milestone::for_test(
            MilestoneId::from_str("milestone/p/0"),
            ProductIdRef::from_str("p"),
            Opaque::from_str(""),
            vec![],
            vec![],
            vec![],
            MilestoneState::Proposed,
        );
        let err = lower_milestone(
            &product(),
            &milestone,
            contract(),
            Priority::from_str("normal"),
            1,
            2,
        )
        .unwrap_err();
        assert!(matches!(err, ProductError::EmptyMilestoneObjective));
    }
}
