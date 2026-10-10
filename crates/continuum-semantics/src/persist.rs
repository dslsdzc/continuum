//! `intent` 表（迁移 **110**）与它的行级读写（设计 §5.1／§5.2）。
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
//! # 迁移 110 只建 `intent` 一张表
//!
//! 设计 §2.4 规定 110 含**七张表**（`intent` / `specification` / `contract` /
//! `contract_requirement` / `plan` / `decision` / `decision_result`）；**其余六张由用它们的
//! task 逐表追加进同一个迁移**（计划「全局约束」：`110` 的 SQL 在 Task 8–15 之间逐表追加，
//! Task 20 之后不得再改）。**本 task 只加 `intent` 这一张**。
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
//! `kind` / `status` 的字面量取自 [`crate::IntentKind::as_str`] / [`crate::IntentStatus::parse`]
//! （与类型同址）。**本文件不自建第二份编码。**

use continuum_persist::{Migration, PersistError, Tx, Value, value::kind_name};

use crate::error::IntentError;
use crate::ids::{ContractId, IntentId, PlanId};
use crate::intent::{
    ContractRef, GraphIdRef, Intent, IntentKind, IntentStatus, Priority, ProductIdRef,
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

/// 本 crate 在 `intent` 表上注册的迁移（设计 §2.4 的 110 段）。
///
/// **本 task 只建这一张表**；其余六张（`specification` / `contract` /
/// `contract_requirement` / `plan` / `decision` / `decision_result`）由 Task 10／11／13／15
/// 追加进**同一个** `p4_semantics_migrations()` 的返回向量（各自的迁移号也是 110，
/// 表不同）。
///
/// `docs/superpowers/specs/2026-10-06-p4-semantic-layer-design.md:846` 那句「其余六张由用它们的
/// task 追加进同一个迁移」的落点就是本函数——读它的人会看到返回向量只有一项，
/// 那是本 task 的实况，不是漏项。
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
        CREATE INDEX idx_intent_parent ON intent(parent_intent);",
    )]
}

/// 写一条 Intent。**不 `commit`**——事务由调用方持有并提交（同 P1 的 `apply_transition`
/// 形状；§318 要求状态更新与对应事件在同一事务内提交，故写函数不自己收口）。
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
/// **[`row_to_intent`] 里那两处 `as u32`**（`version` 与 `contract_version`）走的是
/// `continuum-graph/src/persist.rs:235` 的 `graph.version` 同一条处置：那两列的取值域
/// 由本层自己定（`u32`），写下去的就是 `i64::from(u32)`，故读回来的转换对所有本层写过的
/// 值都是精确的。**本函数本身不做转换**，它只把那一列读成 `i64`。
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
