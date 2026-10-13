//! §223 的 Intent 状态机（§5.3 的迁移表、§223 的 `COMPLETED → EVOLVING` 条件）
//! 与「状态迁移与事件同事务」（§5.4）的用例集。
//!
//! # 本文件钉的每一件，以及它的红条件
//!
//! | 用例 | 钉的判据 | 红条件（档位） |
//! |---|---|---|
//! | `every_listed_pair_is_accepted` | §5.3 的迁移表**逐条**（31 条，含两条 `Ok` 自环与 `COMPLETED → EVOLVING` 的 Product 一侧）：`apply_intent_transition` 得 `Ok`，且迁移真的落库（`status` 到位、`updated_at` 走到 `occurred_at`） | 表里任一对被判非法（**收紧档**——那一条红）；迁移不改 `updated_at`（**移除档**——读回断言红） |
//! | `unlisted_pairs_are_illegal_and_the_error_names_the_pair` | 8 条未列出的对得 `Err(Illegal { from, to })`，**且错误里就是给的那一对**；非法对不动库里的行（`status` / `updated_at` 原样） | 未列出的对被放行（**放宽档**——那一条红）；错误不点名那一对（同断言）；先斩后奏地写了行（行没动的断言红） |
//! | `self_loops_are_decided_on_both_sides` | 自环专案（不靠矩阵顺带）：`RUNNING → RUNNING` 与 `WAITING → WAITING` 两条 `Ok`；`PROPOSED → PROPOSED` 与 `COMPLETED → COMPLETED` 两条 `Err(Illegal { from, to })` | 自环随矩阵顺带判（**取反档**——`PROPOSED → PROPOSED` 那一侧红） |
//! | `completed_to_evolving_depends_on_the_kind` | `COMPLETED → EVOLVING` **两侧都钉**：`kind = PRODUCT` 得 `Ok`；`kind = TASK` 得 `Err(NotProduct { kind })`，**断言是哪一枚** | 不判 kind（**放宽档**——TASK 那条红） |
//! | `the_intent_created_event_is_written_on_the_first_save` | §5.4：`Intent` 首次落库写出**恰一条** `IntentCreated`（断言事件类型、信封 `intent_id` 列与 payload 里的 `intent_id`）；第二次 save 不重复写 | 把事件类型写成别的（**取反档**）；每次 save 都写（**放宽档**——第二次的计数断言红） |
//! | `the_intent_completed_event_is_written_on_the_completed_transition` | §5.4：迁移到 `COMPLETED` 写出 `IntentCompleted`（类型、信封与 payload 各一断言）；中间状态的迁移不写事件 | 把事件类型写成别的（**取反档**）；每次迁移都写（**放宽档**——事件清单全等断言红） |
//! | `a_failed_event_write_leaves_no_half_written_row` | **失败路径 + 半写副作用**：`events` 表不存在时返回具体 `Err(Persist)`（消息点名 events），且**回滚后**那一行的 `status` / `updated_at` 是迁移前的值（行是在的，改没改才是判据） | 吞掉事件写入的失败（**放宽档**）；改不了行还报成功；起始行随失败消失（「行是在的」那一半红） |
//! | `writing_intent_completed_behind_a_skippable_event_is_rejected` | §5.4 第 2 条（**两侧都钉**）：(a) 链上先有可跳过事件时写 `IntentCompleted` 被拒（被 `scan_event_log` 的收口判定拒绝，且回滚后状态未变）；(b) 链上无可跳过事件时得 `Ok` | 只钉 (a) 会让本层写 `IntentCompleted` 的正常路径无人过问（红条件即 (b) 缺失）；不检查链（**放宽档**——(a) 红） |
//! | `an_unknown_intent_is_rejected_and_names_the_id` | 库里没有这个 Intent 时不做任何迁移、不静默创建行，错误点名那个 id | 静默创建／静默通过（**放宽档**）；错误丢名字（同断言） |
//!
//! # 夹具的来历
//!
//! `new_db()` 与 `tests/intent.rs` 的 `db()` 夹具同形（名字带 `new_` 是因为本文件有几条用例
//! 在一个函数里起两次库，`let (_d, db) = new_db();` 的第二次调用会被同名变量遮蔽）；
//! `new_db_without_events()` 是失败路径的造法（照 `continuum-workspace/src/gate.rs` 的
//! `a_failed_audit_write_…`：迁移集里去掉那张表）。
//! `save_intent` 在本文件里被用作「造起点状态」的后门——它不判迁移对（判定在
//! `apply_intent_transition` 里）。

