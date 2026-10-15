//! P5a 验证与证据判定（设计 `docs/superpowers/specs/2026-10-08-p5a-verification-and-evidence-design.md`）。
//!
//! 本 crate 是**第 8 层（跨领域）**成员，落《工程》§8.1 的前九行（`Evidence 数据模型` /
//! `Requirement Coverage Graph` / `VerificationProfile` / `Verifier 优先级判定` /
//! `Independent Verifier` / `Verifier Adversary` / `Mutation Verification` /
//! `Completion Predicate 判定` / `Test Validity Verifier`）。Task 1 建骨架与
//! §258 的 [`EvidenceType`]（十四臂，设计 §4.2）；Task 2 落 §4 的 [`Evidence`] 数据模型
//! 与它唯一的产生点 [`Evidence::from_tool_result`]（§89）；Task 3 落 §191 的
//! [`TestValidity`]（五检查、三档聚合与五臂齐，设计 §6.4）；其余类型与判定由后续 task
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
//! # 本 task（Task 3）明确不做
//!
//! - **不建 Test Validity Verifier 的产生者**（设计 §6.5 的处置）：「谁跑这五个检查、
//!   什么时候跑、输入取自哪里（源码／制品／`Evidence<Test>`）」规范未给，在未给期间造一个
//!   等于替 §191 选了一条它没写的执行路径；本块只定义记录（[`TestValidity`]）与它的形状
//!   校验（五臂齐），产生者留缺口。
//! - **不建 `coverage` / `counts`（Task 5）与 §5 的策略类型（Task 4）**：由用它们的 task
//!   逐一追加，本 task 不预放占位。

mod contract_view;
mod error;
mod evidence;
mod evidence_type;
mod test_validity;

pub use contract_view::RequirementId;
pub use error::VerifyError;
pub use evidence::{
    Claim, Evidence, EvidenceId, EvidenceProducer, EvidenceScope, EvidenceStrength,
    EvidenceSubject, WorkerCompletionDeclaration,
};
pub use evidence_type::EvidenceType;
pub use test_validity::{CheckVerdict, TestValidity, TestValidityCheck, Validity};
