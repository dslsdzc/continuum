//! §4.8 的 lower 写路径与交接面。

use continuum_persist::Tx;
use continuum_semantics::{ContractRef, Intent, IntentKind, Priority, load_intent, save_intent};

use crate::error::ProductError;
use crate::lower::lower_milestone;
use crate::milestone::Milestone;
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
