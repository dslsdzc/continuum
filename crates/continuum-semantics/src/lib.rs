//! P4 语义层（设计 `docs/superpowers/specs/2026-10-06-p4-semantic-layer-design.md`）。
//!
//! 本 crate 是 P4 的**语义层**成员，落《工程》§2.1 组件表里 Intent 存储与状态机
//! 那一格（§222 / §223）以及它之后的 Specification / Contract / Plan / Decision /
//! Constraint Validator——后五者由 Task 10／11／13／15／16 逐一追加，**Task 15 之后
//! 全部落地**。
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
//! Task 16 落 §225 的 Contract 不可越权原则与 §6 的 Constraint Violation Gate
//! （设计 §11.2／§11.3／§11.4）：
//!
//! - **两条拦下路径**：[`ConstraintVerdict::ViolatesRequired`]（`constraints` 里的键
//!   命中一条 `class = REQUIRED` 的 Requirement——§11.3 判据 3 的订正段：「命中之后
//!   按 REQUIRED 那一路去判」）与 [`ConstraintVerdict::ViolatesProhibition`]
//!   （§225 的 MUST NOT 后三条，`what` 指出是哪一枚）；
//! - **「判不了」走 `Err`**：[`ConstraintError::UnknownConstraintKey`]——触发条件是
//!   「`key` 不在当前 Contract 的 Requirement id 集合里」，**与 `class` 无关**；
//!   三侧照片（悬空键 `Err`／空表不是 `Err`／命中非 REQUIRED 键不是 `Err`）**合起来
//!   才算钉住**；
//! - **输入面是值**：[`CandidatePlan`]（§228 的 `Plan` ＋ [`PlanFootprint`]——§225
//!   后三条的对偶声明的载体，Task 14 的 `classify` 复用它）＋ 图侧以**原样类型**
//!   进入的 `&[String]` / `&serde_json::Value`（不经过本层的类型转换）；
//! - **`execution_policy` 的读法**（**本设计的读法**，申报在 `constraint.rs` 的模块
//!   文档里）：一组变更名的 JSON 数组，字面取 §225 清单的原文词；命中禁令三条 →
//!   拦、其余 → 放行。读法的边界（未知条目不拦、逐字比对、非数组视为无声明）与代价
//!   与读法同址；
//! - **§11.4**：[`validate`] **自己不挂起、不写 Intent 状态**——只返回裁决
//!   （不收 `&Tx`、不写任何表）。
//!
//! 以 `tests/constraint.rs` 的十条用例钉住：输入面两条（`fn` 指针标注 ＋ 穷尽解构）、
//! `execution_policy` 的 7（MAY 逐项放行）＋ 3（MUST NOT 后三条逐项拦下）＝ 10 条、
//! 原样类型一条（含 `src/` 的执行层 crate 拼法零命中）、裁决五条（REQUIRED 一条 ＋
//! 三 class 的「违反但不拦下」）、「判不了」三侧。
//!
//! Task 14 落 §230 的 Plan Change 分类（设计 §9.4）：
//!
//! - **两个 class 封闭**（[`PlanChangeClass`]：`Minor` / `Material`），MATERIAL 的
//!   MUST（重触发审查门）落在 [`apply_plan_change`]——MATERIAL 产出新版本
//!   （`major` +1）且 `review_state = DRAFT`；MINOR 不重触发（`minor` +1、审查态保持，
//!   **两侧都钉**）；
//! - **§230 的十个例逐项**（五个 MINOR ＋ 五个 MATERIAL）：**判定映射是本实现的
//!   读法**（收件人：复审者）——MINOR 五例是执行 HOW 的变更（§225 的 MAY 清单与
//!   它们重叠），在两 Plan 两 footprint 上不可见；MATERIAL 三条「内容变化」例
//!   （重大架构变化 / Requirement 改变 / Milestone 目标变化）同判「计划内容
//!   （§228 的五个列表）有任一变化」；
//! - **[`PlanChange`] 的 `affected_scope` 是 [`continuum_canonical::Opaque`]**
//!   （§230 未给取值域——§16 第 15 条，本层不发明）；
//! - **`classify` 是五参**（R-B 裁决）：两份 [`Plan`] ＋ 两份 [`PlanFootprint`]
//!   （§9.4 的四参）＋ `&MaterialThresholds`（第五参）——「『显著扩大』的阈值规范
//!   未给」（§16 第 15 条），比较由调用方的两个**判定引用**承载，本层连比较都不发明
//!   （处置链：设计 §9.4 → §16 第 15 条 → 计划遗留第四节 → R-B；与计划 Step 速写的
//!   四参偏离在此申报）；
//! - **纯函数**：[`classify`] / [`apply_plan_change`] 都不收 `&Tx`、不读写库
//!   （`plan_change` 没有表——设计 §2.4 的 110 段七张表里没有它）。
//!
//! 以 `tests/plan_change.rs` 的七条用例钉住：两个 class 各一条（判别式经穷尽 `match`）、
//! §230 的十例**逐项一条**（表驱动，不抽代表）、两条**直接调用**（两个 class 的
//! 第一例：模型替换 / 重大架构变化——表驱动限度的对照）、重触发审查门的两侧、
//! `affected_scope` 的表外取值往返、输入面签名（`fn` 指针标注 ＋ 两个阈值判据真的
//! 被调用）。
//!
//! Task 15 落 §331 的 Decision 对象与 §332 的恢复链（设计 §10.1／§10.2）：
//!
//! - **§331 的五个成员**（[`Decision`]）：`issue` / `options`（一组
//!   [`DecisionOption`]＝「选项 ＋ 后果」的配对，§106 的「每个选项带后果」落在
//!   [`DecisionOption::new`] 的**构造期**——「等长」由类型保证）／`blocking`；
//!   另补 `id` 与 `intent_id` 两个字段（申报在 [`DecisionId`] 与 `decision.rs`）；
//! - **`blocking` 是被给的事实**：本层只在 §10.1 的两个触发点写 `true`（经
//!   [`create_decision`] 照收），**不发明 `false` 的触发条件**（§16 第 16 条）；
//! - **§107 的 `DecisionResult`**：指向某个 `Decision` 与某个 `DecisionOption`
//!   （`decision_id` ＋ `option_index`）＋ 对 Contract/Plan 的**指名 patch**
//!   （[`ContractUpdate`] / [`PlanUpdate`]，形状是本实现的落点）＋ `decided_at`；
//!   [`apply_result`] 的输入只有它——不是自由文本、不重跑 Intent Compiler；
//! - **§332 的恢复链四步同一事务**（[`apply_result`]，**不自己 `commit`**）：
//!   `DecisionResult` 落库（[`save_decision_result`]：行 ＋ `UserApprovals` 审计）
//!   → Contract/Plan 新版本（前者 `base + 1`；后者 `major` +1、`DRAFT`、旧版本转
//!   `SUPERSEDED`）→ Intent 从 `SUSPENDED` 回 `RUNNING`（`Graph invalidation`
//!   归执行层）；失败路径**四张表**（`decision_result` / `contract` / `plan` /
//!   `intent`）无半写行；
//! - **两张新表**（迁移 110 的第六、七张表，§2.4 的七张由此齐）：`decision`
//!   （`options` 存一组配对对象）与 `decision_result`（主键 `decision_id`——
//!   一个决策恰一个回答）；`plan` 的行级 I/O（[`save_plan`] / [`load_plan`]，Task 13
//!   预告的那一半）随本 task 落地，`PlanType` / `ReviewState` 的编码对随之下
//!   **收窄回 `pub(crate)`**（Task 13 写明的收窄条件）；
//! - **§5.4 的另外两个写入点**：[`create_decision`] 写 `DecisionRequired` 事件；
//!   [`save_decision_result`] 写 `UserApprovals` 审计（**本层是它的第二个生产方**
//!   ——第一个是 `continuum-workspace` 的 Integration Gate，同一 kind 下不保证载荷
//!   同形，§16 第 33 条）。
//!
//! 以 `tests/decision.rs` 的十余条用例钉住：等长读回（枚举三档）＋ trybuild 反例／
//! 正控制、两个触发点的 `blocking == true`、公开面恰好一个 `Decision`、
//! `DecisionRequired` 恰一条、只改被指名字段（逐项）、无「文本重编译」路径、
//! 恢复链失败路径四张表无半写、`UserApprovals` 恰一条（第二生产方）、
//! 『指向』的 fail-closed 两侧、表外取值具体 `Err`、同 id 二次创建撞主键。
//!
//! # 尚未做的（后续 task 的，不是本 crate 的终态）
//!
//! - **§5.4 的最后一个无归属写入点**：`PlanReviewRequired`（`EventType`）
//!   ——**尚无归属 task**（Task 13 的用例清单不含事件条目，本 crate 至今不为它
//!   发明写入点）。本 crate 今日的写入点：`events` 三条（`IntentCreated` /
//!   `IntentCompleted` / `DecisionRequired`，Task 8／9／15）＋ `audit_log` 两条
//!   （`ContractChanges` / `UserApprovals`，Task 11／15，见 [`save_contract`] /
//!   [`save_decision_result`]）。
//! - **不建 ADFIR 图、不引用 `continuum-graph`**：`graph_id` 取不透明串
//!   （[`GraphIdRef`]，设计 §1.2.3 的规则 3），故本 crate 对执行层的依赖边为零
//!   （照片：`tests/intent.rs` 的 `graph_id_and_product_id_are_opaque_strings`——
//!   它扫本 crate 的 `src/`，断言执行层 crate 的**下划线标识符拼法**零命中）。
//! - **迁移 110 的七张表已齐**（Task 15 的 `decision` / `decision_result` 追加进
//!   **同一个迁移项的 SQL**——不是第二个 110 号项，那会被 `migrate()` 静默跳过，
//!   见 [`p4_semantics_migrations`] 的文档）。**110 段此后不应再有新表**；
//!   §332 的 `Graph invalidation` 与 `Resume` 的执行侧（节点解禁、任务续跑）归执行层。

