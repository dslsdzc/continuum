//! §222 的 Intent 数据模型（十五个字段）与 §5.2 的版本语义，以及 `intent` 表（迁移 110）。
//!
//! 本文件的夹具 `db()` 单独成立：只依赖 `p4_semantics_migrations` 与 `Db`，
//! 建库后必须 `migrate()`——漏掉那一行时各用例会一起挂在 `no such table: intent`，
//! 而报错位置指向被测函数，容易误判成实现的问题（同 `continuum-canonical/tests/alias.rs`）。
//!
//! # 本文件钉的每一件，以及它的红条件
//!
//! | 用例 | 钉的判据 | 红条件（档位） |
//! |---|---|---|
//! | `intent_carries_exactly_the_fifteen_fields_of_222` | §222 的字段清单**逐项**（穷尽解构，不带 `..`） | 任一侧加／少一个字段即**编译不过**（**编译期照片，运行期无照片**——纯数据声明） |
//! | `a_child_list_is_not_itself_stored` | (a) `list_children` 按 id 升序返回两个孩子；(b) `PRAGMA table_info(intent)` 没有 `child_intents` 列 | 把子列表落成一个列（**移除档**——两份真值会漂移） |
//! | `version_increments_only_on_a_semantic_change` | (a) 改 `goal` ⇒ `version` +1 **且** `updated_at` 变；(b) 只做状态迁移 ⇒ `version` **不变**、`updated_at` 变（**两侧都钉**） | 状态迁移也 +1（**放宽档**，(b) 红）；语义变化不 +1（**移除档**，(a) 红） |
//! | `version_increments_for_each_of_the_five_semantic_fields` | §5.2 的五个语义字段**逐项各一条** +1，不抽代表 | 只对 `goal` +1（**收紧档**，其余四条红） |
//! | `version_does_not_increment_for_the_non_semantic_fields` | **非**语义字段**逐项**（`status` / `priority` / `active_plan_id` / `graph_id` / `product_id`）变化**不** +1，且 `created_at` 不被更新改写 | 把其中任一算进语义字段（**放宽档**——那一条红） |
//! | `history_does_not_accumulate_in_the_intent_table` | 两次语义改动后 `intent` 表**行数仍是 1**（一行一 Intent） | 改成「每次改动插一行」（**移除档**——行数变 3） |
//! | `contract_id_carries_a_version` | `Intent.contract_id` 是 `ContractRef { id, version }`（穷尽解构） | 少一字段即**编译不过** |
//! | `graph_id_and_product_id_are_opaque_strings` | 两枚不透明串各往返一次 ＋ 本 crate 的 `src/` 里执行层 crate 的下划线拼法零命中 | 取执行层的类型即多一条依赖边（由 `dependency_direction.rs` 的 `ALLOWED` 与 Task 21 的 grep 捕） |
//! | `the_intent_table_has_exactly_the_expected_columns` | `PRAGMA table_info(intent)` 实际列 vs 手写期望，**逐列** | 加／少／改一列即红 |
//! | `enum_columns_use_lowercase_underscore_and_reject_out_of_table_values` | `kind` 六枚、`status` 十三枚的落库字面量**逐枚**；表外取值读回得**具体** `Err` | 编码改成大写／去掉 `_`／表外取值取默认值即红 |
//! | `list_children_rejects_an_out_of_table_enum_too` | 第二条读路径的表外取值处置 | 同上（两条读路径**各有一条照片**） |
//! | `the_migration_registers_exactly_the_intent_table` | 迁移 110 只建 `intent` 一张表 | 多建／少建一张表即红 |

use std::fs;
use std::path::{Path, PathBuf};

use continuum_canonical::{EntityId, Opaque};
use continuum_persist::{Db, Migration, Tx, Value, builtin_migrations};
use continuum_semantics::{
    ContractId, ContractRef, GraphIdRef, Intent, IntentError, IntentId, IntentKind, IntentStatus,
    PlanId, Priority, ProductIdRef, list_children, load_intent, p4_semantics_migrations, save_intent,
};

