//! 设计 §2.4 的七张表
//! （`intent` / `specification` / `contract` / `contract_requirement` / `plan` /
//! `decision` / `decision_result`，迁移 **110**）与它们的行级读写
//! （设计 §5.1／§5.2／§5.4／§6.1／§6.2／§7.1／§7.3／§9.1／§10.1／§10.2）。
//! **110 段的七张表由此齐**：Task 8 加 `intent`、Task 10 加 `specification`、
//! Task 11 加 `contract` 与 `contract_requirement`、Task 13 加 `plan`、
//! **Task 15 加 `decision` 与 `decision_result`**（本 task——两张表连同 §332
//! 恢复链的行级 I/O 一起落地）。
//!
//! # 本文件与 `intent.rs` 的分工（一处与计划正文不同的落点，理由在此）
//!
//! `intent.rs` 是 **§222 的词汇与记录体**（`Intent` 及其字段类型）；
//! 本文件是**表与 I/O**（迁移 110 的 `intent` 一张表、`save_intent` / `load_intent` /
//! `list_children`）。计划「文件结构」一节把「迁移表」列在 `src/intent.rs`
//! （`docs/superpowers/plans/2026-10-06-p4-semantic-layer.md:301`），而同一个文件结构表
//! 又把「`src/persist.rs`：七张表的迁移 110（逐表追加）与行级读写」写在另一行
//! （同文件 `:312`）——**那一节自相抵**。本 task 按 `continuum-canonical` 的
//! `alias` 表处置办（迁移与四条 I/O 同址于 `src/persist.rs`，理由写在那个文件头），
//! 并依计划自己的判据「实现期出现了更一致的归属，改的是计划那一行」（同文件 `:285-287`）。
//!
//! # 迁移 110：本层的表（逐表追加）
//!
//! 设计 §2.4 规定 110 含**七张表**（`intent` / `specification` / `contract` /
//! `contract_requirement` / `plan` / `decision` / `decision_result`）；**其余表由用它们的
//! task 逐表追加**（计划「全局约束」：`110` 的 SQL 在 Task 8–15 之间逐表追加，
//! Task 20 之后不得再改）。**「追加」的落点是同一个迁移项的 SQL 字符串**——
//! 见 [`p4_semantics_migrations`] 的文档（第二个 110 号迁移项会被 `migrate()`
//! 静默跳过）。Task 8 加 `intent`；**Task 10 加 `specification`**；
//! **Task 11 加 `contract` 与 `contract_requirement`**；**Task 13 加 `plan`**；
//! **Task 15 加 `decision` 与 `decision_result`**。
//!
//! 编号 **110**：1、2 是 `continuum-persist` 的内建（events、audit_log），10 是 P1 的
//! artifact，20 是 P1 的 graph，30 是 P2 的 workspace，40 是 P2 的 effect，41 是 P2 的
//! policy，50 是 P3 的 capability，80 是 P3D 的模型表，100 是 P4 Task 7 的 alias。
//! **110 在驱动装配的那个集合里未被占用**（本 task 现场核对：`grep -rn "Migration::new("
//! crates/*/src/` 除本文件那一处外命中的 version 为 1 / 2 / 10 / 20 / 30 / 40 / 41 / 50 /
//! 80 / 100，**另加 `continuum-persist/src/bin/crash-writer.rs:16` 的 60**——那是工具 bin，
//! 不进装配链）。
//!
//! **「未被占用」是按库说的，不是全仓**（同 `continuum-canonical/src/persist.rs` 的
//! `p4_canonical_migrations` 那段）：`crates/continuum-persist/tests/migrations.rs:45`
//! 那个 100 与 `tests/crash_atomicity.rs:9` 那个 60 都活在全仓里，只是它们各自的库
//! 与本 crate 的库**永不同库**。
//!
//! # 表外取值一律 `Err`，不取默认值
//!
//! `kind` / `status` 两列是**封闭枚举**（六值 / 十三值），解码走 [`crate::IntentKind::parse`] /
//! [`crate::IntentStatus::parse`]；认不出的串转成 [`IntentError::UnknownKind`] /
//! [`IntentError::UnknownStatus`] 并**带原始取值**（计划「全局约束」的枚举列纪律）。
//!
//! # 编码本体不在本文件
//!
//! `kind` / `status` 的字面量取自 [`crate::IntentKind::as_str`] / [`crate::IntentStatus::as_str`]
//! 与 [`crate::IntentKind::parse`] / [`crate::IntentStatus::parse`]（两对都与类型同址——`as_str`
//! 无通配臂、`parse` 带通配臂，理由见各自类型）。`unknowns_kind` / `unknown` 的 kind 同理：
//! 编码取 [`crate::Unknowns::kind_str`] / [`crate::Unknown::kind_str`]，解码走
//! [`crate::Unknowns::parse_kind`] / [`crate::Unknown::parse_kind`]（`decode_unknowns` 调用后者）。
//! **白名单一个字面量都不内联在本文件**：`parse` 给 `None` 之后，转成哪一枚错误
//! （带库里那一个原始串）才是本文件的事。照片：`tests/spec.rs` 的
//! `the_unknowns_columns_use_lowercase_underscore_and_reject_out_of_table_values` 逐枚读库钉。
//!
//! 〔订正（2026-10-14，fix 轮 1）〕本段初版写「两列的白名单各写一遍是刻意的」，
//! 把「`parse` 白名单与类型同址」这条纪律**读反**了（当时白名单内联在 `decode_unknowns`
//! 里），且与上上段「解码走 `…::parse`」自相矛盾。现已把白名单移进 `spec.rs` 的
//! `parse_kind`（照 `IntentKind::parse` 形状），旧说法记此。

use continuum_canonical::Opaque;
use continuum_persist::{
    AuditKind, Event, EventType, Migration, PersistError, Tx, Value, value::kind_name,
};

use crate::contract::{Requirement, RequirementClass, TaskContract, VerificationRequirement};
use crate::decision::{ContractUpdate, Decision, DecisionOption, DecisionResult, PlanUpdate};
use crate::error::{ContractError, DecisionError, IntentError, PlanError, SpecError};
use crate::ids::{ContractId, DecisionId, IntentId, PlanId, RequirementId};
use crate::intent::{
    ContractRef, GraphIdRef, Intent, IntentKind, IntentStatus, Priority, ProductIdRef,
};
use crate::intent_tx::apply_intent_transition;
use crate::plan::{Plan, PlanType, PlanVersion, ReviewState};
use crate::spec::{
    AcceptancePredicate, ForbiddenState, Goal, Invariant, Preference, SpecDraft, Specification,
    Unknown, Unknowns, UnknownsKind,
};

/// `intent` 表的列清单，只写一份：三条读路径共用同一列序，`row_to_intent` 按它取列。
///
/// 十五列，与 §222 的十五个字段**不是一一对应**（§5.1 的表：`created_at`/`updated_at` 与
/// `parent_intent?`/`child_intents[]` 各占一行两字段）：`contract_id` 拆成
/// `contract_id` ＋ `contract_version` 两列，而 `child_intents[]` **没有列**
/// （子列表不落列，见 [`save_intent`] 的文档）。
const COLUMNS: &str = "id, version, kind, goal, status, priority, created_at, updated_at, \
                       parent_intent, contract_id, contract_version, active_plan_id, graph_id, \
                       product_id, completion_predicate";