use continuum_canonical::{EntityId, Opaque};
use continuum_persist::{Db, EventType, Migration, PersistError, Tx, Value, builtin_migrations};
use continuum_semantics::{
    ContractId, ContractRef, Intent, IntentError, IntentId, IntentKind, IntentStatus,
    IntentStatusError, Priority, apply_intent_transition, load_intent, p4_semantics_migrations,
    save_intent,
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

/// 只装本 crate 迁移的库：`intent` 表在，**`events` 表不存在**。
///
/// 失败路径的造法照 `continuum-workspace/src/gate.rs` 的
/// `a_failed_audit_write_reports_persist_…`（迁移集里去掉那张表，于是写事件必然失败），
/// 比伪造一个失败的 `Tx` 更省事，也仍是真实的数据库错误。
fn new_db_without_events() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let db = Db::open_with(&dir.path().join("t.db"), p4_semantics_migrations()).unwrap();
    db.migrate().unwrap();
    (dir, db)
}

/// 一条基准 Intent：每个字段都取一个非默认值（同 `tests/intent.rs` 的 `sample`）。
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

/// 以给定 `kind` / `status` 往库里写一条**起点** Intent。
///
/// `save_intent` 不判迁移对——它是本文件「直接造出起点状态」的后门；
/// 判定的照片全部落在 `apply_intent_transition` 上。
fn save_start(tx: &Tx<'_>, id: &str, kind: IntentKind, status: IntentStatus) {
    let mut intent = sample(id);
    intent.kind = kind;
    intent.status = status;
    save_intent(tx, &intent).unwrap();
}

/// 读回一条 Intent；不存在直接炸（用例里它必须存在）。
fn load(tx: &Tx<'_>, id: &str) -> Intent {
    load_intent(tx, &IntentId::from_str(id))
        .unwrap()
        .unwrap_or_else(|| panic!("库里应有 {id:?}"))
}

/// 按 `index` 取一列文本。
fn text_of(v: &Value) -> String {
    match v {
        Value::Text(s) => s.clone(),
        other => panic!("列应为文本，实得 {other:?}"),
    }
}

/// 库里全部事件的类型，按写入顺序（rowid）。
///
/// `event_type` 经 [`EventType`] 的 serde 表示还原：写入侧用的是 `EventType::as_str`
/// （见 `Tx::append_event`），两侧是同一份短名的两个来源，本函数因此顺带钉住
/// 「`as_str` 与 serde 表示没有分叉」（照 `continuum-graph/tests/transition_events.rs`
/// 的 `events_of` 的理由）。[`EventType`] 经 `continuum_persist` 的转出取——本 crate
/// 不登记指向 `continuum-events` 的依赖边。
fn event_types(tx: &Tx<'_>) -> Vec<EventType> {
    tx.query("SELECT event_type FROM events ORDER BY rowid", &[])
        .unwrap()
        .iter()
        .map(|row| {
            let name = text_of(&row[0]);
            serde_json::from_value::<EventType>(serde_json::Value::String(name.clone()))
                .unwrap_or_else(|e| panic!("event_type {name:?} 不是已知的 EventType：{e}"))
        })
        .collect()
}

/// `events.intent_id` 列，按写入顺序。信封上「这条事件属于哪个 Intent」的唯一来源。
fn event_intent_ids(tx: &Tx<'_>) -> Vec<Option<String>> {
    tx.query("SELECT intent_id FROM events ORDER BY rowid", &[])
        .unwrap()
        .iter()
        .map(|row| match &row[0] {
            Value::Null => None,
            Value::Text(s) => Some(s.clone()),
            other => panic!("intent_id 列应是文本或 NULL，实得 {other:?}"),
        })
        .collect()
}

/// 某一类型的事件的 payload，按写入顺序，经 JSON 解析。
fn payloads_of(tx: &Tx<'_>, event_type: &str) -> Vec<serde_json::Value> {
    tx.query(
        "SELECT payload FROM events WHERE event_type = ?1 ORDER BY rowid",
        &[Value::text(event_type)],
    )
    .unwrap()
    .iter()
    .map(|row| match &row[0] {
        Value::Text(s) => {
            serde_json::from_str(s).unwrap_or_else(|e| panic!("payload 应是合法 JSON：{e}"))
        }
        other => panic!("payload 列应是文本，实得 {other:?}"),
    })
    .collect()
}