/// 建一个只装了 P0 内建迁移 ＋ 本 crate 迁移的临时库。
fn db() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let mut migrations: Vec<Migration> = builtin_migrations();
    migrations.extend(p4_semantics_migrations());
    let db = Db::open_with(&path, migrations).unwrap();
    db.migrate().unwrap();
    (dir, db)
}

/// 一条基准 Intent：**每个字段都取一个非默认值**，好让「哪一列没落库／与邻列写串」
/// 这类错在读回时的逐字段比对里现形。
fn sample(id: &str) -> Intent {
    Intent {
        id: IntentId::from_str(id),
        version: 1,
        kind: IntentKind::Task,
        goal: EntityId::from_str("goal-a"),
        status: IntentStatus::Proposed,
        priority: Priority::from_str("high"),
        created_at: 1_000,
        updated_at: 1_000,
        parent_intent: None,
        contract_id: ContractRef {
            id: ContractId::from_str("c-1"),
            version: 1,
        },
        active_plan_id: None,
        graph_id: None,
        product_id: None,
        completion_predicate: Opaque::from_str("pred-a"),
    }
}

/// 直读一行的某一列**原始文本**，绕开全部公开读函数。
///
/// 这是编码纪律（小写、多词接 `_`）的唯一照片：经 `load_intent` 读回来的是已经过
/// `parse` 的值，`as_str` 与 `parse` 一起错（例如把 `task` 也编成 `"product"`）时
/// 它是看不出来的。`column` 只取本文件里写死的字面量，不是外部输入。
fn raw_text(tx: &Tx<'_>, id: &str, column: &str) -> String {
    let rows = tx
        .query(
            &format!("SELECT {column} FROM intent WHERE id = ?1"),
            &[Value::text(id)],
        )
        .unwrap();
    assert_eq!(rows.len(), 1, "intent 表里应有恰一行 {id:?}，实得 {} 行", rows.len());
    match &rows[0][0] {
        Value::Text(s) => s.clone(),
        other => panic!("{column} 列应是文本，实得 {other:?}"),
    }
}

/// `intent` 表的行数。
fn row_count(tx: &Tx<'_>) -> i64 {
    match &tx.query("SELECT COUNT(*) FROM intent", &[]).unwrap()[0][0] {
        Value::Int(i) => *i,
        other => panic!("COUNT(*) 应是整数，实得 {other:?}"),
    }
}

/// 运行期从库里读出的列清单：`(列名, 声明类型, NOT NULL, 主键位次)`。
///
/// 形态与 `continuum-canonical/tests/alias.rs` 的同名辅助函数同。
fn columns(tx: &Tx<'_>) -> Vec<(String, String, i64, i64)> {
    tx.query("PRAGMA table_info(intent)", &[])
        .unwrap()
        .iter()
        .map(|row| {
            let text = |at: usize| match &row[at] {
                Value::Text(s) => s.clone(),
                other => panic!("table_info 第 {at} 列应是文本，实得 {other:?}"),
            };
            let int = |at: usize| match &row[at] {
                Value::Int(i) => *i,
                other => panic!("table_info 第 {at} 列应是整数，实得 {other:?}"),
            };
            (text(1), text(2), int(3), int(5))
        })
        .collect()
}

/// §222 的**十五个字段**（`docs/spec/05-normative.md:132-155`），逐项列出——
/// 本节这张清单也就是那个用例的断言对象。
///
/// 规范原文的十五个（**十三行**，`created_at`/`updated_at` 与 `parent_intent?`/
/// `child_intents[]` 各一行两字段）：
///
/// 1. `id`
/// 2. `version`
/// 3. `kind`
/// 4. `goal`
/// 5. `status`
/// 6. `priority`
/// 7. `created_at`
/// 8. `updated_at`
/// 9. `parent_intent?`
/// 10. `child_intents[]`
/// 11. `contract_id`
/// 12. `active_plan_id?`
/// 13. `graph_id?`
/// 14. `product_id?`
/// 15. `completion_predicate`
///
/// [`Intent`] 有**十四个**成员：`child_intents[]`（第 10 项）**不是成员**——§5.1 的
/// 「子列表不落列」，它由 `parent_intent` 反查（[`list_children`]）。下面那条用例的
/// 穷尽解构绑的正是这十四个。
///
/// # 这一条是编译期照片，运行期无照片
///
/// 它是**纯数据声明**（一个结构体的字段清单），故运行期没有任何东西可断言——
/// 穷尽解构 `let Intent { … } = i;`（**不带 `..`**）是**唯一**能钉住「字段恰好这些」的
/// 机制：任一侧（结构体或这里）加一个字段就**编译不过**。
/// **按本仓纪律，编译不过不算「变红」**，故这里明写：本用例的运行期断言为空，
/// 它承重的是编译期那一层。（`let _ = (…)` 只是把绑定消费掉，免得 unused-variable 警告。）
#[test]
fn intent_carries_exactly_the_fifteen_fields_of_222() {
    let i = sample("i-1");
    let Intent {
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
    } = i;
    let _ = (
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
    );
}