/// 本 crate 的七张表（`intent` / `specification` / `contract` / `contract_requirement` /
/// `plan` / `decision` / `decision_result`）上注册的迁移（设计 §2.4 的 110 段）。
///
/// # 110 是**一个迁移项**，表**逐表追加进它的 SQL**
///
/// **「追加」的落点是同一个迁移项的 SQL 字符串，不是往返回向量里加第二个 version = 110
/// 的项**——`Db::migrate()` 以 `schema_migrations.version`（**主键**）判「已应用」
/// （`crates/continuum-persist/src/db.rs:103-112`），同号的第二个迁移项会被**静默跳过**
/// （第二个表永不建）。照片是 `tests/intent.rs` 的
/// `the_migration_registers_every_table_of_the_110_item`：它断言
/// **`len() == 1`**、并逐表断言 SQL 里每张表都在（Task 8–15 共七张），
/// 那条 `len() == 1` 正是这件事的守卫。
///
/// 〔订正（2026-10-14，Task 10）〕本注释初版写「其余六张……追加进**同一个**
/// `p4_semantics_migrations()` 的返回向量（各自的迁移号也是 110，表不同）」——
/// 那句话按字面做会撞上上面这条跳过规则，**是错的**，旧说法记此。〕
///
/// **迁移名 `p4_intent` 是 Task 8 的历史命名**（当时 110 只建 `intent` 一张表）：
/// 名字不改（已应用的库在 `schema_migrations` 里记着它），它现在是 110 段的名字。
///
/// # `specification` 表的形状（Task 10 追加）
///
/// - 主键 `(intent_id, version)`——**Specification 的版本与 Intent 的版本同号**
///   （§6.2）：一次语义变化产出一个新的 Intent version 与一个对应的 Specification，
///   两者同事务写（[`save_intent_with_specification`]），故「一版一行」由主键钉住，
///   同一 `(intent_id, version)` 的二次写被拒（照片：`tests/spec.rs` 的
///   `an_intent_version_and_its_specification_are_written_in_one_transaction` 的 (b)）。
/// - `goals` / `invariants` / `acceptance_predicates` / `forbidden_states` /
///   `preferences` 五列：JSON 字符串数组（容器），元素就是各类型的不透明串
///   （同 `continuum-model-registry` 的三个列表列）。
/// - `unknowns_kind` ＋ `unknowns` 两列：**§227 两句 MUST 的落库形态**——
///   `unknowns_kind` 是闭集两枚（`none_found` / `enumerated`），`unknowns` 是 JSON 数组
///   （元素 `{kind, value}`）。**两列缺一不可**：只留一个列表列的话，
///   `NoneFound` 与 `Enumerated(vec![])` 会编成同一个表示（`vec![]` 同时表示
///   「已查过、没有」与「没查」），那正是 §227 禁的「把未知项自动转换成假设」。
///
/// # `contract` 与 `contract_requirement` 两张表的形状（Task 11 追加）
///
/// - `contract` 表：主键 `(id, version)`——**§226 的 MUST NOT 原地修改**的落库形态。
///   无 UPDATE 路径（[`save_contract`] 只 INSERT），同 `(id, version)` 的二次写被
///   复合主键拒绝（照片：`tests/contract.rs` 的
///   `a_second_write_of_the_same_id_and_version_hits_the_primary_key`）。
///   九列 = `TaskContract` 的字段减 `requirements[]`（它落另一张表）。
///   **`budget` / `authority_limit` / `data_policy` / `model_policy` / `node_policy` /
///   `acceptance_predicate` 六列都存不透明串**——§224 只给名字（§7.1）；
///   读回逐字节往返的照片在同文件的 `the_other_fields_of_224_are_opaque`。
/// - `contract_requirement` 表：主键 `(contract_id, contract_version, id)`——
///   **归属键两列**把 requirement 钉在一条 contract 的具体版本上（同 `specification`
///   表的 `intent_id` ＋ `version` 处置），`id` 是 Requirement 自己的记录键。
///   Requirement 的五个字段名照 §224（`id` / `class` / `expression` / `source` /
///   `verification_requirement`），列清单**逐列**的照片在
///   `tests/contract.rs` 的 `the_contract_requirement_table_has_exactly_the_expected_columns`。
///   `class` 是闭集四枚（`required` / `preferred` / `flexible` / `unspecified`，
///   编码 / 解码挂在 [`crate::RequirementClass::as_str`] / `parse` 上，与类型同址），
///   表外取值读回 [`crate::ContractError::UnknownClass`]（带原始串、不取默认值）。
/// - **`verification_requirement` 列照常存、照常读**（只是没有判定方法）——
///   §7.2 的「只存不判」：禁读的是**方法**，不是列。
/// - **两张表都不建外键**（同 `intent.contract_id` 的处置）：跨记录对应关系由写路径的
///   调用方保证，本层不在库里登记引用约束。
///
/// # `plan` 表的形状（Task 13 追加）
///
/// - 主键 `(id, major, minor)`——§228 的 `version` 是**两段式**（§9.1：`major` 在
///   MATERIAL 时 +1、`minor` 在 MINOR 时 +1），故版本拆成两列、连同 `id` 构成复合
///   主键：多版本并存（§213 的版本化）、同 `(id, major, minor)` 的二次写由主键拒绝
///   （与 `contract` 的 `(id, version)` 同一条处置；[`save_plan`] 的裸 `INSERT`
///   由 Task 15 落地，同键二次写随它撞键得 `Err`）。
/// - 十一个列：[`crate::Plan`] 的十个字段里**九个直接成列**（`id` / `intent_id` /
///   `type` / `objectives` / `milestones` / `assumptions` / `risks` /
///   `acceptance_conditions` / `review_state`），`version` **拆成两列**（`major` ＋
///   `minor`）——9 ＋ 2 ＝ 11。
///   **列清单逐列的照片**：`tests/plan.rs` 的
///   `the_plan_table_has_exactly_the_expected_columns`。
/// - `type` / `review_state` 是两个**枚举列**（编码 / 解码挂在
///   `PlanType::as_str`／`parse` 与 `ReviewState::as_str`／`parse` 上，与类型同址），
///   表外取值读回 [`PlanError::UnknownType`] / [`PlanError::UnknownReview`]
///   （Task 15 的行级 I/O 落地时兑现——Task 13 的批注说的就是这两个变体）。
/// - 五个列表列（`objectives` / `milestones` / `assumptions` / `risks` /
///   `acceptance_conditions`）按 JSON 字符串数组存（同 `specification` 的五个列表列
///   的容器处置）。
/// - **不建外键**（同其余四张表的处置）。
///
/// # `plan` 表的行级读写（Task 15 落地）
///
/// Task 13 只建了表；**§10.2 的恢复链要写 `plan` 行**——本 task 落它的行级 I/O：
/// [`save_plan`]（裸 `INSERT` 一行版本）与 [`load_plan`]（按 `(id, major, minor)` 读回）。
/// `type` / `review_state` 两列的表外取值读回 [`PlanError::UnknownType`] /
/// [`PlanError::UnknownReview`]（枚举列纪律的读回侧；Task 13 预告的「随行级 I/O 落地」
/// 即此）；两条编码对（`PlanType` / `ReviewState` 的 `as_str` / `parse`）也随之下
/// **收窄回 `pub(crate)`**（Task 13 的加宽申报写明了收窄时点）。
/// 修订语义的另一半（旧版本转 `SUPERSEDED`）在 [`crate::apply_result`] 的 Plan 更新
/// 里落库（Task 13 预告的「落库版修订」）。
///
/// # `decision` 与 `decision_result` 两张表的形状（Task 15 追加）
///
/// - `decision` 表：主键 `id`——[`DecisionId`] 是本层补的记录键（§331 没给，见
///   [`crate::DecisionId`] 与 `decision.rs` 的申报）。五列 = [`Decision`] 的五个成员
///   （`id` / `intent_id` / `issue` / `options` / `blocking`）。
///   `options` 列是 **JSON 对象数组**（元素 `{"option": ..., "consequence": ...}`——
///   等长的配对在库里是**一组对象**，不是两个平行数组：两个平行列在库里可不等长，
///   而 §106 要的是「每个选项带后果」的配对）。`blocking` 存 INTEGER 0/1；
///   读回时非 0/1 是 [`DecisionError::InvalidBlocking`]（不静默当 `true`）。
/// - `decision_result` 表：主键 **`decision_id`**——「一个 Decision 恰一条结果」
///   （§10.2 的 `DecisionResult` 是「对那次询问的回答」；同 `(id, version)` 的失败
///   closed 处置：第二个结果撞主键得 `Err`，不静默第二条、不覆写。若复审裁「同一
///   决策的多个结果」要可表达，改主键是一处小改）。五列 = [`DecisionResult`] 的五个
///   成员；`contract_update` / `plan_update` 两列是 JSON 或 `NULL`（`NULL`＝那一侧的
///   `Option::None`＝本次决定不动它）。**本表今天只写不读**——§332 的消费方
///   （执行层）未建，读回路径随第一个真实消费者落地（不发明 `load_decision_result`）。
/// - **两张表都不建外键**（同其余五张表的处置：跨记录对应关系由写路径的调用方保证）。
pub fn p4_semantics_migrations() -> Vec<Migration> {
    vec![Migration::new(
        110,
        "p4_intent",
        "CREATE TABLE intent (
            id                   TEXT PRIMARY KEY NOT NULL,
            version              INTEGER NOT NULL,
            kind                 TEXT NOT NULL,
            goal                 TEXT NOT NULL,
            status               TEXT NOT NULL,
            priority             TEXT NOT NULL,
            created_at           INTEGER NOT NULL,
            updated_at           INTEGER NOT NULL,
            parent_intent        TEXT,
            contract_id          TEXT NOT NULL,
            contract_version     INTEGER NOT NULL,
            active_plan_id       TEXT,
            graph_id             TEXT,
            product_id           TEXT,
            completion_predicate TEXT NOT NULL
        );
        CREATE INDEX idx_intent_parent ON intent(parent_intent);
        CREATE TABLE specification (
            intent_id             TEXT NOT NULL,
            version               INTEGER NOT NULL,
            goals                 TEXT NOT NULL,
            invariants            TEXT NOT NULL,
            acceptance_predicates TEXT NOT NULL,
            forbidden_states      TEXT NOT NULL,
            preferences           TEXT NOT NULL,
            unknowns_kind         TEXT NOT NULL,
            unknowns              TEXT NOT NULL,
            PRIMARY KEY (intent_id, version)
        );
        CREATE TABLE contract (
            id                   TEXT NOT NULL,
            version              INTEGER NOT NULL,
            intent_id            TEXT NOT NULL,
            budget               TEXT NOT NULL,
            authority_limit      TEXT NOT NULL,
            data_policy          TEXT NOT NULL,
            model_policy         TEXT NOT NULL,
            node_policy          TEXT NOT NULL,
            acceptance_predicate TEXT NOT NULL,
            PRIMARY KEY (id, version)
        );
        CREATE TABLE contract_requirement (
            contract_id              TEXT NOT NULL,
            contract_version         INTEGER NOT NULL,
            id                       TEXT NOT NULL,
            class                    TEXT NOT NULL,
            expression               TEXT NOT NULL,
            source                   TEXT NOT NULL,
            verification_requirement TEXT NOT NULL,
            PRIMARY KEY (contract_id, contract_version, id)
        );
        CREATE TABLE plan (
            id                    TEXT NOT NULL,
            major                 INTEGER NOT NULL,
            minor                 INTEGER NOT NULL,
            intent_id             TEXT NOT NULL,
            type                  TEXT NOT NULL,
            objectives            TEXT NOT NULL,
            milestones            TEXT NOT NULL,
            assumptions           TEXT NOT NULL,
            risks                 TEXT NOT NULL,
            acceptance_conditions TEXT NOT NULL,
            review_state          TEXT NOT NULL,
            PRIMARY KEY (id, major, minor)
        );
        CREATE TABLE decision (
            id        TEXT PRIMARY KEY NOT NULL,
            intent_id TEXT NOT NULL,
            issue     TEXT NOT NULL,
            options   TEXT NOT NULL,
            blocking  INTEGER NOT NULL
        );
        CREATE TABLE decision_result (
            decision_id     TEXT PRIMARY KEY NOT NULL,
            option_index    INTEGER NOT NULL,
            decided_at      INTEGER NOT NULL,
            contract_update TEXT,
            plan_update     TEXT
        );",
    )]
}