/// 直接插一条**可跳过**的事件记录（绕过 `append_event`）：`schema_version` 列写成文本，
/// 版本号不可判定 ⇒ 该记录可跳过。造法照 `continuum-persist/tests/transaction.rs` 的
/// `scan_counts_skippable_records`。
fn insert_skippable_event(tx: &Tx<'_>, event_id: &str) {
    tx.execute(
        "INSERT INTO events
           (event_id, event_type, schema_version, occurred_at, intent_id, node_id, ignorable, payload)
         VALUES (?1, 'node.started', ?2, 1, NULL, NULL, 0, '{}')",
        &[Value::text(event_id), Value::text("one")],
    )
    .unwrap();
}

/// 直接插一行 `intent`（绕过 `save_intent`）。
///
/// `save_intent` 首次落库要写 `IntentCreated`，而本函数的用例库**没有 `events` 表**，
/// 公开写路径走不通；起始行因此经裸 SQL 造出（同 `tests/intent.rs` 里造表外取值的做法）。
fn insert_raw_intent(tx: &Tx<'_>, id: &str, status: &str) {
    tx.execute(
        "INSERT INTO intent
           (id, version, kind, goal, status, priority, created_at, updated_at, parent_intent,
            contract_id, contract_version, active_plan_id, graph_id, product_id, completion_predicate)
         VALUES (?1, 1, 'task', 'goal-a', ?2, 'high', 1000, 1000, NULL, 'c-1', 1, NULL, NULL, NULL, 'pred-a')",
        &[Value::text(id), Value::text(status)],
    )
    .unwrap();
}

/// §5.3 的迁移表**逐条**（不抽代表）：31 条，含两条 `Ok` 自环
/// （`RUNNING → RUNNING` / `WAITING → WAITING`）与 `COMPLETED → EVOLVING` 的
/// Product 一侧（`TASK` 那一侧是 [`completed_to_evolving_depends_on_the_kind`] 的专案）。
///
/// 每条对**各起一个库**：`from` / `to` / `kind` 三方不会互相纠缠。除「得 `Ok`」之外，
/// 每条对还读回断言 `status` 到位、`updated_at` 走到 `occurred_at`
/// （§5.2：状态迁移改 `updated_at`）——只断言 `Ok` 会让「判定放行但什么都没写」漂过去。
#[test]
fn every_listed_pair_is_accepted() {
    use IntentKind::{Product, Task};
    use IntentStatus::*;

    let pairs: [(IntentStatus, IntentStatus, IntentKind); 31] = [
        (Proposed, Specifying, Task),
        (Proposed, Cancelled, Task),
        (Specifying, Planning, Task),
        (Specifying, Proposed, Task), // 规格化失败回退
        (Specifying, Cancelled, Task),
        (Planning, Review, Task),
        (Planning, Suspended, Task),
        (Planning, Cancelled, Task),
        (Review, Ready, Task),
        (Review, Planning, Task), // 审查驳回
        (Review, Cancelled, Task),
        (Ready, Running, Task),
        (Ready, Cancelled, Task),
        (Running, Waiting, Task),
        (Running, Suspended, Task),
        (Running, Verifying, Task),
        (Running, Failed, Task),
        (Running, Cancelled, Task),
        (Running, Running, Task), // §5.3 自环单列（Ok 侧）
        (Waiting, Running, Task),
        (Waiting, Suspended, Task),
        (Waiting, Cancelled, Task),
        (Waiting, Waiting, Task), // §5.3 自环单列（Ok 侧）
        (Suspended, Running, Task),
        (Suspended, Cancelled, Task),
        (Verifying, Completed, Task),
        (Verifying, Failed, Task),
        (Verifying, Running, Task), // 验证失败返工
        (Completed, Evolving, Product), // 仅 Product（§223 原文「Product Intent MAY……」）
        (Evolving, Running, Task),
        (Evolving, Cancelled, Task),
    ];

    for (from, to, kind) in pairs {
        let (_d, db) = new_db();
        let tx = db.begin().unwrap();
        save_start(&tx, "i-1", kind, from);

        apply_intent_transition(&tx, &IntentId::from_str("i-1"), to, 9_000, "ev-1")
            .unwrap_or_else(|e| panic!("{from:?} → {to:?}（kind = {kind:?}）在迁移表里，应被接受：{e:?}"));

        let back = load(&tx, "i-1");
        assert_eq!(
            back.status, to,
            "{from:?} → {to:?} 被判 `Ok`，但库里读回的状态是 {:?}（判定与落库不是一件事）",
            back.status
        );
        assert_eq!(
            back.updated_at, 9_000,
            "{from:?} → {to:?} 的迁移应把 `updated_at` 走到 `occurred_at`（§5.2：状态迁移改它）"
        );
        tx.commit().unwrap();
    }
}