/// §5.1 的「子列表不落列」，**两侧都钉**。
///
/// (a) 派生那一半：`list_children` 按 **id 升序**返回两个孩子（插入次序与 id 次序相反，
///     故「按 id 升序」这件事是被断言出来的，不是碰巧的）。
/// (b) 否定式照片：`PRAGMA table_info(intent)` 的实际列清单里**没有** `child_intents`。
///
/// **红条件**：把子列表落成一个列（**移除档**）——(b) 当场变红；而 (a) 仍绿
/// （派生函数还在，只是真值多了一份）。这正是「两份真值会漂移」那类缺陷的形状：
/// 只钉 (a) 会让多出来的那一列无人过问。
#[test]
fn a_child_list_is_not_itself_stored() {
    let (_d, db) = db();
    let tx = db.begin().unwrap();

    save_intent(&tx, &sample("i-parent")).unwrap();
    // **先写 id 大的那个**：若实现漏了 `ORDER BY id`（例如按插入序、即 rowid 序），
    // 下面那条升序断言会红。
    let mut second = sample("i-b");
    second.parent_intent = Some(IntentId::from_str("i-parent"));
    save_intent(&tx, &second).unwrap();
    let mut first = sample("i-a");
    first.parent_intent = Some(IntentId::from_str("i-parent"));
    save_intent(&tx, &first).unwrap();
    // 一个不相关的 Intent：它的 parent 是别人，不该出现在子列表里。
    let mut other = sample("i-z");
    other.parent_intent = Some(IntentId::from_str("i-other"));
    save_intent(&tx, &other).unwrap();

    let kids = list_children(&tx, &IntentId::from_str("i-parent")).unwrap();
    let ids: Vec<&str> = kids.iter().map(|k| k.id.as_str()).collect();
    assert_eq!(
        ids,
        vec!["i-a", "i-b"],
        "list_children 应按 id 升序返回恰两个孩子，实得 {ids:?}"
    );
    for kid in &kids {
        assert_eq!(
            kid.parent_intent.as_ref().map(IntentId::as_str),
            Some("i-parent"),
            "子列表里混进了 parent 不是 i-parent 的行"
        );
    }

    // (b) 否定式照片。
    let names: Vec<String> = columns(&tx).into_iter().map(|(n, _, _, _)| n).collect();
    assert!(
        !names.iter().any(|n| n == "child_intents"),
        "intent 表里不该有 child_intents 列（§5.1 的子列表不落列），实得列清单 {names:?}"
    );

    tx.commit().unwrap();
}

