#![allow(dead_code)]

//! P7A 测试夹具：锚 Intent、假 baseline 生成器、可用基线与谓词。

use continuum_canonical::{EntityId, Opaque};
use continuum_persist::{Db, Tx, Value};
use std::collections::VecDeque;
use std::sync::Mutex;

use continuum_product::{
    BaselineRequest, BatchBoundary, EngineeringBaseline, EngineeringBaselineGenerator,
    MilestoneBatchDraft, MilestoneDraft, MilestoneGenerator, MilestoneProposalInput, PortError,
    Product, ProgressDelta, ProgressPolicy, ReadinessDimension, ProductReadiness,
    ReadinessDimensionName, ShippableClause, ShippableClauseId, ShippablePredicate,
    establish_baseline, save_product,
};
use continuum_semantics::{
    ContractId, ContractRef, Intent, IntentId, IntentKind, IntentStatus, Priority, ProductIdRef,
    save_intent,
};
use continuum_verify::{
    Claim, Evidence, EvidenceId, EvidenceProducer, EvidenceScope, EvidenceStrength,
    EvidenceSubject, EvidenceType,
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

// ---------------------------------------------------------------------------
// 真库夹具（Task 10 起）
// ---------------------------------------------------------------------------

/// 只带 P0＋200＋语义迁移 110 的真库（不调 runtime 的装配清单）。
pub fn new_db() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("product.db");
    let mut migrations = continuum_persist::builtin_migrations();
    migrations.extend(continuum_product::p7a_product_migrations());
    migrations.extend(continuum_semantics::p4_semantics_migrations());
    let db = Db::open_with(&path, migrations).unwrap();
    db.migrate().unwrap();
    (dir, db)
}

pub fn request() -> BaselineRequest {
    BaselineRequest {
        idea: Opaque::from_str("an idea"),
        constraints: vec![Opaque::from_str("offline")],
    }
}

/// 由内存中的 READY 锚生成一枚尚未落库的 Product。
pub async fn product_with_intent(product_id: &str) -> (Intent, Product) {
    let intent = anchor_intent(product_id, IntentStatus::Ready);
    let product = establish_baseline(
        &intent,
        &request(),
        predicate(),
        &FixedBaselineGenerator {
            result: Ok(usable_baseline()),
        },
    )
    .await
    .unwrap();
    (intent, product)
}

/// 同一事务里写锚 Intent 与 Product（不提交）。
pub fn save_anchor_and_product(tx: &Tx<'_>, intent: &Intent, product: &Product, at: i64) {
    save_intent(tx, intent).unwrap();
    save_product(tx, product, at).unwrap();
}

/// 建库并把 p 号的锚与 Product 落库、提交；返回 Product 快照。
pub async fn saved_product(db: &Db, product_id: &str) -> Product {
    saved_product_with_intent(db, product_id, "intent/root").await
}

/// 同 [`saved_product`]，但锚 Intent 用给定 id。
///
/// `product.intent_id` 有 UNIQUE 约束，同一库里第二枚 Product 必须换锚 id。
pub async fn saved_product_with_intent(db: &Db, product_id: &str, intent_id: &str) -> Product {
    let mut intent = anchor_intent(product_id, IntentStatus::Ready);
    intent.id = IntentId::from_str(intent_id);
    let product = establish_baseline(
        &intent,
        &request(),
        predicate(),
        &FixedBaselineGenerator {
            result: Ok(usable_baseline()),
        },
    )
    .await
    .unwrap();
    let tx = db.begin().unwrap();
    save_anchor_and_product(&tx, &intent, &product, 100);
    tx.commit().unwrap();
    product
}

/// 一封无依赖、无条件的计划草案。
pub fn draft(objective: &str) -> MilestoneDraft {
    MilestoneDraft {
        objective: Opaque::from_str(objective),
        dependencies: vec![],
        entry_conditions: vec![],
        exit_conditions: vec![],
    }
}

pub fn batch(objectives: &[&str]) -> MilestoneBatchDraft {
    MilestoneBatchDraft {
        milestones: objectives.iter().map(|o| draft(o)).collect(),
    }
}

/// 一批固定 objective 的生成器（每个测试只取一批）。
pub fn milestone_generator(objectives: &[&str]) -> FixedMilestoneGenerator {
    FixedMilestoneGenerator::new(vec![Ok(batch(objectives))])
}

pub fn boundary(max_milestones: usize, max_dependency_depth: u32) -> BatchBoundary {
    BatchBoundary::new(max_milestones, max_dependency_depth).unwrap()
}

/// 一封 `e{id}` 的 Test 证据（不依赖制品）。
pub fn evidence(id: &str) -> Evidence {
    Evidence::from_tool_result(
        EvidenceId::from_str(id),
        EvidenceType::Test,
        EvidenceSubject::Unattached,
        Claim::from_str("tests pass"),
        EvidenceProducer::Human,
        vec![],
        EvidenceStrength::from_value(serde_json::json!(1)),
        EvidenceScope::from_value(serde_json::json!({})),
    )
    .unwrap()
}

/// Documentation 一维的 readiness（evidence 固定 `e1`）。
pub fn readiness_with_documentation(value: &str) -> ProductReadiness {
    ProductReadiness::unassessed().with_dimension(
        ReadinessDimensionName::Documentation,
        ReadinessDimension::assessed(
            Opaque::from_str(value),
            vec![EvidenceId::from_str("e1")],
            100,
        )
        .unwrap(),
    )
}

/// 把 Product 的 readiness 直接改成「Documentation=Assessed(value)，其余未评」。
///
/// 只写内部 JSON 作为测试起点，不写 `product_progress`；所有 delta 仍由策略算。
pub fn seed_readiness(tx: &Tx<'_>, product_id: &str, value: &str) {
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
        let dimension = if name == "documentation" {
            serde_json::json!({"value": value, "evidence": ["e1"], "last_verified": 100})
        } else {
            serde_json::json!({"value": null, "evidence": [], "last_verified": null})
        };
        object.insert(name.to_owned(), dimension);
    }
    tx.execute(
        "UPDATE product SET readiness = ?2, updated_at = 101 WHERE id = ?1",
        &[
            Value::text(product_id),
            Value::text(serde_json::Value::Object(object).to_string()),
        ],
    )
    .unwrap();
}