/// 未列出的对得 `Err(Illegal { from, to })`——**断言 `from` / `to` 是给的那一对**
/// （不是「返回了 `Err`」）。
///
/// 另钉「先判合法性、再写行」的形状：非法对**不留下半写的行**（`status` / `updated_at`
/// 原样）。这里**提交**而不是丢弃事务——丢弃必然把事务清干净，「一行都没写」就成了
/// 空断言（同 `continuum-graph/tests/transition_events.rs` 的
/// `illegal_transition_writes_neither_state_nor_event` 的理由）；提交才让它是可证伪的。
#[test]
fn unlisted_pairs_are_illegal_and_the_error_names_the_pair() {
    use IntentStatus::*;

    let pairs: [(IntentStatus, IntentStatus); 8] = [
        (Proposed, Running),    // 跳过整条链
        (Proposed, Completed),
        (Ready, Verifying),     // 跨阶段
        (Running, Completed),   // 不经验证直接收口
        (Verifying, Waiting),
        (Completed, Running),   // 终态出边（EVOLVING 之外没有）
        (Evolving, Completed),
        (Failed, Running),      // 终态
    ];

    for (from, to) in pairs {
        let (_d, db) = new_db();
        let tx = db.begin().unwrap();
        save_start(&tx, "i-1", IntentKind::Task, from);
        let before = load(&tx, "i-1");

        let err = apply_intent_transition(&tx, &IntentId::from_str("i-1"), to, 9_000, "ev-1")
            .unwrap_err();
        assert_eq!(
            err,
            IntentError::Transition(IntentStatusError::Illegal { from, to }),
            "{from:?} → {to:?} 不在迁移表里，错误里应原样带出那一对"
        );

        let after = load(&tx, "i-1");
        assert_eq!(after.status, from, "被拒的迁移改了库里的 status（先斩后奏）");
        assert_eq!(
            after.updated_at, before.updated_at,
            "被拒的迁移改了库里的 updated_at"
        );
        tx.commit().unwrap();
    }

    // 终态的另一半：`Cancelled` 同样没有出边。
    let (_d, db) = new_db();
    let tx = db.begin().unwrap();
    save_start(&tx, "i-1", IntentKind::Task, Cancelled);
    let err =
        apply_intent_transition(&tx, &IntentId::from_str("i-1"), Proposed, 9_000, "ev-2").unwrap_err();
    assert_eq!(
        err,
        IntentError::Transition(IntentStatusError::Illegal {
            from: Cancelled,
            to: Proposed,
        }),
        "Cancelled 是终态，向外的迁移应被拒"
    );
    assert_eq!(load(&tx, "i-1").status, Cancelled, "被拒的迁移改了库");
    tx.commit().unwrap();
}