/// §5.2 的版本语义，**两侧都钉**。
///
/// (a) 改 `goal`（一个语义字段）⇒ `version` **+1** **且** `updated_at` 变。
/// (b) 只做状态迁移（`status` 是**非**语义字段）⇒ `version` **不变**、`updated_at` 变。
///
/// **红条件**：
/// - 状态迁移也 +1（**放宽档**）——(b) 红（版本变成 3）；
/// - 语义变化不 +1（**移除档**）——(a) 红（版本停在 1）；
/// - 语义变化不改 `updated_at`（**移除档**）——(a) 的 `updated_at` 断言红；
/// - 版本语义整个不做（照收入参 `version`）——(a) 红。
///
/// **两条缺一不可**：只钉 (a) 会让「任何写入都 +1」漂过去（那是把版本退化成事件计数，
/// 正是 §5.2 反对的）；只钉 (b) 会让「任何写入都不 +1」漂过去。
#[test]
fn version_increments_only_on_a_semantic_change() {
    let (_d, db) = db();
    let tx = db.begin().unwrap();

    save_intent(&tx, &sample("i-1")).unwrap();
    let v1 = load_intent(&tx, &IntentId::from_str("i-1")).unwrap().unwrap();
    assert_eq!(v1.version, 1, "首次写入应是 v1");
    assert_eq!(v1.updated_at, 1_000);

    // (a) 改一个语义字段（goal），并把 updated_at 挪到新的时刻。
    let mut semantic = sample("i-1");
    semantic.goal = EntityId::from_str("goal-b");
    semantic.updated_at = 2_000;
    save_intent(&tx, &semantic).unwrap();
    let v2 = load_intent(&tx, &IntentId::from_str("i-1")).unwrap().unwrap();
    assert_eq!(v2.version, 2, "(a) 改 goal 之后版本应 +1");
    assert_eq!(v2.goal.as_str(), "goal-b", "(a) 改动本身也要真的落库");
    assert_ne!(v2.updated_at, v1.updated_at, "(a) 语义变化要改 updated_at");
    assert_eq!(v2.updated_at, 2_000);

    // (b) 只做状态迁移：其余一切都与刚才那一版相同。
    let mut status_only = sample("i-1");
    status_only.goal = EntityId::from_str("goal-b");
    status_only.status = IntentStatus::Specifying;
    status_only.updated_at = 3_000;
    save_intent(&tx, &status_only).unwrap();
    let v3 = load_intent(&tx, &IntentId::from_str("i-1")).unwrap().unwrap();
    assert_eq!(
        v3.version, 2,
        "(b) 只做状态迁移不该动版本（版本变成 {} 说明状态被算进了语义字段）",
        v3.version
    );
    assert_eq!(v3.status, IntentStatus::Specifying, "(b) 迁移本身要真的落库");
    assert_eq!(v3.updated_at, 3_000, "(b) 状态迁移要改 updated_at");

    tx.commit().unwrap();
}

/// §5.2 的五个语义字段**逐项各一条** +1 断言，**不抽代表**。
///
/// 五个：`kind` / `goal` / `parent_intent` / `completion_predicate` / `contract_id`。
///
/// **红条件**：只对 `goal` +1（**收紧档**）——其余四条红（版本停在 1）。
/// 五条**各起一个库**、各写一条基准再改一个字段：这样「改哪一个字段」与「上一条留下的状态」
/// 不会互相纠缠（同一个 id 连改五次的话，一条实现漏判某字段会被别的字段的 +1 遮住）。
#[test]
fn version_increments_for_each_of_the_five_semantic_fields() {
    // 每一格：一个「改一个语义字段」的构造函数。
    let cases: [(&str, fn(&mut Intent)); 5] = [
        ("kind", |i| i.kind = IntentKind::Product),
        ("goal", |i| i.goal = EntityId::from_str("goal-b")),
        ("parent_intent", |i| {
            i.parent_intent = Some(IntentId::from_str("i-p"))
        }),
        ("completion_predicate", |i| {
            i.completion_predicate = Opaque::from_str("pred-b")
        }),
        ("contract_id", |i| i.contract_id.version = 2),
    ];

    for (field, mutate) in cases {
        let (_d, db) = db();
        let tx = db.begin().unwrap();

        save_intent(&tx, &sample("i-1")).unwrap();
        let mut changed = sample("i-1");
        mutate(&mut changed);
        changed.updated_at = 2_000;
        save_intent(&tx, &changed).unwrap();

        let back = load_intent(&tx, &IntentId::from_str("i-1")).unwrap().unwrap();
        assert_eq!(
            back.version, 2,
            "改 `{field}` 之后版本应 +1（停在 {} 说明它没被算进语义字段）",
            back.version
        );
        tx.commit().unwrap();
    }
}