mod constraint;
mod contract;
mod decision;
mod diff;
mod error;
mod ids;
mod intent;
mod intent_tx;
mod persist;
mod plan;
mod plan_change;
mod review_view;
mod spec;

pub use constraint::{
    CandidatePlan, ConstraintError, ConstraintVerdict, PlanFootprint, ViolatedProhibition, validate,
};
pub use contract::{Requirement, RequirementClass, TaskContract, VerificationRequirement};
pub use decision::{
    ContractUpdate, Decision, DecisionOption, DecisionResult, PlanUpdate, RequirementUpdate,
};
pub use diff::{ContractDiff, DiffClass, DiffEntry, UnjudgedChange, UnjudgedReason, diff};
pub use error::{ContractError, DecisionError, IntentError, IntentStatusError, PlanError, SpecError};
pub use ids::{ContractId, DecisionId, IntentId, PlanId, RequirementId};
pub use intent::{
    ContractRef, GraphIdRef, Intent, IntentKind, IntentStatus, Priority, ProductIdRef,
};
pub use intent_tx::apply_intent_transition;
pub use persist::{
    apply_result, create_decision, list_children, load_contract, load_decision, load_intent,
    load_plan, load_specification, p4_semantics_migrations, save_contract, save_decision_result,
    save_intent, save_intent_with_specification, save_plan,
};
pub use plan::{
    ModelRefRef, Plan, PlanType, PlanVersion, ReviewFinding, ReviewFindingRecord, ReviewState,
    ReviewIndependence, Severity, revise,
};
pub use plan_change::{
    MaterialThresholds, PlanChange, PlanChangeClass, apply_plan_change, classify,
};
pub use review_view::UserReviewView;
pub use spec::{
    AcceptancePredicate, Conflict, ForbiddenState, Goal, Invariant, Preference, SpecDraft,
    Specification, UndefinedBehavior, Unknown, Unknowns,
};
