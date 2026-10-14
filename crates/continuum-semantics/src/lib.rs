//! P4 语义层（设计 `docs/superpowers/specs/2026-10-06-p4-semantic-layer-design.md`）。
//!
//! 本 crate 是 P4 的**语义层**成员，落《工程》§2.1 组件表里 Intent 存储与状态机
//! 那一格（§222 / §223）以及它之后的 Specification / Contract / Plan / Decision /
//! Constraint Validator（后三者由后续 task 追加进本 crate）。
//!
//! # 本层不依赖任何下层（设计 §1.2／§1.2.3）
//!
//! 本 crate **不登记任何指向执行层／资源层／边界层的 `Cargo.toml` 边**。凡跨层的输入，
//! 一律以「值」由装配方传入。今天有两条 workspace 内的边：
//!
//! - `continuum-semantics ← continuum-canonical`：**§1.3 读法甲的类型边**——
//!   `Intent.goal` 存规范化后的表示，取 [`continuum_canonical::EntityId`] 这一个类型；
//!   不透明占位 [`continuum_canonical::Opaque`] 也经这条边取。
//! - `continuum-semantics ← continuum-persist`：`intent` 表经 `Tx` 读写。
//!
//! **允许集里没有 `continuum-budget`**——§11.5 的第 1 条守卫（Constraint Validator 与
//! Budget Validator 两道门分离）。`continuum-budget` 那一侧的第 3 条守卫（它的允许集里
//! **有**本 crate，且没有任何以 `ConstraintVerdict` 为输入的函数）归 Task 17／19。
//!
//! # 走到哪为止
//!
//! Task 8 建骨架：crate 文档、workspace 登记（`Cargo.toml` 的 members 与
//! `crates/continuum-runtime/tests/dependency_direction.rs` 的 `ALLOWED`），并落
//! §222 的整套 Intent 数据模型（[`Intent`] 及它的字段类型，`intent` 模块）、本层的
//! 标识类型（[`IntentId`] / [`PlanId`] / [`ContractId`]，`ids` 模块）、`intent` 表
//! （迁移 **110**）与三条 I/O（[`save_intent`] / [`load_intent`] / [`list_children`]，
//! `persist` 模块）以及错误类型 [`IntentError`]（`error` 模块）。
//!
//! 以 `tests/intent.rs` 的用例钉住：§222 的字段清单恰是那些、子列表不落列、
//! §5.2 的版本语义两侧（语义变化 +1、状态迁移不变）、五个语义字段**逐项** +1、
//! 历史不堆在 `intent` 表里、`ContractRef` 带版本、`graph_id` / `product_id` 是不透明串、
//! `intent` 表列清单逐列、枚举列的编码纪律与表外取值处置。
//!
//! Task 9 落 §223 的状态机与「状态迁移与事件同事务」（§5.4）：
//!
//! - **迁移判定**在 [`IntentStatus::transition`]（`intent` 模块，`pub(crate)`）：
//!   §5.3 的迁移表（**本设计的决定，不是规范转录**）、两条 `Ok` 自环
//!   （`RUNNING` / `WAITING`，与其余自环一律 `Illegal` 两条独立判定）以及
//!   `COMPLETED → EVOLVING` 的 `kind = PRODUCT` 条件；
//! - **落库版** [`apply_intent_transition`]（`intent_tx` 模块）：读当前态 → 判定 →
//!   写回 `status` / `updated_at` → 写事件，全在调用方的事务里，**不自己 `commit`**；
//! - **§5.4 的两个写入点**：[`save_intent`] 首次落库写 `EventType::IntentCreated`；
//!   [`apply_intent_transition`] 迁移到 `COMPLETED` 写 `EventType::IntentCompleted`，
//!   写完复扫事件日志做收口自检（可跳过记录之后不得存在 `intent.completed`，
//!   那是既有消费者的 MUST，见 `continuum-persist/src/tx.rs`）；
//! - 判定错误 [`IntentStatusError`] 与「没有这个 Intent」的 [`IntentError::UnknownIntent`]
//!   一并落在 `error` 模块。
//!
//! 以 `tests/intent_state.rs` 的用例钉住：§5.3 的迁移表**逐条**（31 条，不抽代表）、
//! 未列出的对得 `Illegal { from, to }` 且带出那一对、自环两侧、`COMPLETED → EVOLVING`
//! 的 kind 两侧、首次落库恰一条 `intent.created`、收口恰一条 `intent.completed`、
//! 失败路径回滚后不留半写的行、可跳过事件之后的 `intent.completed` 被拒（及其反例）。
//!
//! Task 10 落 §227 的六元组与 §214 的 Specification 编译（设计 §6.1／§6.2）：
//!
//! - **§227 两句 MUST 的类型落点**：[`Unknowns`] 恰两枚（`Enumerated` / `NoneFound`）、
//!   [`SpecDraft`] 里没有 `unknowns` 字段、[`SpecDraft::finalize`] 要求**显式给出**
//!   `Unknowns`（不是 `Option`）、[`Specification`] 字段私有且唯一构造入口是 `finalize`
//!   （`spec` 模块）；
//! - **§214 只建类型、不建检测器**：[`Conflict`] / [`UndefinedBehavior`] 作为
//!   [`Unknowns::Enumerated`] 的成员**保存**（「什么算冲突」规范没给判据，§16 第 10 条）；
//! - **`specification` 表**（迁移 110 的第二张表，§6.2）：主键 `(intent_id, version)`，
//!   Specification 的版本与 Intent 的版本**同号**；`unknowns_kind` ＋ `unknowns` 两列
//!   让「已查过、没有」与「枚举为空」分得开；
//! - **同事务写** [`save_intent_with_specification`]（一个函数写两件事，同 P1 的
//!   `apply_transition` 形状，**不自己 `commit`**）与读回 [`load_specification`]。
//!
//! 以 `tests/spec.rs` 的六条用例钉住：`Unknowns` 恰两枚（**编译期**）、
//! `NoneFound` 与 `Enumerated(vec![])` 的往返**可区分**、六元组字段**逐项**、
//! 同事务写的正常／失败两侧、两枚「只保存」的成员、`unknowns` 编码的字面量与
//! 表外取值处置；以 `tests/type_level.rs` 驱动的三份 `tests/compile_fail/` 样例钉住
//! 「crate 外拼不出 `Specification`」「少给 `unknowns` 编不过」「两个检测器名字取不到」。
//!
//! Task 11 落 §224 的 Contract 与 §226 的版本化（设计 §7.1／§7.2／§7.3）：
//!
//! - **Requirement 的五个字段逐字照 §224**（[`Requirement`]：`id` / `class` /
//!   `expression` / `source` / `verification_requirement`）；[`RequirementClass`]
//!   四值封闭（`REQUIRED` / `PREFERRED` / `FLEXIBLE` / `UNSPECIFIED`，编码 / 解码
//!   与类型同址，表外取值读回具体 `Err`）；
//! - **§7.2 的禁读**：[`VerificationRequirement`] 公开方法集只有 `as_str` /
//!   `from_str`——OPEN-001 未决期间只存不判。照片：`tests/type_level.rs` 驱动的
//!   三份 trybuild 反例（`is_satisfied` / `check` / `evaluate` 各编不过）＋ 一份
//!   正控制（`as_str` 编得过）＋ `tests/contract.rs` 的「`src/` 出现点逐条登记」；
//! - **§226 的版本化**：`contract` 表主键 `(id, version)`、[`save_contract`] 只
//!   INSERT（同号二次写撞键得 `Err`、原行逐列不变）；每个新版本在同一事务内写一条
//!   `AuditKind::ContractChanges`（§5.4 的写入点之一，`AuditKind` 经
//!   `continuum-persist` 转出）；
//! - **`TaskContract` 的其余字段一律不透明值**（六个名单；`budget` 与 §12 的预算
//!   不是同一个东西），两个设计未处置的规范列表字段（`preferences[]` /
//!   `prohibitions[]`）未建——申报写在 `src/contract.rs` 的 `TaskContract` 文档里。
//!
//! 以 `tests/contract.rs` 的用例钉住：四 class 逐枚、禁读的三反例一正控、
//! `src/` 出现点清单、两张表的列清单逐列、同 `(id, version)` 二次写撞键且原行不变、
//! `ContractChanges` 恰一条且与 INSERT 同事务、六个 Opaque 字段逐项往返、
//! `RequirementId` 的不改写照片。
//!
//! Task 12 落 §226 的 ContractDiff（设计 §8.1／§8.2）：
//!
//! - **四类差异的判定**：[`DiffClass`] **恰好四枚**（`Added` / `Removed` /
//!   `Strengthened` / `Weakened`）——id 集判增删；同 id 比 `class` 的序
//!   （`UNSPECIFIED < FLEXIBLE < PREFERRED < REQUIRED`——**本设计推出来的序、
//!   不是原文给的**，§8.1 的订正段）；`class` 相同而 `expression` 变了**不下判**
//!   （§224 未给表达式语法，「Requirement 表达式的语义比较」这一步本层没有，
//!   §16 第 12 条）；
//! - **未判定走并列字段** [`UnjudgedChange`]（`ContractDiff.unjudged`）：它是
//!   「记录」，不是差异类，**不进 `entries`、不进 `DiffClass`**（§8.1 的订正段）；
//! - **纯函数**：[`diff`] 收两个 [`ContractRef`] ＋ 两侧 [`Requirement`] 集合——
//!   不收 `&Tx`、不读写库、不调 `propagate_invalidation`、不引用 `continuum-graph`
//!   （§8.2；今天这条边没有消费者，§16 第 13 条）。
//!
//! 以 `tests/diff.rs` 的七条用例钉住：四枚各一条（枚名经穷尽 `match` 读出——
//! 加第五枚即编译不过）、id 集判增删、序升/降、**三对逐对**的序、同 class 异
//! expression 进 `unjudged` 且不在 `entries`、字段清单逐项（穷尽解构、不带 `..`）、
//! 签名只有两个 ref ＋ 两侧集合（`fn` 指针类型标注）。
//!
//! Task 13 落 §228 的 Plan、§229／§338 的审查状态机与 §341 的八项视图
//! （设计 §9.1／§9.2／§9.3／§9.5）：
//!
//! - **§228 的十字段**（[`Plan`]）：`version` 取**两段式**（[`PlanVersion`]——
//!   本设计的决定，§228 只给 `version` 一个字段名；`major` 在 MATERIAL 时 +1、
//!   `minor` 在 MINOR 时 +1）；`type` 是封闭两值 [`PlanType`]；`r#type` 用 raw
//!   identifier 照录 §228 的字段名；
//! - **§229 的六态与 §9.2 的迁移表**（[`ReviewState::transition`]：九对逐条、
//!   终态无出边、六态自环全非法）与**执行门**（[`ReviewState::reaches_ready`]：
//!   `STRATEGIC` 未经 `USER_APPROVED` 得 [`PlanError::UserApprovalRequired`]、
//!   `EXECUTION` 同路径 `Ok`——两侧都钉）；
//! - **「修订」不是第六个态**（§9.2）：[`revise`] 产生新版本（`DRAFT`、major +1）
//!   并让旧版本进 `SUPERSEDED`（一条用例断言两件事都成立）；
//! - **§339／§340**（[`ReviewFinding`] / [`Severity`] / [`ReviewIndependence`] /
//!   [`ReviewFindingRecord`]）：四 severity 逐枚照片；三条独立性事实**必填**、
//!   「同一模型 + 同一上下文」的审查**可观察**（三条是 SHOULD，判定没有照片——
//!   §16 第 14 条）；`ModelRefRef` 是不透明 `String`（§1.2.3 规则 3）；
//! - **§341 的八项视图**（[`UserReviewView`]，`review_view` 模块）：字段恰好八项、
//!   不合并任何两行（**三侧**负向守卫；本组件不是 UI）；
//! - **`plan` 表**（迁移 110 的第五张表，§9.1）：主键 `(id, major, minor)`；
//!   **本 task 只建表**——行级 I/O 随第一个有 I/O 用例的 task 落地。
//!
//! 以 `tests/plan.rs` 的十四条用例钉住：十字段逐项、两段式版本两侧、迁移表九对
//! 逐条与未列出的对（含六态自环全数）、执行门两侧、修订的两件事、四 severity 逐枚、
//! 独立性字段逐项、同模型同上下文可观察、八项视图的三侧负向守卫、八项无内部步骤
//! 字段（源码面对钉）、`plan` 表列清单逐列、三对枚举的编码逐枚。
//!
//! # 尚未做的（后续 task 的，不是本 crate 的终态）
//!
//! - **§5.4 的其余三个写入点**：`PlanReviewRequired` / `DecisionRequired`
//!   （`EventType`）与 `UserApprovals`（`AuditKind`）由用它们的 task 落
//!   （`DecisionRequired` 与 `UserApprovals` 归 Task 15；**`PlanReviewRequired`
//!   尚无归属 task**——Task 13 的用例清单不含事件条目，本 task 不为它发明写入点）。
//!   本 crate 今日的写入点：`events` 两条（`IntentCreated` / `IntentCompleted`，
//!   Task 8 / 9）＋ `audit_log` 一条（`ContractChanges`，Task 11，
//!   见 [`save_contract`]）。
//! - **不建 ADFIR 图、不引用 `continuum-graph`**：`graph_id` 取不透明串
//!   （[`GraphIdRef`]，设计 §1.2.3 的规则 3），故本 crate 对执行层的依赖边为零
//!   （照片：`tests/intent.rs` 的 `graph_id_and_product_id_are_opaque_strings`——
//!   它扫本 crate 的 `src/`，断言执行层 crate 的**下划线标识符拼法**零命中）。
//! - 迁移 110 的其余两张表（`decision` / `decision_result`）由 Task 15
//!   追加进**同一个迁移项的 SQL**（不是第二个 110 号项——那会被 `migrate()` 静默跳过，
//!   见 [`p4_semantics_migrations`] 的文档）。

