//! §4.8 的 lower 写路径与交接面。

use continuum_persist::Tx;
use continuum_semantics::{
    ContractRef, Intent, IntentError, IntentId, IntentKind, IntentStatus, Priority,
    apply_intent_transition, load_intent, save_intent,
};

use crate::drive::ProductPhase;
use crate::error::ProductError;
use crate::lower::lower_milestone;
use crate::milestone::{Milestone, MilestoneId};
use crate::persist::{load_milestone, load_product_drive};
use crate::product::Product;

/// lower 的**唯一写路径**：内部重新调 [`lower_milestone`]；同 id 已存在时六项
/// 逐项比较，全同返回现有值（保留 status），任一不同 ⇒ `IntentIdCollision`。
///
/// **不接受调用方拼好的 `Intent`**：`READY` 不能借这条路径绕过审查链。
pub fn save_lowered_intent(
    tx: &Tx<'_>,
    product: &Product,
    milestone: &Milestone,
    contract: ContractRef,
    priority: Priority,
    created_at: i64,
    updated_at: i64,
) -> Result<Intent, ProductError> {
    let intent = lower_milestone(
        product,
        milestone,
        contract,
        priority,
        created_at,
        updated_at,
    )?;

    if let Some(existing) = load_intent(tx, &intent.id)? {
        if existing.kind == IntentKind::Product
            && existing.goal.as_str() == intent.goal.as_str()
            && existing.product_id.as_ref().map(|id| id.as_str())
                == intent.product_id.as_ref().map(|id| id.as_str())
            && existing.parent_intent.as_ref().map(|id| id.as_str())
                == intent.parent_intent.as_ref().map(|id| id.as_str())
            && existing.contract_id.id.as_str() == intent.contract_id.id.as_str()
            && existing.contract_id.version == intent.contract_id.version
            && existing.completion_predicate.as_str()
                == intent.completion_predicate.as_str()
        {
            return Ok(existing);
        }
        return Err(ProductError::IntentIdCollision {
            id: intent.id.as_str().to_owned(),
        });
    }

    save_intent(tx, &intent)?;
    Ok(intent)
}

/// 交接闸门：只把 **READY** 的 lower 产物交出去；其余十二态一律 `Ok(None)`。
///
/// drive gate（C2=(b)）：缺 `product_drive` 行 ⇒ `MissingDriveState`；
/// `PostShippableImprovement` 且开关关闭 ⇒ `Ok(None)`。
pub fn product_intent_for_handoff(
    tx: &Tx<'_>,
    product: &Product,
    milestone: &MilestoneId,
) -> Result<Option<Intent>, ProductError> {
    let Some(found) = load_milestone(tx, milestone)? else {
        return Err(ProductError::UnknownMilestone {
            id: milestone.as_str().to_owned(),
        });
    };
    if found.product_id().as_str() != product.id().as_str() {
        return Err(ProductError::MilestoneProductMismatch {
            milestone: milestone.as_str().to_owned(),
            product: product.id().as_str().to_owned(),
        });
    }

    let intent_id = IntentId::from_str(&format!("intent/{}", milestone.as_str()));
    let Some(intent) = load_intent(tx, &intent_id)? else {
        return Ok(None);
    };
    if intent.kind != IntentKind::Product
        || intent.product_id.as_ref().map(|id| id.as_str()) != Some(product.id().as_str())
        || intent.contract_id.id.as_str().is_empty()
        || intent.contract_id.version < 1
    {
        return Err(ProductError::InvalidLoweredIntent {
            id: intent.id.as_str().to_owned(),
        });
    }
    if intent.status != IntentStatus::Ready {
        return Ok(None);
    }

    let Some(drive) = load_product_drive(tx, product.id())? else {
        return Err(ProductError::MissingDriveState {
            product: product.id().as_str().to_owned(),
        });
    };
    if drive.phase == ProductPhase::PostShippableImprovement
        && !drive.continuous_product_drive.enabled
    {
        return Ok(None);
    }
    Ok(Some(intent))
}

/// `COMPLETED → EVOLVING`：只对 Product Intent 生效，迁移经
/// [`apply_intent_transition`]，不直接写 `intent` 表。
pub fn retire_completed_product_intent(
    tx: &Tx<'_>,
    intent_id: &IntentId,
    occurred_at: i64,
    event_id: &str,
) -> Result<(), ProductError> {
    let Some(intent) = load_intent(tx, intent_id)? else {
        return Err(ProductError::Intent(IntentError::UnknownIntent {
            id: intent_id.as_str().to_owned(),
        }));
    };
    if intent.kind != IntentKind::Product || intent.status != IntentStatus::Completed {
        return Err(ProductError::InvalidLoweredIntent {
            id: intent_id.as_str().to_owned(),
        });
    }
    apply_intent_transition(tx, intent_id, IntentStatus::Evolving, occurred_at, event_id)?;
    Ok(())
}