/// §5.2 的**非**语义字段**逐项各一条**：改了它们，`version` **不** +1。
///
/// 五个：`status` / `priority` / `active_plan_id` / `graph_id` / `product_id`
/// （[`save_intent`] 的文档逐字列了这五个，并说它们「变**不** +1」——
/// 那句绝对措辞的用例就是本条）。另断言 `created_at` **不被一次更新改写**
/// （同一句文档承诺的另一半）。
///
/// **红条件**：把其中任一算进语义字段（**放宽档**）——那一条红（版本从 1 变 2）。
/// 五条**各起一个库**：否则「改哪一项」与「上一项留下的状态」会互相纠缠。
#[test]
fn version_does_not_increment_for_the_non_semantic_fields() {
    let cases: [(&str, fn(&mut Intent)); 5] = [
        ("status", |i| i.status = IntentStatus::Running),
        ("priority", |i| i.priority = Priority::from_str("low")),
        ("active_plan_id", |i| {
            i.active_plan_id = Some(PlanId::from_str("p-1"))
        }),
        ("graph_id", |i| i.graph_id = Some(GraphIdRef::from_str("g-1"))),
        ("product_id", |i| {
            i.product_id = Some(ProductIdRef::from_str("prod-1"))
        }),
    ];

    for (field, mutate) in cases {
        let (_d, db) = db();
        let tx = db.begin().unwrap();

        save_intent(&tx, &sample("i-1")).unwrap();
        let mut changed = sample("i-1");
        mutate(&mut changed);
        changed.updated_at = 2_000;
        save_intent(&tx, &changed).unwrap();

        let back = load_intent(&tx, &IntentId::from_str("i-1")).unwrap().unwrap();
        assert_eq!(
            back.version, 1,
            "改 `{field}` 之后版本不该 +1（变成 {} 说明它被算进了语义字段）",
            back.version
        );
        assert_eq!(back.updated_at, 2_000, "改 `{field}` 仍要改 updated_at");
        assert_eq!(
            back.created_at, 1_000,
            "改 `{field}` 不该改写 created_at（创建时刻不因一次更新而变）"
        );
        tx.commit().unwrap();
    }
}

/// 历史**不堆在** `intent` 表里（§5.2）：改两次语义之后，行数仍是 1。
///
/// **这与 Contract 的处置刻意不同**（§7.3：§226 明写 Contract MUST NOT 原地修改——
/// `contract` 表主键 `(id, version)`，每个新版本是一行）。两者的差别来自规范原文的差别：
/// §226 对 Contract 说了 MUST NOT 原地修改，§222／§223 对 Intent 说的是「一行一 Intent ＋
/// 状态机原地迁移」（§5.2：历史进 `events` 表与审计表，不进本表）。**不是不一致。**
///
/// **红条件**：改成「每次改动插一行」（**移除档**）——行数变 3，本用例红。
#[test]
fn history_does_not_accumulate_in_the_intent_table() {
    let (_d, db) = db();
    let tx = db.begin().unwrap();

    save_intent(&tx, &sample("i-1")).unwrap();
    assert_eq!(row_count(&tx), 1);

    let mut second = sample("i-1");
    second.goal = EntityId::from_str("goal-b");
    second.updated_at = 2_000;
    save_intent(&tx, &second).unwrap();

    let mut third = sample("i-1");
    third.goal = EntityId::from_str("goal-b");
    third.kind = IntentKind::Research;
    third.updated_at = 3_000;
    save_intent(&tx, &third).unwrap();

    assert_eq!(
        row_count(&tx),
        1,
        "两次语义改动之后 intent 表应仍是一行（一行一 Intent）；改动的权威在 events 表，不在本表"
    );
    let back = load_intent(&tx, &IntentId::from_str("i-1")).unwrap().unwrap();
    assert_eq!(back.version, 3, "两次语义改动应是 v1 → v2 → v3");
    assert_eq!(back.kind, IntentKind::Research);

    tx.commit().unwrap();
}

/// `Intent.contract_id` 是 [`ContractRef`] `{ id, version }`——**字段清单逐项**
/// （穷尽解构，不带 `..`）。
///
/// **§222 只给 `contract_id`，版本是本设计补的**（§7.3：`ContractDiff` 必须知道基准版本）。
/// 这一句也写在 [`ContractRef`] 的类型文档里。
///
/// **红条件**：给 `ContractRef` 加／少一个字段即**编译不过**（编译期照片）。
#[test]
fn contract_id_carries_a_version() {
    let mut i = sample("i-1");
    i.contract_id = ContractRef {
        id: ContractId::from_str("c-9"),
        version: 7,
    };

    let ContractRef { id, version } = i.contract_id;
    assert_eq!(id.as_str(), "c-9");
    assert_eq!(version, 7, "ContractRef 带的是基准版本，不是别的东西");
}

