//! P5a 验证与证据判定（设计 `docs/superpowers/specs/2026-10-08-p5a-verification-and-evidence-design.md`）。
//!
//! 本 crate 是**第 8 层（跨领域）**成员，落《工程》§8.1 的前九行（`Evidence 数据模型` /
//! `Requirement Coverage Graph` / `VerificationProfile` / `Verifier 优先级判定` /
//! `Independent Verifier` / `Verifier Adversary` / `Mutation Verification` /
//! `Completion Predicate 判定` / `Test Validity Verifier`）。本 task 只建骨架与
//! §258 的 `EvidenceType`（十四臂，设计 §4.2）；其余类型与判定由后续 task 逐一追加。
//!
//! # 依赖边（设计 §2.1）
//!
//! 本 crate 只依赖**执行层与其下**：`continuum-artifact` / `continuum-graph` /
//! `continuum-operator`（Task 2 加）与 `continuum-persist` / `continuum-events`
//! （Task 9 加）由各自的 task 增量登记；本 task 对内部 crate 的引用为零，故
//! `crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED` 里本 crate
//! 的条目是**空数组**。凡跨层的输入一律以「值」由装配方传入（P4 设计 §1.2.3 的规则）：
//! 本块需要的 Contract / Requirement 身份由本块以值承载，**不登记指向
//! `continuum-semantics` 的边**（设计 §6.1）。
//!
//! # 本 task 明确不做
//!
//! - **不建 `error.rs`**：`EvidenceType::parse` 与 `ArtifactType::parse` 同形，返回
//!   `Option`；`VerifyError` 由第一个真的返回 `Result` 的 task（Task 2）建。
//!   「叶子 crate 的条目记实际依赖」的同一条口径也用在类型上（不为将来可能用到而先建）。
//! - **不建设计 §2.2 冻结清单的其余类型**：由用它们的 task 逐一追加，本 task 不预放占位。

mod evidence_type;

pub use evidence_type::EvidenceType;
