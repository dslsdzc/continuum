//! `intent` 与 `specification` 两张表（迁移 **110**）与它们的行级读写
//! （设计 §5.1／§5.2／§5.4／§6.1／§6.2）。
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
//! # 迁移 110：`intent` 与 `specification` 两张表（逐表追加）
//!
//! 设计 §2.4 规定 110 含**七张表**（`intent` / `specification` / `contract` /
//! `contract_requirement` / `plan` / `decision` / `decision_result`）；**其余五张由用它们的
//! task 逐表追加**（计划「全局约束」：`110` 的 SQL 在 Task 8–15 之间逐表追加，
//! Task 20 之后不得再改）。**「追加」的落点是同一个迁移项的 SQL 字符串**——
//! 见 [`p4_semantics_migrations`] 的文档（第二个 110 号迁移项会被 `migrate()`
//! 静默跳过）。Task 8 加 `intent`；**Task 10 加 `specification`**。
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
//! （与类型同址）；解码侧的白名单字面量照 `parse` 先例（输入是开放集，逐枚匹配带通配臂）。
//! `unknowns_kind` / `unknown` 的 kind 两处同理（[`crate::Unknowns::kind_str`] /
//! [`crate::Unknown::kind_str`] 与 `decode_unknowns` 的匹配）。
//! **两列的白名单各写一遍是刻意的**（同 [`crate::IntentKind::parse`] 的理由）：
//! 两条路径各自有照片，`tests/spec.rs` 的
//! `the_unknowns_columns_use_lowercase_underscore_and_reject_out_of_table_values` 逐枚读库钉。

use continuum_persist::{Event, EventType, Migration, PersistError, Tx, Value, value::kind_name};

use crate::error::{IntentError, SpecError};
use crate::ids::{ContractId, IntentId, PlanId};
use crate::intent::{
    ContractRef, GraphIdRef, Intent, IntentKind, IntentStatus, Priority, ProductIdRef,
};
use crate::spec::{
    AcceptancePredicate, Conflict, ForbiddenState, Goal, Invariant, Preference, SpecDraft,
    Specification, UndefinedBehavior, Unknown, Unknowns,
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

/// 本 crate 在 `intent` 与 `specification` 两张表上注册的迁移（设计 §2.4 的 110 段）。
///
/// # 110 是**一个迁移项**，表**逐表追加进它的 SQL**
///
/// **「追加」的落点是同一个迁移项的 SQL 字符串，不是往返回向量里加第二个 version = 110
/// 的项**——`Db::migrate()` 以 `schema_migrations.version`（**主键**）判「已应用」
/// （`crates/continuum-persist/src/db.rs:103-112`），同号的第二个迁移项会被**静默跳过**
/// （第二个表永不建）。照片是 `tests/intent.rs` 的
/// `the_migration_registers_the_intent_and_specification_tables`：它断言
/// **`len() == 1`**，那条断言正是这件事的守卫。
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
/// - **表外取值**（闭集的白名单不认）：`unknowns_kind` 不是两枚之一 →
///   [`SpecError::UnknownsKind`]；元素 `kind` 不是两枚之一 → [`SpecError::UnknownItemKind`]。
///   两处都**带库里那一个原始串、不取默认值**（枚举列纪律）。
/// - **容器坏**：`unknowns` 不是 JSON 数组、或元素不是 `{kind, value}` 对象 →
///   [`SpecError::Persist`] 的 `Database` 形状（同 [`decode_members`]：「容器坏」
///   与「表外取值」是两类失败）。
fn decode_unknowns(kind: &str, raw: &str) -> Result<Unknowns, SpecError> {
    match kind {
        // `NoneFound` 时 `raw` 不参与解码：`unknowns_kind` 列是权威，内容列按约定是 `[]`。
        "none_found" => Ok(Unknowns::NoneFound),
        "enumerated" => {
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
                out.push(match item_kind {
                    "conflict" => Unknown::Conflict(Conflict::from_str(value)),
                    "undefined_behavior" => {
                        Unknown::UndefinedBehavior(UndefinedBehavior::from_str(value))
                    }
                    other => return Err(SpecError::UnknownItemKind { value: other.to_owned() }),
                });
            }
            Ok(Unknowns::Enumerated(out))
        }
        other => Err(SpecError::UnknownsKind {
            value: other.to_owned(),
        }),
    }
}
