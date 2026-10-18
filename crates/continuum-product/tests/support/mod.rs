#![allow(dead_code)]

//! P7A 测试夹具：锚 Intent、假 baseline 生成器、可用基线与谓词。

use continuum_canonical::{EntityId, Opaque};
use std::collections::VecDeque;
use std::sync::Mutex;

use continuum_product::{
    BaselineRequest, BatchBoundary, EngineeringBaseline, EngineeringBaselineGenerator,
    MilestoneBatchDraft, MilestoneGenerator, MilestoneProposalInput, PortError,
    ProgressDelta, ProgressPolicy, ProductReadiness, ReadinessDimensionName, ShippableClause,
    ShippableClauseId, ShippablePredicate,
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

/// 逐次返回预置批次的假生成器；同时记录每批看到的 `history.len()` 与边界。
pub struct FixedMilestoneGenerator {
    batches: Mutex<VecDeque<Result<MilestoneBatchDraft, PortError>>>,
    seen_history_lens: Mutex<Vec<usize>>,
    seen_boundaries: Mutex<Vec<BatchBoundary>>,
}

/// 读真实 `Documentation` 三元组的策略；不返回常量。
pub struct DeltaByDimension {
    pub threshold: u32,
}

fn documentation_score(readiness: &ProductReadiness) -> Option<i64> {
    readiness
        .dimension(ReadinessDimensionName::Documentation)
        .value()
        .as_assessed()?
        .as_str()
        .parse()
        .ok()
}

impl ProgressPolicy for DeltaByDimension {
    fn delta(
        &self,
        previous: &ProductReadiness,
        current: &ProductReadiness,
    ) -> ProgressDelta {
        match (
            documentation_score(previous),
            documentation_score(current),
        ) {
            (Some(a), Some(b)) if b > a => ProgressDelta::Improved,
            (Some(a), Some(b)) if b == a => ProgressDelta::NoProgress,
            (Some(_), Some(_)) => ProgressDelta::Regressed,
            _ => ProgressDelta::Undetermined,
        }
    }

    fn stall_threshold(&self) -> u32 {
        self.threshold
    }
}

impl FixedMilestoneGenerator {
    pub fn new(batches: Vec<Result<MilestoneBatchDraft, PortError>>) -> Self {
        Self {
            batches: Mutex::new(batches.into()),
            seen_history_lens: Mutex::new(Vec::new()),
            seen_boundaries: Mutex::new(Vec::new()),
        }
    }

    pub fn seen_history_lens(&self) -> Vec<usize> {
        self.seen_history_lens.lock().unwrap().clone()
    }

    pub fn seen_boundaries(&self) -> Vec<BatchBoundary> {
        self.seen_boundaries.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl MilestoneGenerator for FixedMilestoneGenerator {
    async fn propose(
        &self,
        input: &MilestoneProposalInput<'_>,
    ) -> Result<MilestoneBatchDraft, PortError> {
        self.seen_history_lens
            .lock()
            .unwrap()
            .push(input.history.len());
        self.seen_boundaries.lock().unwrap().push(input.boundary);
        self.batches
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| Err(PortError::new("no preloaded milestone batch")))
    }
}