/// `GraphIdRef` / `ProductIdRef` 是**不透明串**（§1.2.3 的规则 3）。
///
/// 两条：两种类型各断言一次**表外取值往返**（一个本层没有词表的串进、同一个串出）；
/// 再断言本 crate 的 `src/` 里**执行层 crate 的下划线拼法零命中**——
/// 取 `continuum_graph::GraphId` 会写出一条指向执行层的 `Cargo.toml` 边，
/// 而那条边今天不存在（第二侧守卫是 Task 21 的 grep）。
///
/// **红条件**：把 `GraphIdRef` 换成执行层的 `GraphId`——源码扫描那条红
/// （`use continuum_graph::…` 里含那串拼法）。
#[test]
fn graph_id_and_product_id_are_opaque_strings() {
    // 「表外」= 本层没有任何词表能判它合法与否；这类串照样往返。
    let g = GraphIdRef::from_str("节点-图/不是本层的词表");
    assert_eq!(g.as_str(), "节点-图/不是本层的词表");

    let p = ProductIdRef::from_str("产品 id 也是不透明的");
    assert_eq!(p.as_str(), "产品 id 也是不透明的");

    // 源码扫描（射程：本 crate 的 `src/`，不含 `tests/`——本文件里出现的那个拼法
    // 是**被断言的语料**，不是使用时把 `tests/` 也算进来，本守卫就与它自己的目的地打架，
    // 同 `continuum-capability/tests/single_writer.rs` 的射程说明）。
    let sources = crate_sources();
    let joined: String = sources.iter().map(|(_, s)| s.as_str()).collect::<Vec<_>>().join("\n");
    // **先**证明扫描真的读到了这棵树（否则下面那条断言在空语料上恒真、恒绿）。
    assert!(
        sources.len() >= 4,
        "只扫到 {} 个 `src/` 下的 `.rs`（{:?}）：源码树没被读到",
        sources.len(),
        sources.iter().map(|(p, _)| p.display().to_string()).collect::<Vec<_>>()
    );
    assert!(
        joined.contains("pub struct Intent"),
        "读到了 {} 个文件，但拼起来找不到 `pub struct Intent`——读到的不是这棵树",
        sources.len()
    );
    assert!(
        !joined.contains("continuum_graph"),
        "本 crate 的 `src/` 里出现了执行层 crate 的下划线拼法——那会写出一条指向执行层的依赖边\
         （设计 §1.2.3 的规则 3）。`graph_id` / `product_id` 取不透明串，不取执行层的类型。"
    );
}

/// `intent` 表的列清单**逐列**：实际 vs 手写期望。
///
/// 手写期望与迁移 110 的 SQL 是**两份**（这正是本用例的判别力所在：改了一处、忘了另一处即红）。
/// **红条件**：加／少／改一列（含改 `NOT NULL`、改主键位次）即红。
#[test]
fn the_intent_table_has_exactly_the_expected_columns() {
    let (_d, db) = db();
    let tx = db.begin().unwrap();

    let expected: Vec<(&str, &str, i64, i64)> = vec![
        ("id", "TEXT", 1, 1),
        ("version", "INTEGER", 1, 0),
        ("kind", "TEXT", 1, 0),
        ("goal", "TEXT", 1, 0),
        ("status", "TEXT", 1, 0),
        ("priority", "TEXT", 1, 0),
        ("created_at", "INTEGER", 1, 0),
        ("updated_at", "INTEGER", 1, 0),
        ("parent_intent", "TEXT", 0, 0),
        ("contract_id", "TEXT", 1, 0),
        ("contract_version", "INTEGER", 1, 0),
        ("active_plan_id", "TEXT", 0, 0),
        ("graph_id", "TEXT", 0, 0),
        ("product_id", "TEXT", 0, 0),
        ("completion_predicate", "TEXT", 1, 0),
    ];
    let expected: Vec<(String, String, i64, i64)> = expected
        .into_iter()
        .map(|(n, t, nn, pk)| (n.to_owned(), t.to_owned(), nn, pk))
        .collect();

    assert_eq!(
        columns(&tx),
        expected,
        "intent 表的列清单与手写期望不符（列名／类型／NOT NULL／主键位次任一不同即红）"
    );

    tx.commit().unwrap();
}