/// 写一条 Intent。**不 `commit`**——事务由调用方持有并提交（同 P1 的 `apply_transition`
/// 形状；§318 要求状态更新与对应事件在同一事务内提交，故写函数不自己收口）。
///
/// # 首次落库写出 `IntentCreated`（§5.4）
///
/// **仅首次**（`INSERT` 分支）写出恰一条 `EventType::IntentCreated`——§5.4 的六项写入点
/// 之一，本层是它的第一个生产方。后续 save 走 `UPDATE` 分支，不写事件（「Intent 首次落库」
/// 这件事只发生一次）。
///
/// 事件的两个入参由本函数给（`save_intent` 的签名没有它们，也不该有）：
///
/// - `event_id` 派生为 `intent/{id}/created`：这是 P1 给 `ArtifactCreated` 的豁免形状
///   （`artifact/{id}`，见 `docs/superpowers/p1-followups.md` §一.2）的同一判据——
///   **一个 Intent 的首次落库恰一次**，`intent.id` 是主键，故派生 id 不可复用。
///   **限度照实写**：行被外部删掉（本层没有删除路径）后重新保存会再走一次首次落库路径、
///   撞上主键——那时 `append_event` 失败、本次 save 返回 `Err`（不静默留下第二条）。
/// - `occurred_at` 取 `intent.created_at`：首次落库的时刻，由调用方在记录里给出
///   （本层不取时钟，同 [`crate::Intent`] 的字段文档）。
///
/// 事件写在行之后：行写失败时（如表不存在）直接返回错误，不产生指向不存在 Intent 的事件
/// （同 P1 的 `save_artifact`）。事件写失败时行**已经留在事务里**——调用方必须回滚，
/// 不得提交（P1 的纪律，见 `save_intent` 上面那句与 [`crate::apply_intent_transition`]）。
/// 照片：`tests/intent_state.rs` 的 `the_intent_created_event_is_written_on_the_first_save`
/// （首次恰一条、第二次不重复）。
///
/// # 一次写入 = 插入或原地更新（与 `alias` 表**刻意不同**）
///
/// `intent` 表**一行一 Intent**（`id` 主键），一行的 `status` / 语义内容会被改写，
/// 故本函数是 upsert。**这不是 `save_alias` 那条裸 `INSERT`**：`alias` 表靠主键拒绝
/// 第二次写入（§4.6 要「提升必须是一个独立的显式操作」），而 Intent 的原地更新是
/// **规范要的**（§223 的状态机会走十几个迁移）。
///
/// # `version` 由本函数算，不是入参那一个（§5.2）
///
/// - **库里没有这个 `id`**：`version = 1`（`created_at` 取入参那一个）。
/// - **库里有**：与库里那一行逐项比**五个语义字段**——
///   `kind` / `goal` / `parent_intent` / `completion_predicate` / `contract_id`
///   （`contract_id` 含 `id` 与 `version` 两半）。**任一变化** ⇒ `version = 旧 + 1`；
///   **都不变** ⇒ `version = 旧`（`status`、`priority`、`active_plan_id`、`graph_id`、
///   `product_id` 都不在这五个里，故它们变**不** +1）。`created_at` 取库里那一个
///   （创建时刻不因一次更新而改写），`updated_at` 取入参那一个。
///
/// **这里不是「照收入参的 `version`」：因为那样它就退化成调用方的一个字段，
/// §5.2 那条规则无处可判**——本层要的是「同一个 `id` 的第二条记录，语义变了没变」
/// 这个判定由库这边做，否则两条路径各按自己的算法填版本，没有任何东西能把它们对上。
/// 入参的 `Intent::version` 因此**在写侧被忽略**（它是给读侧用的字段：
/// [`load_intent`] 读回来的那一枚才是权威）。
///
/// **比较的是落库形态**，不是类型上的相等：`Opaque` / `EntityId` 都没有 `PartialEq`
/// （设计没有给「两个不透明串相等」以外的判据，见各自的类型文档），故这里比的是
/// 这五个字段**将要写下去的那几列值**——它同时也是「语义变了没变」这个判定在库层的
/// 权威形态（库里存的就是它）。
///
/// # 子列表不落列（§5.1）
///
/// 入参的 `child_intents[]` **没有对应列**（`Intent` 结构体里也没有这个成员）：
/// 只存父指针，子列表由 [`list_children`] 反查。**两份真值会漂移**是本仓一贯判为缺陷的
/// 那一类（同 `design:842`）。照片：`tests/intent.rs` 的
/// `a_child_list_is_not_itself_stored` 的 (b) 半边（`PRAGMA table_info(intent)`
/// 的列清单里没有 `child_intents`）。
pub fn save_intent(tx: &Tx<'_>, intent: &Intent) -> Result<(), IntentError> {
    let semantic = semantic_columns(intent);
    let existing = tx.query(
        "SELECT version, created_at, kind, goal, parent_intent, contract_id, contract_version, \
                completion_predicate \
         FROM intent WHERE id = ?1",
        &[Value::text(intent.id.as_str())],
    )?;

    let (version, created_at) = match existing.first() {
        None => (1i64, intent.created_at),
        Some(row) => {
            let old_version = int_at(row, 0)?;
            let old_created_at = int_at(row, 1)?;
            let changed = row[2..8] != semantic[..];
            let version = if changed { old_version + 1 } else { old_version };
            (version, old_created_at)
        }
    };

    let columns = [
        Value::text(intent.id.as_str()),
        Value::Int(version),
        Value::text(intent.kind.as_str()),
        Value::text(intent.goal.as_str()),
        Value::text(intent.status.as_str()),
        Value::text(intent.priority.as_str()),
        Value::Int(created_at),
        Value::Int(intent.updated_at),
        opt_text(intent.parent_intent.as_ref().map(IntentId::as_str)),
        Value::text(intent.contract_id.id.as_str()),
        Value::Int(i64::from(intent.contract_id.version)),
        opt_text(intent.active_plan_id.as_ref().map(PlanId::as_str)),
        opt_text(intent.graph_id.as_ref().map(GraphIdRef::as_str)),
        opt_text(intent.product_id.as_ref().map(ProductIdRef::as_str)),
        Value::text(intent.completion_predicate.as_str()),
    ];

    if existing.is_empty() {
        tx.execute(
            "INSERT INTO intent (id, version, kind, goal, status, priority, created_at, updated_at, \
                                 parent_intent, contract_id, contract_version, active_plan_id, \
                                 graph_id, product_id, completion_predicate) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            &columns,
        )?;

        // 首次落库写出 IntentCreated（§5.4）——**写在行之后**：行写失败时（如表不存在）
        // 不产生指向不存在 Intent 的事件。事件与行同在这个事务里，由调用方一起提交
        // 或一起回滚。
        let event = Event::new(
            EventType::IntentCreated,
            format!("intent/{}/created", intent.id.as_str()),
            intent.created_at,
            serde_json::json!({ "intent_id": intent.id.as_str() }),
        )
        .with_intent(intent.id.as_str());
        tx.append_event(&event)?;
    } else {
        tx.execute(
            "UPDATE intent SET version = ?2, kind = ?3, goal = ?4, status = ?5, priority = ?6, \
                              created_at = ?7, updated_at = ?8, parent_intent = ?9, contract_id = ?10, \
                              contract_version = ?11, active_plan_id = ?12, graph_id = ?13, \
                              product_id = ?14, completion_predicate = ?15 \
             WHERE id = ?1",
            &columns,
        )?;
    }
    Ok(())
}

/// 按 id 读一条 Intent。**不存在返回 `Ok(None)`**（那不是错误，同 `load_alias` / `load_tool`）。
///
/// `kind` / `status` 是表外取值时返回 [`IntentError::UnknownKind`] /
/// [`IntentError::UnknownStatus`]，**不取默认值**（见文件头）。
pub fn load_intent(tx: &Tx<'_>, id: &IntentId) -> Result<Option<Intent>, IntentError> {
    let rows = tx.query(
        &format!("SELECT {COLUMNS} FROM intent WHERE id = ?1"),
        &[Value::text(id.as_str())],
    )?;
    let Some(row) = rows.into_iter().next() else {
        return Ok(None);
    };
    Ok(Some(row_to_intent(&row)?))
}

/// `parent` 的全部子 Intent，**按 id 升序**（顺序固定，调用方才不必自己排）。
///
/// 这是 §5.1「子列表不落列」的**派生那一半**：`child_intents[]` 没有列，它由
/// `parent_intent` 列反查得到。故本函数与 [`save_intent`] 是一对——父指针是唯一的真值。
///
/// 与 [`load_intent`] 走同一列清单与同一个解码函数；**两条路径各自有表外取值的照片**
/// （`tests/intent.rs` 的 `enum_columns_use_lowercase_underscore_and_reject_out_of_table_values`
/// 与 `list_children_rejects_an_out_of_table_enum_too`）——它们共用解码是实现的选择，
/// 不是判据。
pub fn list_children(tx: &Tx<'_>, parent: &IntentId) -> Result<Vec<Intent>, IntentError> {
    tx.query(
        &format!("SELECT {COLUMNS} FROM intent WHERE parent_intent = ?1 ORDER BY id"),
        &[Value::text(parent.as_str())],
    )?
    .iter()
    .map(|row| row_to_intent(row))
    .collect()
}

/// §5.2 的五个语义字段，按落库形态取值（列序与 [`save_intent`] 里那条 `SELECT` 的
/// `kind` 起那六列一一对应：`kind` / `goal` / `parent_intent` / `contract_id` /
/// `contract_version` / `completion_predicate`）。
///
/// **六项装五个字段**：`contract_id` 那一半拆成 id 与 version 两列，故这里有六项。
fn semantic_columns(intent: &Intent) -> [Value; 6] {
    [
        Value::text(intent.kind.as_str()),
        Value::text(intent.goal.as_str()),
        opt_text(intent.parent_intent.as_ref().map(IntentId::as_str)),
        Value::text(intent.contract_id.id.as_str()),
        Value::Int(i64::from(intent.contract_id.version)),
        Value::text(intent.completion_predicate.as_str()),
    ]
}

