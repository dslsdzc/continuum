//! §231 的 `Product` 与唯一公开创建路径 `establish_baseline`。
//!
//! 字段私有、只派生 `Debug, Clone`；不实现 `Default`／`Deserialize`。
//! 唯一公开创建路径：`establish_baseline`（异步生成）＋ `save_product` 落库。
//! crate 私有唯一构造点是 [`Product::from_verified_parts`]。

use continuum_semantics::{Intent, IntentId, IntentKind, IntentStatus, ProductIdRef};

use crate::baseline::{
    BaselineRequest, EngineeringBaseline, EngineeringBaselineGenerator,
};
use crate::error::ProductError;
use crate::milestone::MilestoneId;
use crate::predicate::ShippablePredicate;
use crate::readiness::ProductReadiness;

/// §231 的七个字段。字段名与书写序照规范逐字；字段私有。
#[derive(Debug, Clone)]
pub struct Product {
    id: ProductIdRef,
    intent_id: IntentId,
    baseline: EngineeringBaseline,
    readiness: ProductReadiness,
    current_milestone: Option<MilestoneId>,
    milestone_history: Vec<MilestoneId>,
    shippable_predicate: ShippablePredicate,
}

impl Product {
    pub fn id(&self) -> &ProductIdRef {
        &self.id
    }

    pub fn intent_id(&self) -> &IntentId {
        &self.intent_id
    }

    pub fn baseline(&self) -> &EngineeringBaseline {
        &self.baseline
    }

    pub fn readiness(&self) -> &ProductReadiness {
        &self.readiness
    }

    pub fn current_milestone(&self) -> Option<&MilestoneId> {
        self.current_milestone.as_ref()
    }

    pub fn milestone_history(&self) -> &[MilestoneId] {
        &self.milestone_history
    }

    pub fn shippable_predicate(&self) -> &ShippablePredicate {
        &self.shippable_predicate
    }

    /// crate 内唯一构造点：`establish_baseline` 与 `persist::row_to_product` 调用。
    ///
    /// 校验：id／intent_id 非空；baseline 可用；history 无重复；current（若有）
    /// 必须在 history 里。**不跨表回查 Intent**——1–4 的写路径不变量在
    /// `establish_baseline`／`save_product` 核。
    pub(crate) fn from_verified_parts(
        id: ProductIdRef,
        intent_id: IntentId,
        baseline: EngineeringBaseline,
        readiness: ProductReadiness,
        current_milestone: Option<MilestoneId>,
        milestone_history: Vec<MilestoneId>,
        shippable_predicate: ShippablePredicate,
    ) -> Result<Self, ProductError> {
        if id.as_str().is_empty() {
            return Err(ProductError::MissingProductId {
                intent: intent_id.as_str().to_owned(),
            });
        }
        if intent_id.as_str().is_empty() {
            return Err(ProductError::MissingProductId {
                intent: intent_id.as_str().to_owned(),
            });
        }
        baseline.is_usable()?;
        for (i, m) in milestone_history.iter().enumerate() {
            if milestone_history[..i].contains(m) {
                return Err(ProductError::MilestoneIdCollision {
                    id: m.as_str().to_owned(),
                });
            }
        }
        if let Some(current) = &current_milestone {
            if !milestone_history.contains(current) {
                return Err(ProductError::UnknownMilestone {
                    id: current.as_str().to_owned(),
                });
            }
        }
        Ok(Self {
            id,
            intent_id,
            baseline,
            readiness,
            current_milestone,
            milestone_history,
            shippable_predicate,
        })
    }
}

/// 校验锚 Intent 的四条写路径不变量，再向生成端口要基线。
///
/// 校验顺序：kind ⇒ product_id ⇒ contract ⇒ status ⇒ 生成 ⇒ baseline 可用 ⇒ 构造。
pub async fn establish_baseline(
    intent: &Intent,
    request: &BaselineRequest,
    shippable_predicate: ShippablePredicate,
    generator: &dyn EngineeringBaselineGenerator,
) -> Result<Product, ProductError> {
    if intent.kind != IntentKind::Product {
        return Err(ProductError::IntentNotProduct {
            id: intent.id.as_str().to_owned(),
        });
    }
    let product_id = match &intent.product_id {
        Some(id) if !id.as_str().is_empty() => id.clone(),
        _ => {
            return Err(ProductError::MissingProductId {
                intent: intent.id.as_str().to_owned(),
            });
        }
    };
    if intent.contract_id.id.as_str().is_empty() || intent.contract_id.version < 1 {
        return Err(ProductError::InvalidContract {
            intent: intent.id.as_str().to_owned(),
        });
    }
    if intent.status != IntentStatus::Ready {
        return Err(ProductError::AnchorIntentNotReady {
            id: intent.id.as_str().to_owned(),
            status: status_name(intent.status),
        });
    }

    let baseline = generator.generate(request).await?;
    baseline.is_usable()?;

    Product::from_verified_parts(
        product_id,
        intent.id.clone(),
        baseline,
        ProductReadiness::unassessed(),
        None,
        Vec::new(),
        shippable_predicate,
    )
}

/// 只用于错误消息，不用于落库。
pub(crate) fn status_name(status: IntentStatus) -> &'static str {
    match status {
        IntentStatus::Proposed => "proposed",
        IntentStatus::Specifying => "specifying",
        IntentStatus::Planning => "planning",
        IntentStatus::Review => "review",
        IntentStatus::Ready => "ready",
        IntentStatus::Running => "running",
        IntentStatus::Waiting => "waiting",
        IntentStatus::Suspended => "suspended",
        IntentStatus::Verifying => "verifying",
        IntentStatus::Completed => "completed",
        IntentStatus::Failed => "failed",
        IntentStatus::Cancelled => "cancelled",
        IntentStatus::Evolving => "evolving",
    }
}

/// 测试专用构造点。**只用于 J6 的 `BaselineUnusable` 防御面**；生产路径不经过它。
/// Task 17 的 J5 允许清单把这一处列为 `#[cfg(test)]` 例外。
#[cfg(test)]
impl Product {
    pub(crate) fn for_test_with_unusable_baseline(
        shippable_predicate: ShippablePredicate,
    ) -> Self {
        Self {
            id: ProductIdRef::from_str("p1"),
            intent_id: IntentId::from_str("intent/root"),
            baseline: crate::baseline::EngineeringBaseline {
                product_goal: continuum_canonical::Opaque::from_str(""),
                target_users: Vec::new(),
                required_capabilities: Vec::new(),
                hard_constraints: Vec::new(),
                architecture: continuum_canonical::Opaque::from_str(""),
                technology_choices: Vec::new(),
                minimum_complete_system: Vec::new(),
                known_unknowns: Vec::new(),
                engineering_risks: Vec::new(),
                acceptance_criteria: Vec::new(),
            },
            readiness: ProductReadiness::unassessed(),
            current_milestone: None,
            milestone_history: Vec::new(),
            shippable_predicate,
        }
    }
}