/// 枚举列的**编码纪律**（小写、多词接 `_`）**逐枚**读库，以及表外取值读回得**具体** `Err`。
///
/// `kind` 六枚、`status` 十三枚**各写一次、读回原始列文本**——只断言「往返一致」是不够的：
/// `as_str` 与 `parse` 互为逆函数时，一张把两枚都编成同一个串的表照样往返得通。
/// 故这里断言的是**落库的那一个串本身**（同 `continuum-canonical/tests/alias.rs` 的
/// `every_alias_scope_round_trips` 那条的理由）。
///
/// 表外取值那一条断言的是**具体哪一枚错误**（不是「返回了 Err」），且带库里那一个原始串。
#[test]
fn enum_columns_use_lowercase_underscore_and_reject_out_of_table_values() {
    let (_d, db) = db();
    let tx = db.begin().unwrap();

    let kinds = [
        (IntentKind::Task, "task"),
        (IntentKind::Product, "product"),
        (IntentKind::Maintenance, "maintenance"),
        (IntentKind::Research, "research"),
        (IntentKind::Background, "background"),
        (IntentKind::RuntimeImprovement, "runtime_improvement"),
    ];
    for (kind, literal) in kinds {
        let mut i = sample("i-kind");
        i.kind = kind;
        save_intent(&tx, &i).unwrap();
        assert_eq!(
            raw_text(&tx, "i-kind", "kind"),
            literal,
            "kind 的落库编码应为小写、多词接 `_`"
        );
        assert_eq!(
            load_intent(&tx, &IntentId::from_str("i-kind")).unwrap().unwrap().kind,
            kind,
            "kind 应往返"
        );
    }

    let statuses = [
        (IntentStatus::Proposed, "proposed"),
        (IntentStatus::Specifying, "specifying"),
        (IntentStatus::Planning, "planning"),
        (IntentStatus::Review, "review"),
        (IntentStatus::Ready, "ready"),
        (IntentStatus::Running, "running"),
        (IntentStatus::Waiting, "waiting"),
        (IntentStatus::Suspended, "suspended"),
        (IntentStatus::Verifying, "verifying"),
        (IntentStatus::Completed, "completed"),
        (IntentStatus::Failed, "failed"),
        (IntentStatus::Cancelled, "cancelled"),
        (IntentStatus::Evolving, "evolving"),
    ];
    for (status, literal) in statuses {
        let mut i = sample("i-status");
        i.status = status;
        save_intent(&tx, &i).unwrap();
        assert_eq!(
            raw_text(&tx, "i-status", "status"),
            literal,
            "status 的落库编码应为小写、多词接 `_`"
        );
        assert_eq!(
            load_intent(&tx, &IntentId::from_str("i-status")).unwrap().unwrap().status,
            status,
            "status 应往返"
        );
    }

    // 表外取值：绕开 `save_intent` 直接把一个本层认不出的串写进去（模拟历史行／被外部改过的行）。
    save_intent(&tx, &sample("i-bad-kind")).unwrap();
    tx.execute(
        "UPDATE intent SET kind = ?1 WHERE id = ?2",
        &[Value::text("nonsense"), Value::text("i-bad-kind")],
    )
    .unwrap();
    assert_eq!(
        load_intent(&tx, &IntentId::from_str("i-bad-kind")).unwrap_err(),
        IntentError::UnknownKind {
            value: "nonsense".to_owned()
        },
        "kind 的表外取值应读回**具体**错误、并带库里那一个原始串（不取默认值）"
    );

    save_intent(&tx, &sample("i-bad-status")).unwrap();
    tx.execute(
        "UPDATE intent SET status = ?1 WHERE id = ?2",
        &[Value::text("nonsense"), Value::text("i-bad-status")],
    )
    .unwrap();
    assert_eq!(
        load_intent(&tx, &IntentId::from_str("i-bad-status")).unwrap_err(),
        IntentError::UnknownStatus {
            value: "nonsense".to_owned()
        },
        "status 的表外取值应读回**具体**错误、并带库里那一个原始串（不取默认值）"
    );

    tx.commit().unwrap();
}