/// 一行 → 一条 Intent。列序与 [`COLUMNS`] 一一对应。
///
/// `kind` / `status` 的表外取值在此转成具体的 [`IntentError::UnknownKind`] /
/// [`IntentError::UnknownStatus`]，**带库里那一个原始串**——把认不出的串猜成某一枚，
/// 正是枚举列纪律要拦的。
fn row_to_intent(row: &[Value]) -> Result<Intent, IntentError> {
    let id = IntentId::from_str(&text_at(row, 0)?);
    let version = int_at(row, 1)? as u32;
    let raw_kind = text_at(row, 2)?;
    let kind = IntentKind::parse(&raw_kind).ok_or(IntentError::UnknownKind { value: raw_kind })?;
    let goal = continuum_canonical::EntityId::from_str(&text_at(row, 3)?);
    let raw_status = text_at(row, 4)?;
    let status =
        IntentStatus::parse(&raw_status).ok_or(IntentError::UnknownStatus { value: raw_status })?;
    let priority = Priority::from_str(&text_at(row, 5)?);
    let created_at = int_at(row, 6)?;
    let updated_at = int_at(row, 7)?;
    let parent_intent = opt_text_at(row, 8)?.map(|s| IntentId::from_str(&s));
    let contract_id = ContractRef {
        id: ContractId::from_str(&text_at(row, 9)?),
        version: int_at(row, 10)? as u32,
    };
    let active_plan_id = opt_text_at(row, 11)?.map(|s| PlanId::from_str(&s));
    let graph_id = opt_text_at(row, 12)?.map(|s| GraphIdRef::from_str(&s));
    let product_id = opt_text_at(row, 13)?.map(|s| ProductIdRef::from_str(&s));
    let completion_predicate = continuum_canonical::Opaque::from_str(&text_at(row, 14)?);
    Ok(Intent {
        id,
        version,
        kind,
        goal,
        status,
        priority,
        created_at,
        updated_at,
        parent_intent,
        contract_id,
        active_plan_id,
        graph_id,
        product_id,
        completion_predicate,
    })
}

/// `Option<&str>` → 可绑定的列值（`None` 落 `NULL`）。
fn opt_text(value: Option<&str>) -> Value {
    match value {
        Some(s) => Value::text(s),
        None => Value::Null,
    }
}

/// 按 `index` 取一列文本。**`PersistError::ColumnType` 只在本函数里构造一处**
/// （形态与 `continuum-canonical/src/persist.rs` 的 `text_at` 同）。
fn text_at(row: &[Value], index: usize) -> Result<String, PersistError> {
    match row.get(index) {
        Some(Value::Text(s)) => Ok(s.clone()),
        Some(other) => Err(PersistError::ColumnType {
            index,
            actual: kind_name(other),
        }),
        None => Err(PersistError::ColumnType {
            index,
            actual: "missing",
        }),
    }
}

/// 按 `index` 取一列可空文本。`NULL` → `None`。
///
/// **`NULL` 与「表外取值」是两件事**：前者是「这一列本来就可以空」（§222 的
/// `parent_intent?` 那一类），后者是「这一列的值不合法」——本函数只处理前者。
fn opt_text_at(row: &[Value], index: usize) -> Result<Option<String>, PersistError> {
    match row.get(index) {
        Some(Value::Null) => Ok(None),
        Some(Value::Text(s)) => Ok(Some(s.clone())),
        Some(other) => Err(PersistError::ColumnType {
            index,
            actual: kind_name(other),
        }),
        None => Err(PersistError::ColumnType {
            index,
            actual: "missing",
        }),
    }
}

/// 按 `index` 取一列整数。
///
/// **[`row_to_intent`] 与 [`current_intent_version`] 的那几处 `as u32`**（`version` 与
/// `contract_version`）走的是 `continuum-graph/src/persist.rs:235` 的 `graph.version`
/// 同一条处置：那几列的取值域由本层自己定（`u32`），写下去的就是 `i64::from(u32)`，
/// 故读回来的转换对所有本层写过的值都是精确的。**本函数本身不做转换**，它只把那一列
/// 读成 `i64`。
fn int_at(row: &[Value], index: usize) -> Result<i64, PersistError> {
    match row.get(index) {
        Some(Value::Int(i)) => Ok(*i),
        Some(other) => Err(PersistError::ColumnType {
            index,
            actual: kind_name(other),
        }),
        None => Err(PersistError::ColumnType {
            index,
            actual: "missing",
        }),
    }
}

// ===== `specification` 表的行级读写（设计 §6.1／§6.2；Task 10） =====

/// `specification` 的内容列清单（读路径用；写路径的列名是 `INSERT` 语句文本的一部分，
/// 不引用本常量）。
///
/// 七列：五个列表列（JSON 字符串数组）＋ `unknowns` 拆开的两列（`unknowns_kind` ＋
/// `unknowns`）。**不含 `intent_id` / `version`**：那两列是定位主键，[`Specification`]
/// 类型自身没有——「Specification 挂在 Intent 上」（§6.2）的挂法就是库里的那两列，
/// 读侧经 [`load_specification`] 的入参传入。
const SPEC_COLUMNS: &str = "goals, invariants, acceptance_predicates, forbidden_states, \
                            preferences, unknowns_kind, unknowns";

/// 写一个 Intent 版本与它的 Specification——**同一事务**（§6.2）。
/// **本函数不自己 `commit`**（同 [`save_intent`] 的纪律：事务由调用方持有并提交；
/// 任一写入失败后调用方必须回滚，不得提交）。
///
/// # 为什么是一个函数，不是「先 `save_intent`、再 `save_specification`」两步
///
/// §6.2 的一句话是一个**动作**：「一次语义变化产出一个新的 Intent version 与一个对应的
/// Specification；**两者同事务写**。这一条排除了『Intent 已经是 v3、Specification 还是
/// v1』这种可漂移的状态」。两个独立函数的调用顺序不是那个动作的落点——落点与 P1 的
/// `apply_transition`（状态＋事件）和 [`crate::apply_intent_transition`] 同形：
/// **一个函数写两件事，共用调用方的一个 `Tx`**。
///
/// # 版本同号的机制：读回，不照收
///
/// 版本由 [`save_intent`] 算（§5.2：五个语义字段有变化才 +1），故本函数在写完 Intent 行后
/// **读回**它的当前版本（[`current_intent_version`]）、写给 specification 行——
/// 「Specification 的版本与 Intent 的版本同号」因此是写路径的不变量，而不是调用方要记得
/// 传对的一个参数。照片：`tests/spec.rs` 的
/// `an_intent_version_and_its_specification_are_written_in_one_transaction` 的 (a)
/// （原始读回 `specification.version` 与 `intent.version` 同号）。
///
/// # 写入次序与失败路径
///
/// 先 `finalize`（纯计算，不失败），再写 Intent 行（含它首次落库的 `IntentCreated`
/// 事件），最后写 specification 行（见 [`insert_specification`] 的裸 `INSERT`）。
/// 任一步失败都返回 `Err`，而此时前一步的写入**已经留在事务里**——调用方必须回滚
/// （丢弃 `Tx`），不得提交。照片：同一条用例的 (b)（spec 主键冲突 ⇒ `Err`；
/// **回滚后** intent 的 `updated_at` 与 specification 的内容都回到失败前的值，
/// 不是只断言 `Err` 是哪一枚）。
///
/// # 事件面
///
/// 本函数不写自己的事件：§5.4 的六个写入点里与它相关的只有 [`save_intent`] 的
/// `IntentCreated`（仅首次）。Specification 的首次落库今天没有对应事件——
/// §5.4 的表里没有它。
pub fn save_intent_with_specification(
    tx: &Tx<'_>,
    intent: &Intent,
    draft: SpecDraft,
    unknowns: Unknowns,
) -> Result<(), SpecError> {
    let specification = draft.finalize(unknowns);
    save_intent(tx, intent)?;
    let version = current_intent_version(tx, &intent.id)?;
    insert_specification(tx, &intent.id, version, &specification)?;
    Ok(())
}

/// 读 [`save_intent_with_specification`] 刚写下的那个 Intent 版本号——specification 行与它
/// 同号。
///
/// **读不到是不可能的**（本函数总在该 Intent 写成功之后被调），但代码上不 `unwrap`：
/// 给 [`IntentError::UnknownIntent`]（带 id）——与「本层不静默创建」的处置一致
/// （同 [`crate::apply_intent_transition`] 的第一步）。
fn current_intent_version(tx: &Tx<'_>, id: &IntentId) -> Result<u32, SpecError> {
    let rows = tx.query(
        "SELECT version FROM intent WHERE id = ?1",
        &[Value::text(id.as_str())],
    )?;
    match rows.first() {
        Some(row) => Ok(int_at(row, 0)? as u32),
        None => Err(SpecError::Intent(IntentError::UnknownIntent {
            id: id.as_str().to_owned(),
        })),
    }
}

/// 写一行 `specification`。**裸 `INSERT`，不是 `INSERT OR REPLACE`**（同 `alias` 表与
/// P3D 的观测表的处置）：同 `(intent_id, version)` 的第二次写入由**复合主键**拒绝。
/// 这是刻意的 fail-closed——§6.2 排除的漂移态是「同一版 Intent 挂两份不同的规格」，
/// 允许覆盖就让它重新可表达；要改规格，先让 Intent 进一个新版本（语义变化 ⇒ `version` +1）。
/// 照片：`tests/spec.rs` 的
/// `an_intent_version_and_its_specification_are_written_in_one_transaction` 的 (b)。
///
/// 编码：五列列表走 [`encode_members`]（元素取各类型自己的 `as_str`），
/// `unknowns` 两列走 [`encode_unknowns`]。**本文件不自建第二份编码。**
fn insert_specification(
    tx: &Tx<'_>,
    id: &IntentId,
    version: u32,
    spec: &Specification,
) -> Result<(), SpecError> {
    let (unknowns_kind, unknowns) = encode_unknowns(spec.unknowns());
    tx.execute(
        "INSERT INTO specification (intent_id, version, goals, invariants, acceptance_predicates, \
                                    forbidden_states, preferences, unknowns_kind, unknowns) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        &[
            Value::text(id.as_str()),
            Value::Int(i64::from(version)),
            Value::text(encode_members(spec.goals(), Goal::as_str, "goals")?),
            Value::text(encode_members(spec.invariants(), Invariant::as_str, "invariants")?),
            Value::text(encode_members(
                spec.acceptance_predicates(),
                AcceptancePredicate::as_str,
                "acceptance_predicates",
            )?),
            Value::text(encode_members(
                spec.forbidden_states(),
                ForbiddenState::as_str,
                "forbidden_states",
            )?),
            Value::text(encode_members(spec.preferences(), Preference::as_str, "preferences")?),
            Value::text(unknowns_kind),
            Value::text(unknowns),
        ],
    )?;
    Ok(())
}