/// **自环专案**（§5.3「自环单列」；照 P3D 的纪律：自环不靠矩阵顺带），**两侧都钉**。
///
/// - `Ok` 侧：`RUNNING → RUNNING` 与 `WAITING → WAITING`；
/// - `Err` 侧：`PROPOSED → PROPOSED` 与 `COMPLETED → COMPLETED`
///   （未列进表的自环一律 `Illegal { from, to }`，且不动库里的行）。
///
/// **红条件**：自环随矩阵顺带判——只钉放行那侧会让「没列进表的自环」漂过去
/// （`PROPOSED → PROPOSED` 那一侧红）；只钉拒绝那侧则相反。
#[test]
fn self_loops_are_decided_on_both_sides() {
    use IntentStatus::*;

    // Ok 侧两条（逐条，不抽代表）。
    for state in [Running, Waiting] {
        let (_d, db) = new_db();
        let tx = db.begin().unwrap();
        save_start(&tx, "i-1", IntentKind::Task, state);
        apply_intent_transition(&tx, &IntentId::from_str("i-1"), state, 9_000, "ev-1")
            .unwrap_or_else(|e| panic!("{state:?} → {state:?} 在表里，应被接受：{e:?}"));
        tx.commit().unwrap();
    }

    // Err 侧两条（断言带出的正是那一对，且行没动）。
    for state in [Proposed, Completed] {
        let (_d, db) = new_db();
        let tx = db.begin().unwrap();
        save_start(&tx, "i-1", IntentKind::Task, state);
        let err = apply_intent_transition(&tx, &IntentId::from_str("i-1"), state, 9_000, "ev-1")
            .unwrap_err();
        assert_eq!(
            err,
            IntentError::Transition(IntentStatusError::Illegal {
                from: state,
                to: state,
            }),
            "{state:?} → {state:?} 不在迁移表里，应被拒且带出那一对"
        );
        assert_eq!(load(&tx, "i-1").status, state, "被拒的自环改了库里的行");
        tx.commit().unwrap();
    }
}

/// `COMPLETED → EVOLVING` **只在 `kind = PRODUCT` 时合法**（§223 原文「Product Intent MAY……」），
/// **两侧都钉**。
///
/// - `PRODUCT`：`Ok`；
/// - `TASK`：`Err(NotProduct { kind: Task })`——**断言是哪一枚**（不是「返回了 `Err`」），且不动库。
///
/// **红条件**：不判 kind（**放宽档**——`TASK` 那条红）。
#[test]
fn completed_to_evolving_depends_on_the_kind() {
    // PRODUCT 一侧。
    let (_d, db) = new_db();
    let tx = db.begin().unwrap();
    save_start(&tx, "i-1", IntentKind::Product, IntentStatus::Completed);
    apply_intent_transition(
        &tx,
        &IntentId::from_str("i-1"),
        IntentStatus::Evolving,
        9_000,
        "ev-1",
    )
    .expect("Product Intent 的 COMPLETED → EVOLVING 应被接受");
    tx.commit().unwrap();

    // TASK 一侧。
    let (_d, db) = new_db();
    let tx = db.begin().unwrap();
    save_start(&tx, "i-1", IntentKind::Task, IntentStatus::Completed);
    let err = apply_intent_transition(
        &tx,
        &IntentId::from_str("i-1"),
        IntentStatus::Evolving,
        9_000,
        "ev-1",
    )
    .unwrap_err();
    assert_eq!(
        err,
        IntentError::Transition(IntentStatusError::NotProduct {
            kind: IntentKind::Task,
        }),
        "TASK 的 COMPLETED → EVOLVING 应报 NotProduct 并带出那个 kind"
    );
    assert_eq!(
        load(&tx, "i-1").status,
        IntentStatus::Completed,
        "被拒的迁移改了库里的行"
    );
    tx.commit().unwrap();
}

/// §5.4：`Intent` **首次落库**写出恰一条 `IntentCreated`——**恰在首次**（第二次 save 不重复写）。
///
/// 断言三件事：事件类型是 `IntentCreated`（经 `EventType` 的 serde 表示还原）、
/// 信封的 `intent_id` 列指向这个 Intent、payload 里点名这个 Intent。
/// 「恰一条」由全等断言与第二次 save 后的重复计数共同钉住。
///
/// **红条件**：把事件类型写成别的（**取反档**——类型断言红）；每次 save 都写
/// （**放宽档**——第二次的计数断言红；若事件 id 是派生的，第二次还会撞主键，
/// `save_intent` 本身会失败）。
#[test]
fn the_intent_created_event_is_written_on_the_first_save() {
    let (_d, db) = new_db();
    let tx = db.begin().unwrap();

    save_intent(&tx, &sample("i-1")).unwrap();
    assert_eq!(
        event_types(&tx),
        vec![EventType::IntentCreated],
        "首次落库应恰写出一个 intent.created"
    );
    assert_eq!(
        event_intent_ids(&tx),
        vec![Some("i-1".to_owned())],
        "信封的 intent_id 列应指向这个 Intent"
    );
    let payloads = payloads_of(&tx, EventType::IntentCreated.as_str());
    assert_eq!(payloads.len(), 1, "intent.created 应恰一条");
    assert_eq!(
        payloads[0].get("intent_id").and_then(|v| v.as_str()),
        Some("i-1"),
        "payload 应点名是哪个 Intent"
    );

    // 第二次 save（一次更新）：不写第二条。
    let mut second = sample("i-1");
    second.priority = Priority::from_str("low");
    second.updated_at = 2_000;
    save_intent(&tx, &second).unwrap();
    assert_eq!(
        event_types(&tx),
        vec![EventType::IntentCreated],
        "第二次 save 不该再写 intent.created"
    );

    tx.commit().unwrap();
}

