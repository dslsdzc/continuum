//! §227 的六元组与 `unknowns` 的强制形态（设计 §6.1／§6.2）的用例集。
//!
//! # 本文件钉的每一件，以及它的红条件
//!
//! | 用例 | 钉的判据 | 红条件（档位） |
//! |---|---|---|
//! | `unknowns_has_exactly_two_variants` | `Unknowns` 恰两枚（`Enumerated` / `NoneFound`）——机制 = 穷尽 `match` **不带 `..`**（加第三枚即**编译不过**） | 加第三枚即**编译不过**（**编译期照片，运行期无照片**——同 `tests/intent.rs` 的字段清单用例） |
//! | `none_found_and_an_empty_enumerated_are_distinguishable_after_a_round_trip` | **本 task 的要害**：`NoneFound` 与 `Enumerated(vec![])` 各写一次、读回，**各自那一枚** | 把两者编成同一个落库表示（**移除档**）——`vec![]` 同时表示「已查过、没有」与「没查」，正是 §227 禁的「把未知项自动转换成假设」最容易发生的位置 |
//! | `a_specification_has_exactly_the_six_tuple_fields` | §227 六元组的字段清单**逐项**：运行期经六个访问器；编译期是 `src/spec.rs` 的穷尽解构与 `tests/compile_fail/a_specification_cannot_be_constructed_outside_the_crate.rs` | 访问器与字段错位即红；字段加／少一个，那两处编译期照片即**编译不过** |
//! | `an_intent_version_and_its_specification_are_written_in_one_transaction` | §6.2：(a) 写 Intent v3 与 Specification 同事务、两表都读到、版本同号；(b) 失败路径**两张表都没有半写的行** | 拆成两个事务／spec 失败被吞／回滚后改动残留（**移除档**与**放宽档**，(b) 的原值断言红） |
//! | `conflict_and_undefined_behaviour_are_types_only` | §6.1 的「只保存、不检测」：两枚成员写进去、读回来，内容原样（**否定式照片**在 `tests/compile_fail/conflict_and_undefined_behaviour_are_types_only.rs` 与 Task 21 的 grep） | 造一个检测器即发明判据（§16 第 10 条）；把两枚合并成一枚即红（两支逐项断言） |
//! | `the_unknowns_columns_use_lowercase_underscore_and_reject_out_of_table_values` | `unknowns_kind` 与 `Unknown` 的 kind 的落库字面量**逐枚**（raw 读列）；表外取值读回得**具体** `Err`；容器坏得 `Persist(Database)` | 编码改成大写／去掉 `_`／表外取值取默认值即红 |
//!
//! # 夹具的来历
//!
//! `new_db()` 与 `tests/intent.rs` 的 `db()` 夹具同形（P0 内建迁移 ＋ 本 crate 迁移 110，
//! 建库后必须 `migrate()`）。
//! `sample_intent` / `sample_draft` 都是**每个字段／每一项取一个非默认值**的基准值：
//! 「哪一列没落库／与邻列写串」这类错在读回时的逐项比对里现形。
//!
//! `save_intent` 在本文件里被用作「造起点版本」的后门（同 `tests/intent_state.rs` 的
//! 用法）；Specification 的落库路径只有一条：`save_intent_with_specification`。

use continuum_canonical::{EntityId, Opaque};
use continuum_persist::{Db, Migration, PersistError, Tx, Value, builtin_migrations};
use continuum_semantics::{
    AcceptancePredicate, Conflict, ContractId, ContractRef, ForbiddenState, Goal, Intent, IntentId,
    IntentKind, IntentStatus, Invariant, Preference, Priority, SpecDraft, SpecError, Specification,
    UndefinedBehavior, Unknown, Unknowns, load_intent, load_specification, p4_semantics_migrations,
    save_intent, save_intent_with_specification,
};

/// 建一个只装了 P0 内建迁移 ＋ 本 crate 迁移的临时库（与 `tests/intent.rs` 的夹具同形）。
fn new_db() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let mut migrations: Vec<Migration> = builtin_migrations();
    migrations.extend(p4_semantics_migrations());
    let db = Db::open_with(&path, migrations).unwrap();
    db.migrate().unwrap();
    (dir, db)
}

