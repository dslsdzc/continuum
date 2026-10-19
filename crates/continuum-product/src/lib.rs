//! P7A 产品循环：产品对象、九维 readiness、Milestone 惰性生成、Progress Gate 与
//! Shippable 判定。
//!
//! 本 crate 是长期循环层（第 7 层）的一个库 crate。C2=(b) 已批准：迁移 200 一次
//! 成型为四张表，`ContinuousProductDrive` 落在独立的 `product_drive` 表。

mod baseline;
mod drive;
mod error;
mod handoff;
mod lower;
mod milestone;
mod persist;
mod predicate;
mod product;
mod progress;
mod readiness;
mod shippable;

pub use baseline::{BaselineRequest, EngineeringBaseline, EngineeringBaselineGenerator};
pub use drive::{
    ContinuousProductDrive, ProductDriveState, ProductPhase, advance_product_phase,
};
pub use error::{PortError, ProductError};
pub use handoff::{
    product_intent_for_handoff, retire_completed_product_intent, save_lowered_intent,
};
pub use lower::lower_milestone;
pub use milestone::{
    BatchBoundary, Milestone, MilestoneBatchDraft, MilestoneDependency, MilestoneDraft,
    MilestoneGenerator, MilestoneId, MilestoneProposalInput, MilestoneState, propose_next_batch,
};
pub use persist::{
    emit_replan_decision, load_milestone, load_milestones, load_product, load_product_drive,
    load_progress, p7a_product_migrations, record_milestone_progress, record_milestone_state,
    save_milestones, save_product, set_current_milestone,
};
pub use predicate::{ShippableClause, ShippableClauseId, ShippablePredicate};
pub use product::{Product, establish_baseline};
pub use progress::{
    ProgressDelta, ProgressGateVerdict, ProgressPolicy, ProgressRecord,
    ProgressUndetermined, ReplanDraft, evaluate_progress,
};
pub use readiness::{
    ProductReadiness, ReadinessDimension, ReadinessDimensionName, ReadinessValue,
};
pub use shippable::{
    ClauseAssessment, ClauseVerdict, NotShippableReason, ShippableAssessment,
    ShippableUndetermined, ShippableVerdict, evaluate_shippable,
};
