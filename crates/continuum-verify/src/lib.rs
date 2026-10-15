//! P5a 验证与证据判定（设计 `docs/superpowers/specs/2026-10-08-p5a-verification-and-evidence-design.md`）。
//!
//! 本 crate 是**第 8 层（跨领域）**成员，落《工程》§8.1 的前九行（`Evidence 数据模型` /
//! `Requirement Coverage Graph` / `VerificationProfile` / `Verifier 优先级判定` /
//! `Independent Verifier` / `Verifier Adversary` / `Mutation Verification` /
//! `Completion Predicate 判定` / `Test Validity Verifier`）。Task 1 建骨架与
//! §258 的 [`EvidenceType`]（十四臂，设计 §4.2）；Task 2 落 §4 的 [`Evidence`] 数据模型
//! 与它唯一的产生点 [`Evidence::from_tool_result`]（§89）；其余类型与判定由后续 task
//! 逐一追加。
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
//! # 本 task（Task 2）明确不做
//!
//! - **不检查**：`claim` 的内容、`strength` / `scope` 的值、`artifact_refs` 里制品是否
//!   存在（那要查库，属于 `artifact_exists`，Task 9）、生产者的节点是否真属该图
//!   （在 Task 6 的装配处查）——设计 §4.4 的「它不检查什么」逐条照办。
//! - **`contract_view.rs` 本 task 只放 [`RequirementId`] 一枚**：`RequirementClass` /
//!   `ContractRequirement` / `ContractView` 由 Task 5 追加到同一文件。
//! - **不建设计 §2.2 冻结清单的其余类型**：由用它们的 task 逐一追加，本 task 不预放占位。

mod contract_view;
mod error;
mod evidence;
mod evidence_type;

pub use contract_view::RequirementId;
pub use error::VerifyError;
pub use evidence::{
    Claim, Evidence, EvidenceId, EvidenceProducer, EvidenceScope, EvidenceStrength,
    EvidenceSubject, WorkerCompletionDeclaration,
};
pub use evidence_type::EvidenceType;