/// 一条基准 Intent：每个字段都取一个非默认值（同 `tests/intent.rs` 的 `sample`）。
///
/// `goal` 与 `updated_at` 参数化：本文件的用例靠改 `goal` 让 `save_intent` 算出新版本
/// （§5.2 的语义字段之一），靠 `updated_at` 区分两次写入。
fn sample_intent(id: &str, goal: &str, updated_at: i64) -> Intent {
    Intent {
        id: IntentId::from_str(id),
        version: 1, // 写侧忽略（`save_intent` 自己算），读侧才权威。
        kind: IntentKind::Task,
        goal: EntityId::from_str(goal),
        status: IntentStatus::Proposed,
        priority: Priority::from_str("high"),
        created_at: 1_000,
        updated_at,
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

/// 一份基准草案：五类产物都非空（`goals` 两项，断言顺序与内容都钉）。
///
/// **没有 `unknowns` 字段**——§227 的两句 MUST 的落点：`unknowns` 只在 `finalize`
/// 的入参上出现（`a_draft_without_unknowns_cannot_be_finalized` 那份样例钉签名）。
fn sample_draft() -> SpecDraft {
    SpecDraft {
        goals: vec![Goal::from_str("g-1"), Goal::from_str("g-2")],
        invariants: vec![Invariant::from_str("inv-1")],
        acceptance_predicates: vec![AcceptancePredicate::from_str("ap-1")],
        forbidden_states: vec![ForbiddenState::from_str("fs-1")],
        preferences: vec![Preference::from_str("pref-1")],
    }
}

/// 读回一条 Specification；不存在直接炸（用例里它必须存在）。
fn saved_spec(tx: &Tx<'_>, id: &str, version: u32) -> Specification {
    load_specification(tx, &IntentId::from_str(id), version)
        .unwrap()
        .unwrap_or_else(|| panic!("库里应有 {id:?} 的 v{version} Specification"))
}

/// `unknowns` 的**哪一枚**，摊成一个可比较的字符串。
///
/// 机制 = 穷尽 `match` **不带 `..`**：给 `Unknowns` 加第三枚即**编译不过**
/// （[`unknowns_has_exactly_two_variants`] 的编译期照片）。
/// 返回串里带上条目数，是因为「读回的是 `Enumerated` 且**空**」与「非空的
/// `Enumerated`」在 `none_found_…_round_trip` 的断言里要分开。
fn kind_of(u: &Unknowns) -> String {
    match u {
        Unknowns::Enumerated(items) => format!("enumerated/{}", items.len()),
        Unknowns::NoneFound => "none_found".to_owned(),
    }
}

/// `specification` 表某一行的某一列，**原始文本**（绕开全部公开读函数）。
///
/// 这是编码纪律（小写、多词接 `_`）的唯一照片：经 `load_specification` 读回来的是已经过
/// 解码的值，`编码` 与 `解码` 一起错（例如把 `enumerated` 也编成 `none_found`）时它是
/// 看不出来的。`column` 只取本文件里写死的字面量，不是外部输入。
fn raw_text(tx: &Tx<'_>, id: &str, version: u32, column: &str) -> String {
    let rows = tx
        .query(
            &format!("SELECT {column} FROM specification WHERE intent_id = ?1 AND version = ?2"),
            &[Value::text(id), Value::Int(i64::from(version))],
        )
        .unwrap();
    assert_eq!(
        rows.len(),
        1,
        "specification 表里应有恰一行 ({id:?}, v{version})，实得 {} 行",
        rows.len()
    );
    match &rows[0][0] {
        Value::Text(s) => s.clone(),
        other => panic!("{column} 列应是文本，实得 {other:?}"),
    }
}

/// `specification` 表的行数。
fn spec_row_count(tx: &Tx<'_>) -> i64 {
    match &tx.query("SELECT COUNT(*) FROM specification", &[]).unwrap()[0][0] {
        Value::Int(i) => *i,
        other => panic!("COUNT(*) 应是整数，实得 {other:?}"),
    }
}

/// `Unknowns` **恰两枚**（`Enumerated` / `NoneFound`）——**编译期照片，运行期无照片**。
///
/// 机制 = [`kind_of`] 的穷尽 `match` **不带 `..`**：给 `Unknowns` 加第三枚即**编译不过**。
/// 按本仓纪律，编译不过不算「变红」，故这里明写：本用例的运行期断言**只能**证明
/// 「这两枚我没漏」，证明不了「没有第三枚」——那一半承重在编译期
/// （同 `tests/intent.rs` 的 `intent_carries_exactly_the_fifteen_fields_of_222`）。
///
/// **红条件**：加第三枚即编译不过（`kind_of` 的 `match` 与 `src/spec.rs` 的
/// `as_str` 两处都会红）。
#[test]
fn unknowns_has_exactly_two_variants() {
    assert_eq!(kind_of(&Unknowns::NoneFound), "none_found");
    assert_eq!(kind_of(&Unknowns::Enumerated(vec![])), "enumerated/0");
}

/// **本 task 的要害**（§227 两句 MUST）：`NoneFound`（「已查过、没有」）与
/// `Enumerated(vec![])`（「显式枚举出的未知项——恰为空」）**各写一次、读回**，
/// 断言读回的是**各自那一枚**。
///
/// **红条件**：把两者编成同一个落库表示（**移除档**）——例如去掉 `unknowns_kind` 列、
/// 只留一个列表列：两行读回后都成 `Enumerated(vec![])`（或都成 `NoneFound`），
/// 至少一条断言红。这正是 §227 禁的「把未知项自动转换成假设」最容易发生的位置：
/// `vec![]` 同时表示「已查过、没有」与「没查」，而 §227 要的是后者**不可表达**
/// （`SpecDraft` 没有 `unknowns` 字段、`finalize` 要求显式给出 `Unknowns` 之一）。
#[test]
fn none_found_and_an_empty_enumerated_are_distinguishable_after_a_round_trip() {
    let (_d, db) = new_db();

    // v1：已查过、没有未知项。
    let tx = db.begin().unwrap();
    save_intent_with_specification(
        &tx,
        &sample_intent("i-1", "goal-a", 1_000),
        sample_draft(),
        Unknowns::NoneFound,
    )
    .unwrap();
    tx.commit().unwrap();

    // v2：显式枚举出的未知项——**空列表**。它与 v1 的差别**只在 `unknowns_kind` 列上**，
    // 两个 `unknowns` 内容列都是空列表。
    let tx = db.begin().unwrap();
    save_intent_with_specification(
        &tx,
        &sample_intent("i-1", "goal-b", 2_000),
        sample_draft(),
        Unknowns::Enumerated(vec![]),
    )
    .unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let v1 = saved_spec(&tx, "i-1", 1);
    let v2 = saved_spec(&tx, "i-1", 2);
    assert_eq!(
        kind_of(v1.unknowns()),
        "none_found",
        "v1 写的是 `NoneFound`，读回的应是那一枚"
    );
    assert_eq!(
        kind_of(v2.unknowns()),
        "enumerated/0",
        "v2 写的是 `Enumerated(vec![])`，读回的应是那一枚——**不是** `NoneFound`：\
         「已查过、枚举为空」与「没查」必须分得开（§227 的两句 MUST）"
    );
    tx.commit().unwrap();
}

/// §227 六元组的字段清单**逐项**。
///
/// 机制有三层，各钉一半：
/// 1. **运行期（本用例）**：经六个访问器逐项读回写库前的内容（顺序与内容都断）；
/// 2. **编译期（crate 内）**：`src/spec.rs` 的
///    `the_six_tuple_fields_are_exhaustively_destructured` 对 `Specification`
///    **穷尽解构、不带 `..`**——六个字段私有（唯一构造入口是 `finalize`），
///    故那张照片只有 crate 内做得到（本仓已有「照片住在 `src` 的 `#[cfg(test)]` 里」的
///    体例，见 `continuum-canonical/src/resolver.rs`）；
/// 3. **编译期（crate 外）**：`tests/compile_fail/a_specification_cannot_be_constructed_outside_the_crate.rs`
///    写全六个字段的字面量仍编不过（E0451 字段私有，`.stderr` 实跑取值）。
///
/// **红条件**：任一侧加／少／改一个字段，第 2、3 层的照片即**编译不过**；访问器与字段
/// 错位（把 `invariants()` 接到 `acceptance_predicates` 上）则本用例的内容断言红。
#[test]
fn a_specification_has_exactly_the_six_tuple_fields() {
    let (_d, db) = new_db();
    let tx = db.begin().unwrap();

    save_intent_with_specification(
        &tx,
        &sample_intent("i-1", "goal-a", 1_000),
        sample_draft(),
        Unknowns::Enumerated(vec![Unknown::Conflict(Conflict::from_str("冲突甲"))]),
    )
    .unwrap();

    let spec = saved_spec(&tx, "i-1", 1);

    // 五个列表逐项：顺序与内容都断（`sample_draft` 的 `goals` 是两项）。
    let goals: Vec<&str> = spec.goals().iter().map(Goal::as_str).collect();
    assert_eq!(goals, vec!["g-1", "g-2"], "goals 应逐项、按写入顺序读回");
    let invariants: Vec<&str> = spec.invariants().iter().map(Invariant::as_str).collect();
    assert_eq!(invariants, vec!["inv-1"]);
    let predicates: Vec<&str> = spec
        .acceptance_predicates()
        .iter()
        .map(AcceptancePredicate::as_str)
        .collect();
    assert_eq!(predicates, vec!["ap-1"]);
    let forbidden: Vec<&str> = spec
        .forbidden_states()
        .iter()
        .map(ForbiddenState::as_str)
        .collect();
    assert_eq!(forbidden, vec!["fs-1"]);
    let preferences: Vec<&str> = spec.preferences().iter().map(Preference::as_str).collect();
    assert_eq!(preferences, vec!["pref-1"]);

    // 第六项：`unknowns` 那一枚。
    match spec.unknowns() {
        Unknowns::NoneFound => panic!("写的是 `Enumerated`，读回不该是 `NoneFound`"),
        Unknowns::Enumerated(items) => {
            assert_eq!(items.len(), 1, "写进去一枚，读回应恰一枚");
            match &items[0] {
                Unknown::Conflict(c) => assert_eq!(c.as_str(), "冲突甲"),
                Unknown::UndefinedBehavior(_) => panic!("写的是 `Conflict`，读回不该是别的支"),
            }
        }
    }

    tx.commit().unwrap();
}

/// §6.2：Intent 的一个版本与它的 Specification **同事务写**——(a) 正常路径、(b) 失败路径。
///
/// (a) 正常路径：写 Intent v3 与 Specification 同事务，两张表都读到；
///     specification 行的 `version` 列与 intent 行的 `version` 列**同号**（原始列读回），
///     且 v1／v2 没有 spec 行（spec 行只在组合写时产生）。
/// (b) 失败路径 + 半写副作用：第二次组合写令 Specification 的主键冲突
///     （同一 `(intent_id, version)` 的二次写）⇒ 路径整体 `Err`；
///     **回滚后两张表都没有半写的行**——intent 行是失败前的值（`updated_at` 不是这次
///     尝试给的那一个）、specification 表恰是原来那一行（内容不是这次尝试给的草案），
///     **不是只断言 `Err` 是哪一枚**。
///
/// 红条件里「把两次写拆成两个事务」这一档的**结构性守卫**：本层的写函数只拿
/// `&Tx`、拿不到 `Db`，而 `Tx::commit` 消费 `self`（同 `tests/intent_state.rs` 的论证）
/// ——故「组合函数自己提交一半」写不出来；能发生的是 (i) spec 失败被吞成 `Ok`
/// （（b）的 Err 断言红）、(ii) 回滚后 intent 的改动残留（`updated_at` 原值断言红）。
#[test]
fn an_intent_version_and_its_specification_are_written_in_one_transaction() {
    let (_d, db) = new_db();

    // 起点 v1 与 v2 都经 `save_intent` 写（它们**没有** Specification）。
    let tx = db.begin().unwrap();
    save_intent(&tx, &sample_intent("i-1", "goal-a", 1_000)).unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    save_intent(&tx, &sample_intent("i-1", "goal-b", 2_000)).unwrap();
    tx.commit().unwrap();

    // (a) 语义再变一次 ⇒ v3；v3 与它的 Specification 同事务写。
    let tx = db.begin().unwrap();
    save_intent_with_specification(
        &tx,
        &sample_intent("i-1", "goal-c", 3_000),
        sample_draft(),
        Unknowns::NoneFound,
    )
    .unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let v3 = load_intent(&tx, &IntentId::from_str("i-1")).unwrap().unwrap();
    assert_eq!(v3.version, 3, "(a) 语义变化的第三次写入应是 v3");
    assert!(
        load_specification(&tx, &IntentId::from_str("i-1"), 3)
            .unwrap()
            .is_some(),
        "(a) v3 的 Specification 应读得到"
    );
    assert!(
        load_specification(&tx, &IntentId::from_str("i-1"), 2)
            .unwrap()
            .is_none(),
        "(a) v2 没写过 Specification，应读回 None（spec 行按需产生，不是每种 intent 版本自动有）"
    );
    // 版本同号：specification 行的 `version` 列（原始读回）恰是 intent 的当前版本。
    let spec_version: i64 = match &tx
        .query(
            "SELECT version FROM specification WHERE intent_id = ?1",
            &[Value::text("i-1")],
        )
        .unwrap()[0][0]
    {
        Value::Int(i) => *i,
        other => panic!("version 列应是整数，实得 {other:?}"),
    };
    assert_eq!(
        spec_version,
        i64::from(v3.version),
        "(a) Specification 的版本与 Intent 的版本应同号（§6.2）"
    );
    tx.commit().unwrap();

    // (b) 失败路径：再写一次 v3 的 Specification ⇒ 主键冲突 ⇒ 整体 `Err`。
    let tx = db.begin().unwrap();
    let err = save_intent_with_specification(
        &tx,
        &sample_intent("i-1", "goal-c", 9_000),
        SpecDraft {
            goals: vec![Goal::from_str("SHOULD-NOT-LAND")],
            invariants: Vec::new(),
            acceptance_predicates: Vec::new(),
            forbidden_states: Vec::new(),
            preferences: Vec::new(),
        },
        Unknowns::NoneFound,
    )
    .unwrap_err();
    assert!(
        matches!(&err, SpecError::Persist(PersistError::Database(m)) if m.contains("specification")),
        "(b) 期望具体 Err 是「specification 主键冲突」的 Persist，实得 {err:?}"
    );
    // 调用方在 `Err` 后回滚（丢弃 Tx）——「同事务」的落点在调用方这一步。
    drop(tx);

    let tx = db.begin().unwrap();
    let after = load_intent(&tx, &IntentId::from_str("i-1")).unwrap().unwrap();
    assert_eq!(after.version, 3, "(b) 回滚后版本应仍是 3");
    assert_eq!(
        after.updated_at, 3_000,
        "(b) 回滚后 `updated_at` 应是失败前的值（不是这次尝试给的 9_000——那不是「没有半写的行」）"
    );
    assert_eq!(
        spec_row_count(&tx),
        1,
        "(b) specification 表应恰一行（原来那一行；不是「这次写入留下了第二行」）"
    );
    let back = saved_spec(&tx, "i-1", 3);
    let goals: Vec<&str> = back.goals().iter().map(Goal::as_str).collect();
    assert_eq!(
        goals,
        vec!["g-1", "g-2"],
        "(b) spec 行的内容应是原来那一份（不是这次尝试给的 SHOULD-NOT-LAND）"
    );
    tx.commit().unwrap();
}

/// §6.1 的**「只保存、不检测」**：`Conflict` / `UndefinedBehavior` 作为
/// `Unknowns::Enumerated` 的成员**写进去、读回来**，两枚逐项、内容原样。
///
/// # 否定式照片（「不建检测器」）住在别处两处
///
/// 1. `tests/compile_fail/conflict_and_undefined_behaviour_are_types_only.rs` ＋
///    `tests/type_level.rs` 的驱动：`detect_conflicts` / `find_undefined_behaviour`
///    这两个名字在 crate 的公开面上**取不到**（`.stderr` 实跑取值）；
/// 2. Task 21 的 grep 复核 `src/` 里没有这两个函数。
///
/// **为什么这不是漏项**：§214 的「什么算冲突」规范没给判据（设计 §16 第 10 条），
/// 故本层只把两枚作为成员**保存**，不发明检测算法——造一个检测器就是发明判据。
///
/// **红条件**：把两枚合并成一枚即红（本用例逐支断言）；把 `Unknown` 的
/// `Enumerated` 载荷换成裸 `Vec<String>` 则 `Conflict` / `UndefinedBehavior`
/// 两个类型没有落点，本用例与其编译期照片一起红。
#[test]
fn conflict_and_undefined_behaviour_are_types_only() {
    let (_d, db) = new_db();
    let tx = db.begin().unwrap();

    let unknowns = Unknowns::Enumerated(vec![
        Unknown::Conflict(Conflict::from_str("g-1 与 g-2 的目标互斥")),
        Unknown::UndefinedBehavior(UndefinedBehavior::from_str("失败重试的停止条件未定义")),
    ]);
    save_intent_with_specification(
        &tx,
        &sample_intent("i-1", "goal-a", 1_000),
        sample_draft(),
        unknowns,
    )
    .unwrap();
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let spec = saved_spec(&tx, "i-1", 1);
    match spec.unknowns() {
        Unknowns::NoneFound => panic!("写的是 `Enumerated`，读回不该是 `NoneFound`"),
        Unknowns::Enumerated(items) => {
            assert_eq!(items.len(), 2, "写进去两枚，读回应恰两枚（顺序保留）");
            match &items[0] {
                Unknown::Conflict(c) => assert_eq!(c.as_str(), "g-1 与 g-2 的目标互斥"),
                Unknown::UndefinedBehavior(_) => {
                    panic!("第一枚写的是 `Conflict`，读回不该是 `UndefinedBehavior`")
                }
            }
            match &items[1] {
                Unknown::UndefinedBehavior(u) => {
                    assert_eq!(u.as_str(), "失败重试的停止条件未定义")
                }
                Unknown::Conflict(_) => {
                    panic!("第二枚写的是 `UndefinedBehavior`，读回不该是 `Conflict`")
                }
            }
        }
    }
    tx.commit().unwrap();
}

/// 编码纪律（小写、多词接 `_`）**逐枚**读库，以及表外取值的**具体** `Err`。
///
/// 五段：
/// (a) `unknowns_kind` 两枚的字面量（`none_found` / `enumerated`），raw 读列；
/// (b) `Unknown` 的 kind 两枚的字面量（`conflict` / `undefined_behavior`），
///     raw 读 JSON 再解析；
/// (c) `unknowns_kind` 列表外 ⇒ `Err(SpecError::UnknownsKind { value })`
///     （带库里那一个原始串，不取默认值）；
/// (d) `Unknown` 的 kind 表外 ⇒ `Err(SpecError::UnknownItemKind { value })`；
/// (e) **容器坏**（`goals` 列不是 JSON 数组）⇒ `Persist(Database)`，消息点名 `goals`
///     ——「容器坏了才报错、元素是自由文本没有表外取值」是 `continuum-model-registry`
///     的列表列同一条分工（设计 §14 的「编码纪律」一行）。
///
/// 只断言往返一致是不够的：编码与解码互为逆函数时，一张把两枚都编成同一个串的表照样
/// 往返得通（同 `tests/intent.rs` 的 `enum_columns_…` 的理由）。
///
/// **红条件**：编码改成大写／去掉 `_`／表外取值取默认值／容器错被吞进元素里即红。
#[test]
fn the_unknowns_columns_use_lowercase_underscore_and_reject_out_of_table_values() {
    let (_d, db) = new_db();

    // (a) `unknowns_kind` 两枚各写一次、raw 读列。
    let tx = db.begin().unwrap();
    save_intent_with_specification(
        &tx,
        &sample_intent("i-1", "goal-a", 1_000),
        sample_draft(),
        Unknowns::NoneFound,
    )
    .unwrap();
    assert_eq!(
        raw_text(&tx, "i-1", 1, "unknowns_kind"),
        "none_found",
        "(a) `NoneFound` 的落库编码应为小写、多词接 `_`"
    );
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    save_intent_with_specification(
        &tx,
        &sample_intent("i-1", "goal-b", 2_000),
        sample_draft(),
        Unknowns::Enumerated(vec![]),
    )
    .unwrap();
    assert_eq!(
        raw_text(&tx, "i-1", 2, "unknowns_kind"),
        "enumerated",
        "(a) `Enumerated` 的落库编码应为小写"
    );
    tx.commit().unwrap();

    // (b) `Unknown` 的 kind 两枚各写一次、raw 读 JSON。
    let tx = db.begin().unwrap();
    save_intent_with_specification(
        &tx,
        &sample_intent("i-1", "goal-c", 3_000),
        sample_draft(),
        Unknowns::Enumerated(vec![
            Unknown::Conflict(Conflict::from_str("c")),
            Unknown::UndefinedBehavior(UndefinedBehavior::from_str("u")),
        ]),
    )
    .unwrap();
    let raw_unknowns = raw_text(&tx, "i-1", 3, "unknowns");
    let parsed: serde_json::Value = serde_json::from_str(&raw_unknowns).unwrap();
    let kinds: Vec<&str> = parsed
        .as_array()
        .expect("unknowns 列应是 JSON 数组")
        .iter()
        .map(|item| item.get("kind").and_then(|v| v.as_str()).expect("元素应有 kind"))
        .collect();
    assert_eq!(
        kinds,
        vec!["conflict", "undefined_behavior"],
        "(b) `Unknown` 两枚的落库编码应为小写、多词接 `_`"
    );
    tx.commit().unwrap();

    // (c) `unknowns_kind` 表外取值：绕开公开写路径直接改列（模拟历史行／被外部改过的行）。
    let tx = db.begin().unwrap();
    tx.execute(
        "UPDATE specification SET unknowns_kind = ?1 WHERE intent_id = ?2 AND version = ?3",
        &[
            Value::text("nonsense"),
            Value::text("i-1"),
            Value::Int(1),
        ],
    )
    .unwrap();
    assert_eq!(
        load_specification(&tx, &IntentId::from_str("i-1"), 1).unwrap_err(),
        SpecError::UnknownsKind {
            value: "nonsense".to_owned()
        },
        "(c) `unknowns_kind` 的表外取值应读回**具体**错误、并带库里那一个原始串（不取默认值）"
    );

    // (d) `Unknown` 的 kind 表外取值：把第三行（v3）的列表元素改坏。
    tx.execute(
        "UPDATE specification SET unknowns = ?1 WHERE intent_id = ?2 AND version = ?3",
        &[
            Value::text(r#"[{"kind": "nonsense", "value": "x"}]"#),
            Value::text("i-1"),
            Value::Int(3),
        ],
    )
    .unwrap();
    assert_eq!(
        load_specification(&tx, &IntentId::from_str("i-1"), 3).unwrap_err(),
        SpecError::UnknownItemKind {
            value: "nonsense".to_owned()
        },
        "(d) `Unknown` 的 kind 表外取值应读回**具体**错误、并带原始串（不取默认值）"
    );

    // (e) 容器坏：`goals` 列不是 JSON 数组。
    tx.execute(
        "UPDATE specification SET goals = ?1 WHERE intent_id = ?2 AND version = ?3",
        &[
            Value::text("not-json"),
            Value::text("i-1"),
            Value::Int(1),
        ],
    )
    .unwrap();
    let err = load_specification(&tx, &IntentId::from_str("i-1"), 1).unwrap_err();
    assert!(
        matches!(&err, SpecError::Persist(PersistError::Database(m)) if m.contains("goals")),
        "(e) 容器坏应是「goals 不是 JSON 数组」的 Persist，实得 {err:?}"
    );

    tx.commit().unwrap();
}