/// **第二条读路径**的表外取值处置（[`list_children`]）。
///
/// 与 [`load_intent`] 共用解码函数是**实现的选择，不是判据**——两条路径各有一条照片，
/// 免得一条坏了而另一条遮住它。
#[test]
fn list_children_rejects_an_out_of_table_enum_too() {
    let (_d, db) = db();
    let tx = db.begin().unwrap();

    save_intent(&tx, &sample("i-parent")).unwrap();
    let mut child = sample("i-child");
    child.parent_intent = Some(IntentId::from_str("i-parent"));
    save_intent(&tx, &child).unwrap();

    tx.execute(
        "UPDATE intent SET status = ?1 WHERE id = ?2",
        &[Value::text("nonsense"), Value::text("i-child")],
    )
    .unwrap();

    assert_eq!(
        list_children(&tx, &IntentId::from_str("i-parent")).unwrap_err(),
        IntentError::UnknownStatus {
            value: "nonsense".to_owned()
        }
    );

    tx.commit().unwrap();
}

/// 迁移 110 **只建 `intent` 一张表**（本 task 的实况）。
///
/// 设计 §2.4 规定 110 含七张表；**其余六张由用它们的 task 逐表追加进同一个
/// `p4_semantics_migrations()`**（Task 10／11／13／15）。故**本用例的这条计数断言会随那些
/// task 递增**——它是「本 task 只加一张」的照片，不是一条永久约束。
///
/// **红条件**：本 task 顺手多建一张表（**移除档**——它没有使用者，也就没有别的用例会红）。
#[test]
fn the_migration_registers_exactly_the_intent_table() {
    let migrations = p4_semantics_migrations();
    assert_eq!(
        migrations.len(),
        1,
        "本 task 只建 `intent` 一张表；其余六张由 Task 10／11／13／15 追加（届时本断言要跟着改）"
    );
    assert_eq!(migrations[0].version, 110);
    assert_eq!(migrations[0].name, "p4_intent");
    assert!(
        migrations[0].sql.contains("CREATE TABLE intent"),
        "迁移 110 应建 `intent` 表"
    );
    // 一张表，一条 CREATE TABLE（多一条就是本 task 多建了一张表）。
    assert_eq!(
        migrations[0].sql.matches("CREATE TABLE").count(),
        1,
        "迁移 110 的 SQL 里应恰有一条 `CREATE TABLE`"
    );
}

/// 本 crate `src/` 下的全部 `.rs`（路径排序，消息里的次序因此确定）。
///
/// **按 `CARGO_MANIFEST_DIR` 定，不按进程 CWD**：按相对路径扫的守卫在 crate 根下跑是绿的，
/// 换个目录跑就**读不到东西**——而「什么都没读到」正是上面那条正控制要逮的形状
/// （同 `continuum-capability/tests/single_writer.rs` 的 `repo_root`）。
fn crate_sources() -> Vec<(PathBuf, String)> {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    collect_rs(&src, &mut files);
    files.sort();
    files
        .into_iter()
        .map(|path| {
            let source = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("读不到源文件 {}：{e}", path.display()));
            (path, source)
        })
        .collect()
}

/// 递归收集 `dir` 下全部 `.rs`。**读不到的目录直接炸，不跳过**——静默跳过只会让守卫变窄，
/// 而变窄之后它仍然是绿的。
fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("列不出目录 {}：{e}", dir.display()))
    {
        let path = entry.unwrap_or_else(|e| panic!("列目录 {} 时出错：{e}", dir.display())).path();
        if path.is_dir() {
            collect_rs(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// `PlanId` 的两个方法可用（本 task 只登记这个类型，`Plan` 本体归 Task 13）——
/// 没有这一条，上面那些以 `PlanId` 为类型的字段（`active_plan_id`）就只钉了「类型存在」。
#[test]
fn plan_id_round_trips() {
    let p = PlanId::from_str("plan-1");
    assert_eq!(p.as_str(), "plan-1");
}