/// 按 `(intent_id, version)` 读一条 Specification。**不存在返回 `Ok(None)`**
/// （那不是错误，同 [`load_intent`] / `load_alias`）。
///
/// 版本必须**显式给出**：Specification 的版本与 Intent 的版本同号（§6.2），消费方
/// （如之后 Contract 编译）按它拿那一版的规格；「最近一版」由调用方自己先读 Intent 行。
///
/// `unknowns_kind` / `unknown` 的 kind 是表外取值时返回 [`SpecError::UnknownsKind`] /
/// [`SpecError::UnknownItemKind`]，**不取默认值**；列表列不是 JSON 数组、
/// `unknowns` 元素不是 `{kind, value}` 对象时返回 [`SpecError::Persist`] 的 `Database`
/// 形状（「容器坏」与「表外取值」两类失败的分工，见 [`SpecError`] 的文档）。
/// 照片：`tests/spec.rs` 的
/// `the_unknowns_columns_use_lowercase_underscore_and_reject_out_of_table_values`（五段）。
pub fn load_specification(
    tx: &Tx<'_>,
    id: &IntentId,
    version: u32,
) -> Result<Option<Specification>, SpecError> {
    let rows = tx.query(
        &format!("SELECT {SPEC_COLUMNS} FROM specification WHERE intent_id = ?1 AND version = ?2"),
        &[Value::text(id.as_str()), Value::Int(i64::from(version))],
    )?;
    let Some(row) = rows.into_iter().next() else {
        return Ok(None);
    };
    Ok(Some(row_to_specification(&row)?))
}

/// 一行 → 一条 Specification。列序与 [`SPEC_COLUMNS`] 一一对应。
fn row_to_specification(row: &[Value]) -> Result<Specification, SpecError> {
    let goals = decode_members(&text_at(row, 0)?, Goal::from_str, "goals")?;
    let invariants = decode_members(&text_at(row, 1)?, Invariant::from_str, "invariants")?;
    let acceptance_predicates = decode_members(
        &text_at(row, 2)?,
        AcceptancePredicate::from_str,
        "acceptance_predicates",
    )?;
    let forbidden_states = decode_members(
        &text_at(row, 3)?,
        ForbiddenState::from_str,
        "forbidden_states",
    )?;
    let preferences = decode_members(&text_at(row, 4)?, Preference::from_str, "preferences")?;
    let unknowns = decode_unknowns(&text_at(row, 5)?, &text_at(row, 6)?)?;
    Ok(Specification::new(
        goals,
        invariants,
        acceptance_predicates,
        forbidden_states,
        preferences,
        unknowns,
    ))
}

/// 自由文本列表列的容器编码：JSON 字符串数组。
///
/// 与 `continuum-model-registry` 的 `encode_string_list` 同一处置：**只做容器**——
/// 元素的编码取各类型自己的 `as_str`（经 `fn` 指针传入，调用点看得到用的是哪一个）。
/// 选 JSON 数组的理由同那三个列表列（元素是自由文本，定长分隔符在有取值域的那天
/// 会撞上取值本身）。
fn encode_members<T>(
    items: &[T],
    as_str: fn(&T) -> &str,
    column: &str,
) -> Result<String, PersistError> {
    let strs: Vec<&str> = items.iter().map(as_str).collect();
    serde_json::to_string(&strs)
        .map_err(|e| PersistError::Database(format!("specification.{column} 不可序列化为 JSON: {e}")))
}

/// [`encode_members`] 的逆。**容器坏了才报错**：元素是自由文本，没有表外取值可言。
fn decode_members<T>(
    raw: &str,
    from_str: fn(&str) -> T,
    column: &str,
) -> Result<Vec<T>, PersistError> {
    let strs: Vec<String> = serde_json::from_str(raw).map_err(|e| {
        PersistError::Database(format!(
            "specification.{column} 不是字符串数组（{raw}）: {e}"
        ))
    })?;
    Ok(strs.iter().map(|s| from_str(s.as_str())).collect())
}

/// `unknowns` 两列的编码。
///
/// - `unknowns_kind` 取 [`Unknowns::kind_str`]（**编码与类型同址**，本文件不自建第二份）；
/// - `unknowns` 列：`NoneFound` 写空列表 `[]`，`Enumerated` 写 `{kind, value}` 对象数组。
///   两枚的区分**完全由 kind 列承担**，内容列只承载列表本身——这正是 `NoneFound` 与
///   `Enumerated(vec![])` 不塌成同一个表示的原因（§227）。
///   `Value::to_string()` 是 serde_json 的 Display（紧凑 JSON），对 `Value` 不失败。
fn encode_unknowns(unknowns: &Unknowns) -> (String, String) {
    let list = match unknowns {
        Unknowns::NoneFound => serde_json::Value::Array(Vec::new()),
        Unknowns::Enumerated(items) => serde_json::Value::Array(
            items
                .iter()
                .map(|u| serde_json::json!({"kind": u.kind_str(), "value": u.description()}))
                .collect(),
        ),
    };
    (unknowns.kind_str().to_owned(), list.to_string())
}

/// [`encode_unknowns`] 的逆。
///
/// # 两条失败面（与 [`decode_members`] 的分工）
///
/// - **表外取值**（[`Unknowns::parse_kind`] / [`Unknown::parse_kind`] 给 `None`）：
///   `unknowns_kind` 不是两枚之一 → [`SpecError::UnknownsKind`]；元素 `kind` 不是两枚之一
///   → [`SpecError::UnknownItemKind`]。两处都**带库里那一个原始串、不取默认值**（枚举列纪律）。
/// - **容器坏**：`unknowns` 不是 JSON 数组、或元素不是 `{kind, value}` 对象 →
///   [`SpecError::Persist`] 的 `Database` 形状（同 [`decode_members`]：「容器坏」
///   与「表外取值」是两类失败）。
fn decode_unknowns(kind: &str, raw: &str) -> Result<Unknowns, SpecError> {
    match Unknowns::parse_kind(kind) {
        // `NoneFound` 时 `raw` 不参与解码：`unknowns_kind` 列是权威，内容列按约定是 `[]`。
        Some(UnknownsKind::NoneFound) => Ok(Unknowns::NoneFound),
        Some(UnknownsKind::Enumerated) => {
            let items: Vec<serde_json::Value> = serde_json::from_str(raw).map_err(|e| {
                PersistError::Database(format!(
                    "specification.unknowns 不是 JSON 数组（{raw}）: {e}"
                ))
            })?;
            let mut out = Vec::with_capacity(items.len());
            for item in &items {
                let (item_kind, value) = match (
                    item.get("kind").and_then(|v| v.as_str()),
                    item.get("value").and_then(|v| v.as_str()),
                ) {
                    (Some(k), Some(v)) => (k, v),
                    _ => {
                        return Err(PersistError::Database(format!(
                            "specification.unknowns 的元素不是 {{kind, value}} 对象: {item}"
                        ))
                        .into());
                    }
                };
                // 白名单在 [`Unknown::parse_kind`]（与类型同址）；错误构造留在这里。
                let unknown = Unknown::parse_kind(item_kind, value)
                    .ok_or_else(|| SpecError::UnknownItemKind { value: item_kind.to_owned() })?;
                out.push(unknown);
            }
            Ok(Unknowns::Enumerated(out))
        }
        None => Err(SpecError::UnknownsKind {
            value: kind.to_owned(),
        }),
    }
}

// ===== `contract` / `contract_requirement` 表的行级读写（设计 §7.1／§7.3；Task 11） =====

/// `contract` 表的内容列清单（读路径用；写路径的列名是 `INSERT` 语句文本的一部分，
/// 不引用本常量）。
///
/// 七列 = `TaskContract` 字段减主键两列（`id` / `version`）与 `requirements[]`
/// （后者落 `contract_requirement` 表）。
const CONTRACT_COLUMNS: &str = "intent_id, budget, authority_limit, data_policy, model_policy, \
                                node_policy, acceptance_predicate";

