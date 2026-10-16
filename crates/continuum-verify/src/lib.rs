//! P5a 验证与证据判定（设计 `docs/superpowers/specs/2026-10-08-p5a-verification-and-evidence-design.md`）。
//!
//! 本 crate 是**第 8 层（跨领域）**成员，落《工程》§8.1 的前九行（`Evidence 数据模型` /
//! `Requirement Coverage Graph` / `VerificationProfile` / `Verifier 优先级判定` /
//! `Independent Verifier` / `Verifier Adversary` / `Mutation Verification` /
//! `Completion Predicate 判定` / `Test Validity Verifier`）。Task 1 建骨架与
//! §258 的 [`EvidenceType`]（十四臂，设计 §4.2）；Task 2 落 §4 的 [`Evidence`] 数据模型
//! 与它唯一的产生点 [`Evidence::from_tool_result`]（§89）；Task 3 落 §191 的
//! [`TestValidity`]（五检查、三档聚合与五臂齐，设计 §6.4）；Task 4 落 §5 的节点级
//! [`VerificationPolicy`]（§5.1 的严格解析）、任务级 [`VerificationProfile`] 与它的
//! 取得点 [`profile_for`]（§5.2/§5.3）、§262 的五级序 [`VerifierLevel`]；Task 5 落
//! §6.1–§6.3 的 Requirement Coverage（[`ContractView`] 一侧的四枚类型、[`CoverageVerdict`]
//! 的两臂、[`counts`] 的五条计数规则与 [`coverage`] 的逐项判定）；Task 6 落 §7 的
//! Verifier 选择与隔离（[`select_verifier`] / [`independence_of`]、§7.5 的
//! [`VerifierVerdict`] / [`VerdictAggregate`] / [`BlindVerdict`] / [`clamp_by_readability`]）
//! 与两个输入面（[`InitialVerifierInput`] / [`PostBlindVerifierInput`]）；Task 7 落 §8 的
//! Completion Predicate 判定（§8.1 的十项 [`Conjunct`]、§8.2 的两臂 [`CompletionVerdict`]、
//! §8.3 的逐项判据与 §8.4 的 [`CompletionInput`] / [`judge_completion`]，含 `undecidable`
//! 列表的项类型 [`UndecidableItem`]）；Task 8 落 §9 的 Verifier Adversary（§9.1 的两臂
//! [`AdversaryOutcome`] 与消费规则 [`adversary_permits_passing`]；§9.2 的 mutation 插槽本轮
//! 不判、无类型落点）；其余类型与判定由后续 task 逐一追加。
//!
//! # 依赖边（设计 §2.1）
//!
//! 本 crate 只依赖**执行层与其下**：`continuum-artifact` / `continuum-graph` /
//! `continuum-operator` 自 Task 2 起登记（`ArtifactId` 出现在 `Evidence.artifact_refs`，
//! `NodeId` 与 `BackendId` 出现在 `EvidenceProducer::Node`），`continuum-persist` /
//! `continuum-events` 由 Task 9 登记（迁移 130 与 `VerificationFailed`）。故
//! `crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED` 里本 crate
//! 的条目自 Task 2 起是**这三条**。凡跨层的输入一律以「值」由装配方传入
//! （P4 设计 §1.2.3 的规则）：本块需要的 Contract / Requirement 身份由本块以值承载
//! （[`RequirementId`] 是本块定义的一枚，与 P4 的同名类型是两个类型，见设计 §4.3.1
//! 的订正段），**不登记指向 `continuum-semantics` 的边**（设计 §6.1）。
//!
//! # 本 task（Task 4）明确不做
//!
//! - **不建 `VerificationProfile` 的生成器**（设计 §5.3 第 1 条）：生成者未定
//!   （A6/OPEN-009），本块只冻结取得点 [`profile_for`]；[`VerificationProfileSource`]
//!   的形态本轮不决定（来自 Contract 的哪一字段取决于 OPEN-009 怎么解），只取
//!   「有档案 / 没有档案」两臂。
//! - **`optional_evidence_types` / `fuzz_budget` / `mutation_threshold` /
//!   `property_requirements` 只存不判**（§5.2）：规范未给它们的用途或量纲，本块不派活。
//! - **`crates/continuum-graph/src/node.rs` 只改 `:19` 一行注记**：`verification_policy`
//!   字段类型仍是 `serde_json::Value`（设计 §2.3 的裁定：给字段定型会造出执行层与
//!   跨领域层之间的 2-环）。

mod adversary;
mod completion;
mod conjunct;
mod contract_view;
mod coverage;
mod error;
mod evidence;
mod evidence_type;
mod input;
mod policy;
mod profile;
mod selection;
mod test_validity;
mod verdict;
mod verifier_level;

pub use adversary::{AdversaryOutcome, adversary_permits_passing};
pub use completion::{CompletionInput, CompletionVerdict, judge_completion};
pub use conjunct::{Conjunct, UndecidableItem};
pub use contract_view::{ContractRequirement, ContractView, RequirementClass, RequirementId};
pub use coverage::{CoverageVerdict, IncompleteReason, RequirementCoverage, counts, coverage};
pub use error::VerifyError;
pub use evidence::{
    Claim, Evidence, EvidenceId, EvidenceProducer, EvidenceScope, EvidenceStrength,
    EvidenceSubject, WorkerCompletionDeclaration,
};
pub use evidence_type::EvidenceType;
pub use input::{InitialVerifierInput, PostBlindVerifierInput};
pub use policy::VerificationPolicy;
pub use profile::{VerificationProfile, VerificationProfileSource, profile_for};
pub use selection::{
    AvailableVerifier, IndependenceVerdict, VerifierSelection, WorkerIdentity, independence_of,
    select_verifier,
};
pub use test_validity::{CheckVerdict, TestValidity, TestValidityCheck, Validity};
pub use verdict::{BlindVerdict, VerdictAggregate, VerifierVerdict, clamp_by_readability};
pub use verifier_level::VerifierLevel;