mod contract;
mod diff;
mod error;
mod ids;
mod intent;
mod intent_tx;
mod persist;
mod plan;
mod review_view;
mod spec;

pub use contract::{Requirement, RequirementClass, TaskContract, VerificationRequirement};
pub use diff::{ContractDiff, DiffClass, DiffEntry, UnjudgedChange, UnjudgedReason, diff};
pub use error::{ContractError, IntentError, IntentStatusError, PlanError, SpecError};
pub use ids::{ContractId, IntentId, PlanId, RequirementId};
pub use intent::{
    ContractRef, GraphIdRef, Intent, IntentKind, IntentStatus, Priority, ProductIdRef,
};
pub use intent_tx::apply_intent_transition;
pub use persist::{
    list_children, load_contract, load_intent, load_specification, p4_semantics_migrations,
    save_contract, save_intent, save_intent_with_specification,
};
pub use plan::{
    ModelRefRef, Plan, PlanType, PlanVersion, ReviewFinding, ReviewFindingRecord, ReviewState,
    ReviewIndependence, Severity, revise,
};
pub use review_view::UserReviewView;
pub use spec::{
    AcceptancePredicate, Conflict, ForbiddenState, Goal, Invariant, Preference, SpecDraft,
    Specification, UndefinedBehavior, Unknown, Unknowns,
};