/// 写一条 TaskContract（**只 INSERT，无 UPDATE 路径**，§226），并在同一事务内写一条
/// `AuditKind::ContractChanges`（§5.4）。**本函数不自己 `commit`**（同 [`save_intent`]
/// 的纪律：事务由调用方持有并提交；任一写入失败后调用方必须回滚，不得提交）。
///
/// # `occurred_at`：签名比任务简报的速写多一个参数（**申报**）
///
/// 简报 Step 3 的速写是 `(tx, c) -> Result<(), ContractError>`。落地时补上它，
/// 理由两条：
///
/// 1. **审计的 `occurred_at` 没有第二个来源**：`Tx::append_audit` 必填它，而本层
///    **不取系统时钟**（Task 21 的源码面复核 `SystemTime::now` 零命中），故只能由
///    调用方给。先例：本 crate Task 9 的 `apply_intent_transition(…, occurred_at, …)`、
///    P1 的 `apply_transition(…, occurred_at, …)`、P3 的 `authorize(…, now)` 与
///    P2 的 gate（`Gate::audit` 的 `occurred_at` 由调用方给出）。
/// 2. **签名形状照 Task 9**（`to` 后接 `occurred_at`）：本条没有事件 id 可给
///    （审计没有 id 字段，链的 `seq` 由 `append_audit` 自己推）。
///
/// # `version` 由调用方给，不存在「自动 +1」
///
/// §226 说「用户修改要求时 MUST 创建 `Contract vN+1`」——「新版本」是一个**显式动作**。
/// 本函数不读旧版本、不替调用方算 N+1：若那样，同 `(id, version)` 的二次写会被
/// 静默升版写下去，而 §226 的 MUST NOT 原地修改就没有落点了。入参给哪个版本，
/// 写下去的就是哪个版本；已存在的 `(id, version)` 撞主键得 `Err`（**绝不覆盖**）。
///
/// # 写入次序与失败路径
///
/// 先写 `contract` 行，再逐条写 `contract_requirement` 行，最后写审计——审计**写在行之后**
/// （同 [`save_intent`] 的事件次序）：任一步失败都返回 `Err`，而此时前一步的写入
/// **已经留在事务里**——调用方必须回滚（丢弃 `Tx`），不得提交。照片：
/// `tests/contract.rs` 的 `a_new_version_writes_exactly_one_contract_changes_audit_row`
/// 的 (c)（`audit_log` 表缺失 ⇒ `Err`；**回滚后两张表都没有半写的行**）。
///
/// # 审计面
///
/// 恰一条 `AuditKind::ContractChanges`（§5.4 的六项写入点之一；`EventType` /
/// `AuditKind` 的变体一个不改、一张表不加）。payload 是 JSON 对象
/// （`contract_id` / `version` / `intent_id`），供后续对账取用。
pub fn save_contract(
    tx: &Tx<'_>,
    contract: &TaskContract,
    occurred_at: i64,
) -> Result<(), ContractError> {
    tx.execute(
        "INSERT INTO contract (id, version, intent_id, budget, authority_limit, data_policy, \
                                model_policy, node_policy, acceptance_predicate) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        &[
            Value::text(contract.id.as_str()),
            Value::Int(i64::from(contract.version)),
            Value::text(contract.intent_id.as_str()),
            Value::text(contract.budget.as_str()),
            Value::text(contract.authority_limit.as_str()),
            Value::text(contract.data_policy.as_str()),
            Value::text(contract.model_policy.as_str()),
            Value::text(contract.node_policy.as_str()),
            Value::text(contract.acceptance_predicate.as_str()),
        ],
    )?;

    for requirement in &contract.requirements {
        insert_requirement(tx, contract, requirement)?;
    }

    tx.append_audit(
        AuditKind::ContractChanges,
        occurred_at,
        Value::text(
            serde_json::to_string(&serde_json::json!({
                "contract_id": contract.id.as_str(),
                "version": contract.version,
                "intent_id": contract.intent_id.as_str(),
            }))
            .expect("payload 由 json! 构造，必可序列化"),
        ),
    )?;
    Ok(())
}

/// 写一行 `contract_requirement`。**裸 `INSERT`**：同
/// `(contract_id, contract_version, id)` 的第二次写入由复合主键拒绝（同一版本里
/// 重复的 requirement id 因此不会静默产生两行——它撞键得 `Err`）。
fn insert_requirement(
    tx: &Tx<'_>,
    contract: &TaskContract,
    requirement: &Requirement,
) -> Result<(), ContractError> {
    tx.execute(
        "INSERT INTO contract_requirement (contract_id, contract_version, id, class, \
                                            expression, source, verification_requirement) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        &[
            Value::text(contract.id.as_str()),
            Value::Int(i64::from(contract.version)),
            Value::text(requirement.id.as_str()),
            Value::text(requirement.class.as_str()),
            Value::text(requirement.expression.as_str()),
            Value::text(requirement.source.as_str()),
            Value::text(requirement.verification_requirement.as_str()),
        ],
    )?;
    Ok(())
}

/// 按 `(id, version)` 读一条 TaskContract。**不存在返回 `Ok(None)`**
/// （那不是错误，同 [`load_intent`] / [`load_specification`]）。
///
/// 版本必须**显式给出**（同 [`load_specification`]）：§226 的版本化是「哪个版本读哪个
/// 版本」，「最近一版」由调用方自己先算（本函数不替它挑）。
///
/// `requirements` 按**写入顺序**（行写入顺序，`ORDER BY rowid`——同 P0 的
/// `scan_event_log` 表达事件写序的处置）读回。**§224 未给 requirements 的顺序语义**，
/// 这里保存事实：调用方写入的顺序就是读回的顺序；本函数不做 id 排序。
///
/// `class` 是表外取值时返回 [`ContractError::UnknownClass`]，**不取默认值**
/// （见文件头）；其余列是不透明串，逐字节进出。
pub fn load_contract(
    tx: &Tx<'_>,
    id: &ContractId,
    version: u32,
) -> Result<Option<TaskContract>, ContractError> {
    let rows = tx.query(
        &format!("SELECT {CONTRACT_COLUMNS} FROM contract WHERE id = ?1 AND version = ?2"),
        &[Value::text(id.as_str()), Value::Int(i64::from(version))],
    )?;
    let Some(row) = rows.into_iter().next() else {
        return Ok(None);
    };
    let requirements = load_requirements(tx, id, version)?;
    Ok(Some(TaskContract {
        id: id.clone(),
        version,
        intent_id: IntentId::from_str(&text_at(&row, 0)?),
        requirements,
        budget: Opaque::from_str(&text_at(&row, 1)?),
        authority_limit: Opaque::from_str(&text_at(&row, 2)?),
        data_policy: Opaque::from_str(&text_at(&row, 3)?),
        model_policy: Opaque::from_str(&text_at(&row, 4)?),
        node_policy: Opaque::from_str(&text_at(&row, 5)?),
        acceptance_predicate: Opaque::from_str(&text_at(&row, 6)?),
    }))
}

/// 一条 contract 版本的全部 requirement 行，按写入顺序。
///
/// 列序与 [`insert_requirement`] 的 `INSERT` 列清单一一对应：
/// `id` / `class` / `expression` / `source` / `verification_requirement`。
fn load_requirements(
    tx: &Tx<'_>,
    id: &ContractId,
    version: u32,
) -> Result<Vec<Requirement>, ContractError> {
    tx.query(
        "SELECT id, class, expression, source, verification_requirement \
         FROM contract_requirement WHERE contract_id = ?1 AND contract_version = ?2 \
         ORDER BY rowid",
        &[Value::text(id.as_str()), Value::Int(i64::from(version))],
    )?
    .iter()
    .map(|row| {
        let raw_class = text_at(row, 1)?;
        let class = RequirementClass::parse(&raw_class)
            .ok_or(ContractError::UnknownClass { value: raw_class })?;
        Ok(Requirement {
            id: RequirementId::from_str(&text_at(row, 0)?),
            class,
            expression: Opaque::from_str(&text_at(row, 2)?),
            source: Opaque::from_str(&text_at(row, 3)?),
            verification_requirement: VerificationRequirement::from_str(&text_at(row, 4)?),
        })
    })
    .collect()
}

// ===== `plan` 表的行级读写（设计 §9.1；Task 15 落地 Task 13 预告的那一半） =====

/// 写一行 `plan` 版本。**裸 `INSERT`，不是 `INSERT OR REPLACE`**（同 `contract` 的
/// `(id, version)` 与 `alias` 表的处置）：同 `(id, major, minor)` 的第二次写入由
/// **复合主键**拒绝——多版本并存（§213）的落库形态是「一个版本一行」，
/// 允许覆盖会让「历史版本不变」这件事重新可表达。
///
/// # 为什么没有 `occurred_at`（与 [`save_contract`] 不同）
///
/// `plan` 表**没有时间列**（§228 的十字段里没有创建 / 更新时刻），也没有与它挂钩的
/// 事件 / 审计（§5.4 的写入点表里没有 plan 行），故本函数没有可给时间的消费者。
/// **修订语义的另一半**（旧版本转 `SUPERSEDED`）不在本函数里——它是
/// [`crate::apply_result`] 的 Plan 更新（「修订」是一次动作的两件事，Task 13 的
/// [`crate::revise`] 已把值层写死；落库版的这两件事同址在调用它的恢复链路里）。
///
/// # 写入次序与失败路径
///
/// 只写一行；失败返回 `Err`（撞主键也走 [`PlanError::Persist`] 的 `Database` 形状，
/// 同 [`crate::save_contract`]）。**本函数不自己 `commit`**（同其余写函数的纪律）。
pub fn save_plan(tx: &Tx<'_>, plan: &Plan) -> Result<(), PlanError> {
    tx.execute(
        "INSERT INTO plan (id, major, minor, intent_id, type, objectives, milestones, \
                           assumptions, risks, acceptance_conditions, review_state) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        &[
            Value::text(plan.id.as_str()),
            Value::Int(i64::from(plan.version.major)),
            Value::Int(i64::from(plan.version.minor)),
            Value::text(plan.intent_id.as_str()),
            Value::text(plan.r#type.as_str()),
            Value::text(encode_plan_column(&plan.objectives, "objectives")?),
            Value::text(encode_plan_column(&plan.milestones, "milestones")?),
            Value::text(encode_plan_column(&plan.assumptions, "assumptions")?),
            Value::text(encode_plan_column(&plan.risks, "risks")?),
            Value::text(encode_plan_column(
                &plan.acceptance_conditions,
                "acceptance_conditions",
            )?),
            Value::text(plan.review_state.as_str()),
        ],
    )?;
    Ok(())
}