/// §5.4：迁移到 `COMPLETED` 写出 `IntentCompleted`（类型、信封与 payload 各一断言）；
/// 到达其他状态的迁移不写事件。
///
/// **红条件**：把事件类型写成别的（**取反档**）；每次迁移都写（**放宽档**——
/// 中间态的迁移若写事件，「恰两条」的全等断言红）。
#[test]
fn the_intent_completed_event_is_written_on_the_completed_transition() {
    let (_d, db) = new_db();
    let tx = db.begin().unwrap();
    save_start(&tx, "i-1", IntentKind::Task, IntentStatus::Verifying);

    apply_intent_transition(
        &tx,
        &IntentId::from_str("i-1"),
        IntentStatus::Completed,
        9_000,
        "ev-done",
    )
    .unwrap();

    assert_eq!(
        event_types(&tx),
        vec![EventType::IntentCreated, EventType::IntentCompleted],
        "落库一条 intent.created（首次）与一条 intent.completed（收口），别无其他"
    );
    assert_eq!(
        event_intent_ids(&tx),
        vec![Some("i-1".to_owned()), Some("i-1".to_owned())],
        "两条事件的信封都应指向这个 Intent"
    );
    let payloads = payloads_of(&tx, EventType::IntentCompleted.as_str());
    assert_eq!(payloads.len(), 1, "intent.completed 应恰一条");
    assert_eq!(
        payloads[0].get("intent_id").and_then(|v| v.as_str()),
        Some("i-1"),
        "payload 应点名是哪个 Intent"
    );

    tx.commit().unwrap();
}

/// **失败路径 + 半写副作用**：`events` 表不存在时，状态迁移先改 `intent` 行、
/// 再写事件而失败。
///
/// 断言**返回具体 `Err`**（`Persist`，消息点名 `events` 表）**且**回滚后 `intent` 表里
/// 那一行的 `status` / `updated_at` **是迁移前的值**——不是「表里没有行」：
/// 行是在的（起始行由本用例自己造并提交），改没改才是判据。
///
/// **前提**：`apply_intent_transition` 不自己 `commit`——「调用方在 `Err` 后回滚」
/// 就是本用例丢掉 `Tx`（`Drop` 即 `ROLLBACK`）那一步。另有一层类型保证：
/// 本函数拿不到 `Db`，而 `Tx::commit` 消费 `self`（同 `continuum-graph` 的
/// `rollback_discards_both_state_and_event` 的注释）。
#[test]
fn a_failed_event_write_leaves_no_half_written_row() {
    let (_d, db) = new_db_without_events();
    // 起始行经裸 SQL 造出：`save_intent` 首次落库要写 `IntentCreated`，这个库没有 events 表。
    let tx = db.begin().unwrap();
    insert_raw_intent(&tx, "i-1", "verifying");
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let err = apply_intent_transition(
        &tx,
        &IntentId::from_str("i-1"),
        IntentStatus::Completed,
        9_000,
        "ev-1",
    )
    .unwrap_err();
    assert!(
        matches!(&err, IntentError::Persist(PersistError::Database(m)) if m.contains("events")),
        "期望具体 Err 是「events 表缺失」的 Persist，实得 {err:?}"
    );
    // 调用方在 `Err` 后回滚（丢弃 Tx）。
    drop(tx);

    let tx = db.begin().unwrap();
    let back = load(&tx, "i-1");
    assert_eq!(
        back.status,
        IntentStatus::Verifying,
        "回滚后 status 应是迁移前的值（不是「表里没有行」——行是在的）"
    );
    assert_eq!(
        back.updated_at, 1_000,
        "回滚后 updated_at 应是迁移前的值"
    );
    tx.commit().unwrap();
}