/// 按 `(id, version)` 读一条 Plan。**不存在返回 `Ok(None)`**（那不是错误，同
/// [`load_contract`]）。
///
/// `version` 必须**显式给出**（同 [`load_contract`]）：两段式版本的「最近一版」由
/// 调用方自己先算（本函数不替它挑）。
///
/// `type` / `review_state` 是表外取值时返回 [`PlanError::UnknownType`] /
/// [`PlanError::UnknownReview`]，**不取默认值**；列表列不是 JSON 字符串数组时返回
/// [`PlanError::Persist`] 的 `Database` 形状（「容器坏」与「表外取值」两类失败的分工，
/// 同 [`load_specification`]）。
pub fn load_plan(
    tx: &Tx<'_>,
    id: &PlanId,
    version: PlanVersion,
) -> Result<Option<Plan>, PlanError> {
    let rows = tx.query(
        "SELECT intent_id, type, objectives, milestones, assumptions, risks, \
                acceptance_conditions, review_state \
         FROM plan WHERE id = ?1 AND major = ?2 AND minor = ?3",
        &[
            Value::text(id.as_str()),
            Value::Int(i64::from(version.major)),
            Value::Int(i64::from(version.minor)),
        ],
    )?;
    let Some(row) = rows.into_iter().next() else {
        return Ok(None);
    };
    let raw_type = text_at(&row, 1)?;
    let plan_type = PlanType::parse(&raw_type).ok_or(PlanError::UnknownType { value: raw_type })?;
    let raw_review = text_at(&row, 7)?;
    let review_state =
        ReviewState::parse(&raw_review).ok_or(PlanError::UnknownReview { value: raw_review })?;
    Ok(Some(Plan {
        id: id.clone(),
        version,
        intent_id: IntentId::from_str(&text_at(&row, 0)?),
        r#type: plan_type,
        objectives: decode_plan_column(&text_at(&row, 2)?, "objectives")?,
        milestones: decode_plan_column(&text_at(&row, 3)?, "milestones")?,
        assumptions: decode_plan_column(&text_at(&row, 4)?, "assumptions")?,
        risks: decode_plan_column(&text_at(&row, 5)?, "risks")?,
        acceptance_conditions: decode_plan_column(&text_at(&row, 6)?, "acceptance_conditions")?,
        review_state,
    }))
}

/// `plan` 表的列表列容器编码：JSON 字符串数组（与 [`encode_members`] 同形，**只做
/// 容器**——元素是 [`Opaque`]，直接 `as_str`）。**不复用 [`encode_members`]**：那一个
/// 的失败消息指向 `specification` 表，复用会让错误指错表。
fn encode_plan_column(items: &[Opaque], column: &str) -> Result<String, PersistError> {
    let strs: Vec<&str> = items.iter().map(Opaque::as_str).collect();
    serde_json::to_string(&strs)
        .map_err(|e| PersistError::Database(format!("plan.{column} 不可序列化为 JSON: {e}")))
}

/// [`encode_plan_column`] 的逆。**容器坏了才报错**：元素是自由文本。
fn decode_plan_column(raw: &str, column: &str) -> Result<Vec<Opaque>, PersistError> {
    let strs: Vec<String> = serde_json::from_str(raw).map_err(|e| {
        PersistError::Database(format!("plan.{column} 不是字符串数组（{raw}）: {e}"))
    })?;
    Ok(strs.iter().map(|s| Opaque::from_str(s)).collect())
}

// ===== `decision` 表的行级读写与 §331 的创建（设计 §10.1；Task 15） =====

/// 创建一张 Decision：写 `decision` 行 ＋ 恰一条 `EventType::DecisionRequired`
/// （§5.4 的写入点之一，本层是它的首个生产方）。**本函数不自己 `commit`**（同其余
/// 写函数的纪律：事务由调用方持有并提交；任一步失败后调用方必须回滚）。
///
/// # `occurred_at`：签名比任务简报的速写多一个参数（**申报**）
///
/// 简报 Step 3 的速写只给了 `Decision` 与 `DecisionOption` 的形状，未点
/// `create_decision` 的签名；事件必填 `occurred_at`（`Tx::append_event`），而本层
/// **不取系统时钟**——故只能由调用方给（先例：本 crate 的 `save_intent`
/// 用 `created_at`、[`crate::save_contract`] 的 `occurred_at` 参数）。
///
/// # `blocking` 照收调用方给出的事实
///
/// 本函数**不从任何判定算 `blocking`**（§10.1 的两个触发点是调用方的场景）；
/// `false` 的触发条件本层不发明（见 `decision.rs` 的模块文档）。
///
/// # 事件 id 派生为 `decision/{id}/required`
///
/// 同 `save_intent` 的 `intent/{id}/created` 的豁免判据：**一张决策单的首次落库恰
/// 一次**（`decision.id` 是主键），故派生 id 不可复用。事件的 `occurred_at` 与
/// payload 的 `decision_id` 见 [`create_decision`] 的调用点；事件写在行之后——
/// 行写失败（如表不存在 / 同 id 二次创建撞主键）时不产生指向不存在决策单的事件。
///
/// 照片：`tests/decision.rs` 的
/// `a_decision_required_event_is_written_on_creation` 与
/// `a_second_creation_of_the_same_id_hits_the_primary_key`。
pub fn create_decision(
    tx: &Tx<'_>,
    decision: &Decision,
    occurred_at: i64,
) -> Result<(), DecisionError> {
    tx.execute(
        "INSERT INTO decision (id, intent_id, issue, options, blocking) VALUES (?1, ?2, ?3, ?4, ?5)",
        &[
            Value::text(decision.id.as_str()),
            Value::text(decision.intent_id.as_str()),
            Value::text(decision.issue.as_str()),
            Value::text(encode_options(&decision.options)),
            Value::Int(i64::from(decision.blocking)),
        ],
    )?;

    let event = Event::new(
        EventType::DecisionRequired,
        format!("decision/{}/required", decision.id.as_str()),
        occurred_at,
        serde_json::json!({ "decision_id": decision.id.as_str() }),
    )
    .with_intent(decision.intent_id.as_str());
    tx.append_event(&event)?;
    Ok(())
}

/// 按 id 读一张 Decision。**不存在返回 `Ok(None)`**（那不是错误，同 [`load_intent`]）。
///
/// `blocking` 列不是 0/1 时返回 [`DecisionError::InvalidBlocking`]（带原始数值，
/// **不取默认值**）；`options` 列不是 `{option, consequence}` 对象数组时返回
/// [`DecisionError::Persist`] 的 `Database` 形状（「容器坏」与「表外取值」两类失败）。
pub fn load_decision(tx: &Tx<'_>, id: &DecisionId) -> Result<Option<Decision>, DecisionError> {
    let rows = tx.query(
        "SELECT id, intent_id, issue, options, blocking FROM decision WHERE id = ?1",
        &[Value::text(id.as_str())],
    )?;
    let Some(row) = rows.into_iter().next() else {
        return Ok(None);
    };
    let blocking = match int_at(&row, 4)? {
        0 => false,
        1 => true,
        other => return Err(DecisionError::InvalidBlocking { value: other }),
    };
    Ok(Some(Decision {
        id: DecisionId::from_str(&text_at(&row, 0)?),
        intent_id: IntentId::from_str(&text_at(&row, 1)?),
        issue: Opaque::from_str(&text_at(&row, 2)?),
        options: decode_options(&text_at(&row, 3)?)?,
        blocking,
    }))
}

/// `options` 列的容器编码：JSON 数组，元素 `{"option": ..., "consequence": ...}`——
/// **一组配对对象**（不是两个平行数组；理由见 [`p4_semantics_migrations`] 的文档）。
fn encode_options(options: &[DecisionOption]) -> String {
    let list: Vec<serde_json::Value> = options
        .iter()
        .map(|o| {
            serde_json::json!({
                "option": o.option.as_str(),
                "consequence": o.consequence.as_str(),
            })
        })
        .collect();
    serde_json::Value::Array(list).to_string()
}

/// [`encode_options`] 的逆。**容器坏了才报错**（元素是自由文本，没有表外取值可言）。
fn decode_options(raw: &str) -> Result<Vec<DecisionOption>, DecisionError> {
    let items: Vec<serde_json::Value> = serde_json::from_str(raw).map_err(|e| {
        PersistError::Database(format!("decision.options 不是 JSON 数组（{raw}）: {e}"))
    })?;
    let mut out = Vec::with_capacity(items.len());
    for item in &items {
        let (option, consequence) = match (
            item.get("option").and_then(|v| v.as_str()),
            item.get("consequence").and_then(|v| v.as_str()),
        ) {
            (Some(o), Some(c)) => (o, c),
            _ => {
                return Err(PersistError::Database(format!(
                    "decision.options 的元素不是 {{option, consequence}} 对象: {item}"
                ))
                .into());
            }
        };
        out.push(DecisionOption::new(
            Opaque::from_str(option),
            Opaque::from_str(consequence),
        ));
    }
    Ok(out)
}

// ===== `decision_result` 表与 §332 的恢复链（设计 §10.2；Task 15） =====

/// `DecisionResult` 落库：写 `decision_result` 行 ＋ 恰一条 `AuditKind::UserApprovals`
/// 审计（§5.4 的写入点之一；**本层是它的第二个生产方**——第一个是
/// `continuum-workspace` 的 Integration Gate，载荷形状不同，见 `tests/decision.rs` 的
/// `a_user_approvals_audit_row_is_written_for_a_decision_result` 的申报）。
/// **本函数不自己 `commit`**。
///
/// # 次序与失败路径
///
/// 先写行、再写审计（同 [`save_contract`] 的次序）：行写失败（如同 `decision_id` 的
/// 二次落库撞主键）时不产生指向不存在结果的审计；审计写失败时行**已经留在事务里**
/// ——调用方必须回滚。审计的 `occurred_at` 取 `r.decided_at`（本层不取时钟；
/// 决定时刻是记录里那一个）。
///
/// payload 是**决策单与被选项**（`decision_id` ＋ `option_index`）——与 Gate 的
/// 操作名载荷同 kind 不同形，读 `audit_log` 的一方不得假定「一个 kind 一种载荷」
/// （§5.4、§16 第 33 条）。
pub fn save_decision_result(tx: &Tx<'_>, r: &DecisionResult) -> Result<(), DecisionError> {
    let contract_update = r.contract_update.as_ref().map(encode_contract_update);
    let plan_update = r.plan_update.as_ref().map(encode_plan_update);
    tx.execute(
        "INSERT INTO decision_result (decision_id, option_index, decided_at, contract_update, \
                                       plan_update) \
         VALUES (?1, ?2, ?3, ?4, ?5)",
        &[
            Value::text(r.decision_id.as_str()),
            Value::Int(r.option_index as i64),
            Value::Int(r.decided_at),
            opt_text(contract_update.as_deref()),
            opt_text(plan_update.as_deref()),
        ],
    )?;

    tx.append_audit(
        AuditKind::UserApprovals,
        r.decided_at,
        Value::text(
            serde_json::json!({
                "decision_id": r.decision_id.as_str(),
                "option_index": r.option_index,
            })
            .to_string(),
        ),
    )?;
    Ok(())
}

/// §332 的恢复链（§10.2），**四步同一事务**——本函数不自己 `commit`：
///
/// 1. `DecisionResult` 落库（[`save_decision_result`]：行 ＋ `UserApprovals` 审计）；
/// 2. Contract 新版本（若 `r.contract_update` 是 `Some`）；
/// 3. Plan 新版本（若 `r.plan_update` 是 `Some`）；旧版本转 `SUPERSEDED`；
/// 4. Intent 状态回迁（`SUSPENDED` → `RUNNING`，经 [`crate::apply_intent_transition`]；
///    事件 id 派生为 `decision/{id}/resume`——一个决策恰一条结果，故恰一次恢复）。
///
/// **`Graph invalidation` 不在本层**（§332 的四步里那一步归执行层，本层不碰图，§8.2）。
///
/// # 输入只有 `r`（§107）
///
/// 「不得将其视为普通会话消息并重新解释整个任务」的落点：Contract / Plan 的新版本
/// 是**从基准版本拷贝再套指名 patch**（[`patch_contract`] / [`patch_plan`]），
/// 不是重新生成——照片 (a) 见 `tests/decision.rs` 的
/// `a_decision_result_changes_only_the_fields_it_names`。
///
/// # 次序与失败路径
///
/// 先**预读并校验**（Decision 存在、`option_index` 在界内、两个 update 的基准存在、
/// 指名改的 requirement 都在基准里），**再**按上列次序写——非法输入不写任何一行
/// （同 [`crate::apply_intent_transition`] 的「先判后写」）。写到一半失败（缺
/// `audit_log` 表、新版本撞主键、Intent 回迁非法）时，前面的写入**已经留在事务里**
/// ——调用方必须回滚，不得提交。四张表（`decision_result` / `contract` / `plan` /
/// `intent`）在回滚后都没有半写行的照片：`tests/decision.rs` 的
/// `the_recovery_chain_is_atomic`（四个失败点各起一个库）。
pub fn apply_result(tx: &Tx<'_>, r: &DecisionResult) -> Result<(), DecisionError> {
    // ---- 预读与校验（不写任何一行）----
    let Some(decision) = load_decision(tx, &r.decision_id)? else {
        // 不静默创建——登记是 `create_decision` 的活（同 `UnknownIntent` 的处置）。
        return Err(DecisionError::UnknownDecision {
            id: r.decision_id.as_str().to_owned(),
        });
    };
    if r.option_index >= decision.options.len() {
        return Err(DecisionError::UnknownOption {
            decision_id: r.decision_id.as_str().to_owned(),
            index: r.option_index,
        });
    }

    let new_contract = match &r.contract_update {
        None => None,
        Some(update) => {
            let Some(base) = load_contract(tx, &update.id, update.base_version)? else {
                return Err(DecisionError::UnknownContract {
                    id: update.id.as_str().to_owned(),
                    version: update.base_version,
                });
            };
            Some(patch_contract(&base, update)?)
        }
    };

    let new_plan = match &r.plan_update {
        None => None,
        Some(update) => {
            let Some(base) = load_plan(tx, &update.id, update.base_version)? else {
                return Err(DecisionError::UnknownPlan {
                    id: update.id.as_str().to_owned(),
                    major: update.base_version.major,
                    minor: update.base_version.minor,
                });
            };
            Some((patch_plan(&base, update), update.base_version))
        }
    };

    // ---- 写入（同一事务的四个动作，次序固定）----
    save_decision_result(tx, r)?;
    if let Some(contract) = &new_contract {
        save_contract(tx, contract, r.decided_at)?;
    }
    if let Some((plan, base_version)) = &new_plan {
        save_plan(tx, plan)?;
        supersede_plan_version(tx, &plan.id, *base_version)?;
    }
    apply_intent_transition(
        tx,
        &decision.intent_id,
        IntentStatus::Running,
        r.decided_at,
        &format!("decision/{}/resume", r.decision_id.as_str()),
    )?;
    Ok(())
}

/// 从基准 Contract 拷贝再套指名 patch——**新版本 = `base_version + 1`**
/// （推进规则与照片见 [`crate::ContractUpdate`] 的文档）。
///
/// 指名一个不在基准里的 requirement id 是 [`DecisionError::UnknownRequirement`]
/// （同 Constraint Validator 悬空键的 fail-closed 处置：判不了走 `Err`，不静默添加）。
/// 未被指名的字段**逐项从基准拷贝**（照片 (a) 的机制就是这个「先拷贝再改」）。
fn patch_contract(
    base: &TaskContract,
    update: &ContractUpdate,
) -> Result<TaskContract, DecisionError> {
    let mut requirements = base.requirements.clone();
    for patch in &update.requirements {
        let Some(target) = requirements
            .iter_mut()
            .find(|requirement| requirement.id.as_str() == patch.id.as_str())
        else {
            return Err(DecisionError::UnknownRequirement {
                contract_id: update.id.as_str().to_owned(),
                version: update.base_version,
                id: patch.id.as_str().to_owned(),
            });
        };
        if let Some(class) = patch.class {
            target.class = class;
        }
        if let Some(expression) = &patch.expression {
            target.expression = expression.clone();
        }
        if let Some(source) = &patch.source {
            target.source = source.clone();
        }
        if let Some(verification_requirement) = &patch.verification_requirement {
            target.verification_requirement = verification_requirement.clone();
        }
    }
    Ok(TaskContract {
        id: base.id.clone(),
        version: base.version + 1,
        intent_id: base.intent_id.clone(),
        requirements,
        budget: base.budget.clone(),
        authority_limit: base.authority_limit.clone(),
        data_policy: base.data_policy.clone(),
        model_policy: base.model_policy.clone(),
        node_policy: base.node_policy.clone(),
        acceptance_predicate: base.acceptance_predicate.clone(),
    })
}

/// 从基准 Plan 拷贝再套指名 patch（`None` 字段不动；`Some` 是整列替换），
/// 并按修订语义推进版本：`major` +1、`minor` 归零、`review_state = DRAFT`。
///
/// 值层的两件事与 [`crate::revise`] 同源（修订 = 新版本 + 旧版本进 `SUPERSEDED`）；
/// 这里做的是第一件，第二件在 [`supersede_plan_version`]。
fn patch_plan(base: &Plan, update: &PlanUpdate) -> Plan {
    let mut next = base.clone();
    next.version = base.version.next_material();
    next.review_state = ReviewState::Draft;
    if let Some(objectives) = &update.objectives {
        next.objectives = objectives.clone();
    }
    if let Some(milestones) = &update.milestones {
        next.milestones = milestones.clone();
    }
    if let Some(assumptions) = &update.assumptions {
        next.assumptions = assumptions.clone();
    }
    if let Some(risks) = &update.risks {
        next.risks = risks.clone();
    }
    if let Some(acceptance_conditions) = &update.acceptance_conditions {
        next.acceptance_conditions = acceptance_conditions.clone();
    }
    next
}

/// 修订语义的另一半：旧版本那一行的 `review_state` 转 `SUPERSEDED`（**只动这一列**，
/// 其余逐项不变——同 [`crate::revise`] 的值层）。
///
/// `plan` 表因此有**一处** `UPDATE` 路径（与 `contract` 的「无 UPDATE」不同）——
/// 依据是 §9.2「旧版本进 `SUPERSEDED`」这件事本身；本层没有别的更新 `plan` 的路径。
fn supersede_plan_version(
    tx: &Tx<'_>,
    id: &PlanId,
    version: PlanVersion,
) -> Result<(), PlanError> {
    tx.execute(
        "UPDATE plan SET review_state = ?2 WHERE id = ?1 AND major = ?3 AND minor = ?4",
        &[
            Value::text(id.as_str()),
            Value::text(ReviewState::Superseded.as_str()),
            Value::Int(i64::from(version.major)),
            Value::Int(i64::from(version.minor)),
        ],
    )?;
    Ok(())
}

/// `ContractUpdate` 的 JSON 编码（`decision_result.contract_update` 列）：
/// requirement patch 逐条落成对象数组，`RequirementClass` 的编码取
/// [`crate::RequirementClass::as_str`]（**与类型同址**，本文件不内联字面量）。
fn encode_contract_update(update: &ContractUpdate) -> String {
    let requirements: Vec<serde_json::Value> = update
        .requirements
        .iter()
        .map(|patch| {
            serde_json::json!({
                "id": patch.id.as_str(),
                "class": patch.class.map(|class| class.as_str()),
                "expression": patch.expression.as_ref().map(|o| o.as_str()),
                "source": patch.source.as_ref().map(|o| o.as_str()),
                "verification_requirement": patch.verification_requirement.as_ref().map(|v| v.as_str()),
            })
        })
        .collect();
    serde_json::json!({
        "id": update.id.as_str(),
        "base_version": update.base_version,
        "requirements": requirements,
    })
    .to_string()
}

/// `PlanUpdate` 的 JSON 编码（`decision_result.plan_update` 列）：五个列表列各是
/// `null`（不动）或字符串数组（整列替换）。
fn encode_plan_update(update: &PlanUpdate) -> String {
    fn list(items: &[Opaque]) -> Vec<&str> {
        items.iter().map(Opaque::as_str).collect()
    }
    serde_json::json!({
        "id": update.id.as_str(),
        "base_major": update.base_version.major,
        "base_minor": update.base_version.minor,
        "objectives": update.objectives.as_deref().map(list),
        "milestones": update.milestones.as_deref().map(list),
        "assumptions": update.assumptions.as_deref().map(list),
        "risks": update.risks.as_deref().map(list),
        "acceptance_conditions": update.acceptance_conditions.as_deref().map(list),
    })
    .to_string()
}