/// §5.4 第 2 条，**两侧都钉**。
///
/// (a) 链上先有一条**可跳过**（不可解码／版本未知）的事件，再迁移到 `COMPLETED`：
///     `IntentCompleted` 的既有消费者（`Tx::scan_event_log`：可跳过记录之后存在
///     `intent.completed` 即报 `Err`）使这次迁移被拒——断言具体 `Err`（消息是消费者
///     那条判定），且**回滚后**状态仍是迁移前的值（拒绝不是只报个错：写入也撤掉）。
/// (b) **反例一条**：链上无可跳过事件时，同一条迁移得 `Ok`——只钉 (a) 会让
///     「本层能不能写 `IntentCompleted`」这件事在正常路径上无人过问。
///
/// **红条件**：不检查链（**放宽档**——(a) 红）。
#[test]
fn writing_intent_completed_behind_a_skippable_event_is_rejected() {
    // (a) 可跳过记录在前。
    let (_d, db) = new_db();
    // 起点行与可跳过记录**先提交**（模拟「别层在此前写了坏记录」的处境）；
    // 这样 apply 失败后的回滚才只撤它自己的半写，而不是把起点一起撤掉。
    let tx = db.begin().unwrap();
    save_start(&tx, "i-1", IntentKind::Task, IntentStatus::Verifying);
    insert_skippable_event(&tx, "ev-skip");
    tx.commit().unwrap();

    let tx = db.begin().unwrap();
    let err = apply_intent_transition(
        &tx,
        &IntentId::from_str("i-1"),
        IntentStatus::Completed,
        9_000,
        "ev-done",
    )
    .unwrap_err();
    assert!(
        matches!(&err, IntentError::Persist(PersistError::Database(m))
            if m.contains("可跳过事件之后存在 intent.completed")),
        "(a) 期望消费者（scan_event_log 的收口判定）那条 Err，实得 {err:?}"
    );
    // 调用方在 `Err` 后回滚（丢弃 Tx）。
    drop(tx);

    let tx = db.begin().unwrap();
    assert_eq!(
        load(&tx, "i-1").status,
        IntentStatus::Verifying,
        "(a) 被拒的迁移应随回滚撤销（status 仍是迁移前的值）"
    );
    tx.commit().unwrap();

    // (b) 反例：链上无可跳过事件时同一条迁移得 Ok。
    let (_d, db) = new_db();
    let tx = db.begin().unwrap();
    save_start(&tx, "i-1", IntentKind::Task, IntentStatus::Verifying);
    apply_intent_transition(
        &tx,
        &IntentId::from_str("i-1"),
        IntentStatus::Completed,
        9_000,
        "ev-done",
    )
    .expect("(b) 链上无可跳过事件时，写 IntentCompleted 应得 Ok");
    assert_eq!(load(&tx, "i-1").status, IntentStatus::Completed);
    tx.commit().unwrap();
}

/// 库里没有这个 Intent 时不做任何迁移——**不静默创建行**（同 P1 的 `UnknownNode` /
/// P3D 的 `UnknownModel`：把一次笔误的 id 变成一行新 Intent，会让「没见过的 Intent」
/// 与「打错字的 Intent」在库里长得一样），错误点名那个 id。
///
/// **红条件**：静默创建／静默通过（**放宽档**）。
#[test]
fn an_unknown_intent_is_rejected_and_names_the_id() {
    let (_d, db) = new_db();
    let tx = db.begin().unwrap();

    let err = apply_intent_transition(
        &tx,
        &IntentId::from_str("i-404"),
        IntentStatus::Proposed,
        9_000,
        "ev-1",
    )
    .unwrap_err();
    assert_eq!(
        err,
        IntentError::UnknownIntent {
            id: "i-404".to_owned()
        },
        "错误的 id 应原样带出"
    );
    assert!(
        load_intent(&tx, &IntentId::from_str("i-404"))
            .unwrap()
            .is_none(),
        "被拒的调用不该凭空造出一行 Intent"
    );
    // 提交而非丢弃：若实现偷偷写了半行，会留在已提交的事务里（见本文件另一条用例的注释）。
    tx.commit().unwrap();
}
