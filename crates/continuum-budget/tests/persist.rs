//! Task 17 的账本与表形状半边（运行期的 `tree.rs` 半边见 `tests/tree.rs`）：
//! 迁移 120 的登记、两张表的列清单、账本的只追加与派生余额、悬空预扣的可检出、
//! 以及枚举列的编码纪律（小写加下划线、与类型同址的 `as_str`/`parse`、
//! 表外取值读回具体 `Err`）。
//! 设计 §12.2／§12.2.1／§2.4；规范 §333；ENG-005 §三。
//!
//! # 本文件钉的每一件，以及它的红条件
//!
//! | 用例 | 钉的判据 | 红条件（档位） |
//! |---|---|---|
//! | `the_migration_120_is_a_single_item_with_both_tables` | 120 是**一个**迁移项（第二个同号项会被 `migrate()` 静默跳过——第二个表永不建），且 SQL 里两张表都在 | 拆成两个 120 项（**移除档**——`len()==1` 那半边红） |
//! | `the_two_budget_tables_have_exactly_the_expected_columns` | 两张表**逐列**（实际列 vs 手写期望） | 加/删/改名一列即红 |
//! | `the_ledger_is_append_only_and_balances_are_derived` | 只追加、不 UPDATE：先前的行在任何后续操作后**逐行不变**（前缀比对）；余额**现算**（`remaining()` 立刻反映）；表里**没有 `balance` 一列**（逐列） | 把余额落成一列并原地更新（**移除档**——`balance` 列断言红）；改旧行（前缀比对红） |
//! | `an_over_settlement_writes_no_second_ledger_row` | 一次 `Over` 结算后账本**只多一条 `settle` 行**（否定式照片——超支的裁决不在本层，账上不留裁决的痕迹） | 为超支写第二行账（**移除档**——条数断言红） |
//! | `a_dangling_reservation_is_detectable` | 「有一条 `reserve` 而没有对应的 `settle` 或 `release`」**是可检出的**（两侧：`settle` 后、`release` 行后都不再列出；两条 `reserve` 一条 `settle` 时列出**未消化的那一条**） | 检不出（**放宽档**）；消化的那条仍被列出（FIFO 配对红） |
//! | `the_enum_codes_use_lowercase_underscore_and_parse_back` | 两个枚举列（`dimension` / `kind`）与 owner 的复合键：编码小写加下划线、`as_str`/`parse` **与类型同址**、四/五枚逐枚往返 | 编码改成大写/连字符（**取反档**——逐枚断言红） |
//! | `out_of_table_values_read_back_as_specific_errors` | 表外取值读回**具体 `Err`**、**带原始串、不取默认值**（三列各一条：`owner` / `dimension` / `kind`） | 表外取值被静默映射成某一枚（**放宽档**）；读回 `Ok` 或不点名原串即红 |
//!
//! # 夹具的来历
//!
//! `new_db()` 与 `tests/tree.rs` 的 `new_db()` 同形（P0 内建迁移 ＋ 本 crate 迁移 120，
//! 建库后必须 `migrate()`）。「表外取值」三条经**裸 `INSERT`** 造行——那是测试的常规
//! 手法（同 `tests/contract.rs` 的二次写照片），本层没有任何写路径会产出表外取值。

use continuum_budget::{
    BudgetError, BudgetOwner, DanglingReservation, DimensionKind, Dimensions, LedgerKind,
    allocate, dangling_reservations, p4_budget_migrations, record_root, remaining, reserve, settle,
};
use continuum_persist::{Db, Migration, PersistError, Tx, Value, builtin_migrations};
use continuum_semantics::IntentId;

/// 建一个只装了 P0 内建迁移 ＋ 本 crate 迁移 120 的临时库（与 `tests/tree.rs` 同形）。
fn new_db() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let mut migrations: Vec<Migration> = builtin_migrations();
    migrations.extend(p4_budget_migrations());
    let db = Db::open_with(&path, migrations).unwrap();
    db.migrate().unwrap();
    (dir, db)
}

/// `BudgetOwner::Intent` 的简写。
fn intent(id: &str) -> BudgetOwner {
    BudgetOwner::Intent(IntentId::from_str(id))
}

/// 五维同值。
fn all(v: i64) -> Dimensions {
    Dimensions {
        money: Some(v),
        wall_time: Some(v),
        token: Some(v),
        gpu_time: Some(v),
        network_transfer: Some(v),
    }
}

/// 只有 `d` 这一维取值 `v`，其余全 `None`。
fn only(d: DimensionKind, v: i64) -> Dimensions {
    let mut out = Dimensions {
        money: None,
        wall_time: None,
        token: None,
        gpu_time: None,
        network_transfer: None,
    };
    match d {
        DimensionKind::Money => out.money = Some(v),
        DimensionKind::WallTime => out.wall_time = Some(v),
        DimensionKind::Token => out.token = Some(v),
        DimensionKind::GpuTime => out.gpu_time = Some(v),
        DimensionKind::NetworkTransfer => out.network_transfer = Some(v),
    }
    out
}

/// 账本全部行（`owner, dimension, kind, amount`，插入序）。
fn ledger_rows(tx: &Tx<'_>) -> Vec<(String, String, String, i64)> {
    tx.query(
        "SELECT owner, dimension, kind, amount FROM budget_ledger ORDER BY rowid",
        &[],
    )
    .unwrap()
    .into_iter()
    .map(|row| match (&row[0], &row[1], &row[2], &row[3]) {
        (Value::Text(o), Value::Text(d), Value::Text(k), Value::Int(a)) => {
            (o.clone(), d.clone(), k.clone(), *a)
        }
        other => panic!("账本行的形状不符（owner/dimension/kind/amount）: {other:?}"),
    })
    .collect()
}

/// 一张表的列名（`PRAGMA table_info` 的第二列），按声明序。
fn table_columns(tx: &Tx<'_>, table: &str) -> Vec<String> {
    tx.query(&format!("PRAGMA table_info({table})"), &[])
        .unwrap()
        .into_iter()
        .map(|row| match &row[1] {
            Value::Text(name) => name.clone(),
            other => panic!("table_info 的 name 列不是文本: {other:?}"),
        })
        .collect()
}

/// 一条树：系统维护根 ＋ 一个 Intent 子节点（账本用例的基准）。
fn base_tree(tx: &Tx<'_>, sys: &BudgetOwner, child: &BudgetOwner) {
    record_root(tx, sys.clone(), all(100), 0).unwrap();
    allocate(tx, sys.clone(), child.clone(), all(50), 1).unwrap();
}

// ===== 迁移 120 =====

/// 迁移 120 是**一个**迁移项，SQL 里两张表都在。
///
/// # 为什么 `len() == 1` 是要紧的
///
/// `Db::migrate()` 以 `schema_migrations.version`（主键）判「已应用」——**同号的
/// 第二个迁移项会被静默跳过**（第二个表永不建）。同 `continuum-semantics` 的
/// `the_migration_registers_every_table_of_the_110_item` 那条 `len() == 1` 守卫。
#[test]
fn the_migration_120_is_a_single_item_with_both_tables() {
    let migrations = p4_budget_migrations();
    assert_eq!(
        migrations.len(),
        1,
        "120 应是**一个**迁移项（第二个同号项会被 `migrate()` 静默跳过，第二个表永不建）"
    );
    assert_eq!(migrations[0].version, 120);
    assert_eq!(migrations[0].name, "p4_budget");
    for table in ["budget_node", "budget_ledger"] {
        assert!(
            migrations[0].sql.contains(&format!("CREATE TABLE {table}")),
            "迁移 120 应建 `{table}` 表"
        );
    }
    assert_eq!(
        migrations[0].sql.matches("CREATE TABLE").count(),
        2,
        "此刻恰两张表（`budget_node` / `budget_ledger`，设计 §2.4 的 120 段）"
    );
}

// ===== 两张表的列清单（逐列） =====

/// 两张表**逐列**（实际列 vs 手写期望）。
///
/// # 列清单的来历（写进注释）
///
/// - `budget_ledger` 的五列 `(owner, dimension, kind, amount, at)` 是**设计 §12.2
///   写死**的；`kind ∈ {allocate, reserve, release, settle}` 落 `kind` 列（闭集，
///   编码挂在 [`LedgerKind`] 上）。
/// - `budget_node` 的列设计**没给**——本实现取：`owner`（主键）＋ `parent`（父的
///   编码；NULL = 根）＋ 五维（`money` / `wall_time` / `token` / `gpu_time` /
///   `network_transfer`，NULL = `None`）。`parent` 列是 I2 的判据来源：
///   `Σ 子节点分配额` 就是「`parent` 列等于该节点的那些行」的和。
/// - 两张表都**不建外键**（同 `continuum-semantics` 的处置：跨记录对应关系由
///   写路径的调用方保证）。
#[test]
fn the_two_budget_tables_have_exactly_the_expected_columns() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();

    assert_eq!(
        table_columns(&tx, "budget_node"),
        vec![
            "owner",
            "parent",
            "money",
            "wall_time",
            "token",
            "gpu_time",
            "network_transfer",
        ],
        "budget_node 的列清单逐列"
    );
    assert_eq!(
        table_columns(&tx, "budget_ledger"),
        vec!["owner", "dimension", "kind", "amount", "at"],
        "budget_ledger 的五列是设计 §12.2 写死的"
    );
}

// ===== 账本：只追加、余额现算 =====

/// 账本**只追加、不 UPDATE**：先前的行在任何后续操作后**逐行不变**（前缀比对），
/// 且余额是**现算**的（`remaining()` 在账本变化后立刻反映）。
///
/// # 红条件
///
/// 把余额也落成一列并在原地更新（**移除档**——那时「先前的行不变」仍成立，
/// 但 `remaining()` 与账本会漂移；故本条另断言「表里没有 `balance` 一列」，逐列）。
#[test]
fn the_ledger_is_append_only_and_balances_are_derived() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();

    let sys = BudgetOwner::SystemMaintenance;
    let owner = intent("i-1");
    base_tree(&tx, &sys, &owner);

    // 先跑一段真实操作流，快照账本。
    let r1 = reserve(&tx, owner.clone(), only(DimensionKind::Token, 7), 2)
        .ok()
        .expect("正常预扣应通过");
    settle(&tx, r1, only(DimensionKind::Token, 5), 3).unwrap();
    let snapshot = ledger_rows(&tx);
    assert!(!snapshot.is_empty(), "基准操作流应已写账");

    // 后续操作只追加：快照是新区间的前缀，逐行不变。
    let _r2 = reserve(&tx, owner.clone(), only(DimensionKind::Money, 9), 4)
        .ok()
        .expect("正常预扣应通过");
    let after = ledger_rows(&tx);
    assert!(
        after.len() > snapshot.len(),
        "后续操作应追加了新行（否则前缀比对是空断言）"
    );
    assert_eq!(
        &after[..snapshot.len()],
        &snapshot[..],
        "先前的行在任何后续操作后逐行不变（只追加、不 UPDATE）"
    );

    // 余额是现算的：账本变化后 `remaining()` 立刻反映。
    let available = remaining(&tx, &owner).unwrap();
    assert_eq!(available.money(), Some(41), "50 − 9 = 41（现算）");
    assert_eq!(available.token(), Some(45), "50 − 5 = 45（现算）");

    // 表里没有 `balance` 一列（逐列）——余额不落列。
    assert!(
        !table_columns(&tx, "budget_ledger").contains(&"balance".to_owned()),
        "账本里没有 `balance` 一列：余额是派生值，不是落列的原值"
    );
}

/// 一次 `Over` 结算后账本**只多一条 `settle` 行**。
///
/// # 否定式照片（依据写进注释）
///
/// 运行期实际超支的裁决**不在本层**：ENG-005 §三.2 把它判给
/// `FailureClass::Constraint` ＋ 该节点的 `EscalationPolicy::Decision`，两者都是
/// **执行层的词汇**（`crates/continuum-graph/src/failure.rs:8`、`:32`）——
/// **本层只回答「超了多少、哪一维」**，账上不留裁决的痕迹。
///
/// # 夹具与红条件
///
/// 预扣只有一维（token），故本条唯一涉及的那维恰写**一条** `settle` 行。
/// **红条件**：为超支写第二行账（移除档——条数断言红）。
#[test]
fn an_over_settlement_writes_no_second_ledger_row() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();

    let sys = BudgetOwner::SystemMaintenance;
    let owner = intent("i-1");
    base_tree(&tx, &sys, &owner);

    let r = reserve(&tx, owner.clone(), only(DimensionKind::Token, 7), 2)
        .ok()
        .expect("正常预扣应通过");
    let before = ledger_rows(&tx);

    settle(&tx, r, only(DimensionKind::Token, 10), 3).unwrap();

    let after = ledger_rows(&tx);
    assert_eq!(
        after.len(),
        before.len() + 1,
        "一次 Over 结算只多**一条** `settle` 行（不写第二行账）"
    );
    assert_eq!(
        &after[..before.len()],
        &before[..],
        "先前的行逐行不变"
    );
    let added = &after[after.len() - 1];
    assert_eq!(
        added,
        &(
            owner.as_key(),
            "token".to_owned(),
            "settle".to_owned(),
            10
        ),
        "新增的那一条是 settle 行：amount 是实际用量（超支的裁决不在这里）"
    );
}

// ===== 悬空预扣：可检出（两侧） =====

/// §12.2 的「半写的副作用在账本上表现为『有一条 `reserve` 而没有对应的
/// `settle` 或 `release`』，**是可检出的**」——本用例是那句绝对措辞的照片。
///
/// # 两侧
///
/// - `settle` 之后同一条不再被列出；
/// - **`release` 行之后**同一条不再被列出（本层今天没有 `release` 入口——
///   裸插一行模拟将来 `release` 的账本形态，钉的是读法的配对规则）；
/// - 两条 `reserve` 一条 `settle` 时，列出的是**未消化的那一条**（FIFO）。
///
/// # 取名与落点的申报
///
/// `dangling_reservations` 的取名与落点是**本计划自定**（设计只写「可检出」，
/// 没给函数名），且 Task 18 的恢复钩子必须消费它（§12.3 第 2 处）。
#[test]
fn a_dangling_reservation_is_detectable() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();

    let sys = BudgetOwner::SystemMaintenance;
    let owner = intent("i-1");
    base_tree(&tx, &sys, &owner);

    // (a) 造一条悬空 reserve ⇒ 列出它。
    let r = reserve(&tx, owner.clone(), only(DimensionKind::Token, 7), 2)
        .ok()
        .expect("正常预扣应通过");
    assert_eq!(
        dangling_reservations(&tx).unwrap(),
        vec![DanglingReservation {
            owner: owner.clone(),
            dimension: DimensionKind::Token,
            amount: 7,
        }],
        "悬空的那一条被列出"
    );

    // (b) settle 之后同一条不再被列出。
    settle(&tx, r, only(DimensionKind::Token, 7), 3).unwrap();
    assert!(
        dangling_reservations(&tx).unwrap().is_empty(),
        "settle 之后不再列出"
    );

    // (c) release 行之后同一条不再被列出（裸插模拟——本层没有 release 入口）。
    tx.execute(
        "INSERT INTO budget_ledger (owner, dimension, kind, amount, at) \
         VALUES (?1, 'token', 'reserve', 5, 4)",
        &[Value::text(owner.as_key())],
    )
    .unwrap();
    tx.execute(
        "INSERT INTO budget_ledger (owner, dimension, kind, amount, at) \
         VALUES (?1, 'token', 'release', 5, 5)",
        &[Value::text(owner.as_key())],
    )
    .unwrap();
    assert!(
        dangling_reservations(&tx).unwrap().is_empty(),
        "release 之后不再列出"
    );

    // (d) 两条 reserve 一条 settle：列出**未消化的那一条**（FIFO 配对）。
    tx.execute(
        "INSERT INTO budget_ledger (owner, dimension, kind, amount, at) \
         VALUES (?1, 'money', 'reserve', 10, 6)",
        &[Value::text(owner.as_key())],
    )
    .unwrap();
    tx.execute(
        "INSERT INTO budget_ledger (owner, dimension, kind, amount, at) \
         VALUES (?1, 'money', 'reserve', 20, 7)",
        &[Value::text(owner.as_key())],
    )
    .unwrap();
    tx.execute(
        "INSERT INTO budget_ledger (owner, dimension, kind, amount, at) \
         VALUES (?1, 'money', 'settle', 10, 8)",
        &[Value::text(owner.as_key())],
    )
    .unwrap();
    assert_eq!(
        dangling_reservations(&tx).unwrap(),
        vec![DanglingReservation {
            owner: owner.clone(),
            dimension: DimensionKind::Money,
            amount: 20,
        }],
        "最早的那条被消化（FIFO），列出的是未消化的 20"
    );
}

// ===== 编码纪律：小写加下划线、与类型同址 =====

/// 两个枚举列（`dimension` / `kind`）与 owner 的复合键：编码**小写加下划线**、
/// `as_str`/`parse` **与类型同址**、四/五枚**逐枚**往返。
///
/// # 逐枚（不抽代表）
///
/// 五枚量纲、四枚账本 kind 各一条断言；`parse` 认不出的串给 `None`（由调用点转
/// 具体 `Err`，见下一条）。owner 的复合键同为四变体往返——含**能含任意字符**的
/// id（不裁剪、不转义丢失：JSON 承载的转义由 serde_json 保证）。
#[test]
fn the_enum_codes_use_lowercase_underscore_and_parse_back() {
    for (kind, code) in [
        (DimensionKind::Money, "money"),
        (DimensionKind::WallTime, "wall_time"),
        (DimensionKind::Token, "token"),
        (DimensionKind::GpuTime, "gpu_time"),
        (DimensionKind::NetworkTransfer, "network_transfer"),
    ] {
        assert_eq!(kind.as_str(), code);
        assert_eq!(DimensionKind::parse(code), Some(kind));
    }
    assert_eq!(DimensionKind::parse("Money"), None, "编码是小写——大写不认");
    assert_eq!(DimensionKind::parse("wall-时间"), None, "表外取值不猜");

    for (kind, code) in [
        (LedgerKind::Allocate, "allocate"),
        (LedgerKind::Reserve, "reserve"),
        (LedgerKind::Release, "release"),
        (LedgerKind::Settle, "settle"),
    ] {
        assert_eq!(kind.as_str(), code);
        assert_eq!(LedgerKind::parse(code), Some(kind));
    }
    assert_eq!(LedgerKind::parse("ALLOCATE"), None);

    // owner 的复合键：四变体逐枚往返（id 里带分隔符/空格也原样）。
    let owners = [
        BudgetOwner::SystemMaintenance,
        BudgetOwner::Intent(IntentId::from_str("i:1 带 空格")),
        BudgetOwner::NodeRun {
            intent: IntentId::from_str("i:1"),
            node: "n:1".to_owned(),
        },
        BudgetOwner::Exploration {
            intent: IntentId::from_str("i:1"),
            of_node: "n:1".to_owned(),
        },
    ];
    for owner in &owners {
        let key = owner.as_key();
        assert_eq!(
            BudgetOwner::parse(&key).as_ref(),
            Some(owner),
            "owner 的键四变体逐枚往返（含含特殊字符的 id）"
        );
    }
    assert_eq!(BudgetOwner::parse("banana"), None, "表外的 owner 键给 None");
    assert_eq!(BudgetOwner::parse("{}"), None, "缺 kind 的键给 None");
}

/// 表外取值读回**具体 `Err`**、**带原始串、不取默认值**（三列各一条）。
///
/// # 落点申报（`BudgetError` 的「恰好三枚」与持久化失败）
///
/// 设计的 **§12.2.1 写死 `BudgetError` 的变体「恰好三枚」**（三个**语义**失败），
/// 而本层的每个入口都要读库——持久化失败若被降级成三枚里的任何一枚，调用方会
/// 以为「数据就是这样」（本仓对 `Persist` 枚一贯的判据，见 `IntentError::Persist`
/// 一族）。故本实现比 §12.2.1 多一枚 **`Persist(#[from] PersistError)`**：
/// 它**不属于**那份「预算层失败」清单（那三枚语义枚一枚不改），是持久层的失败按
/// 本仓一律的处置转出。**这三条用例正是它的照片**：表外取值不被静默映射、
/// 不被猜成某一枚。
///
/// # 红条件
///
/// 表外取值被静默映射成某一枚（**放宽档**）；读回 `Ok` 或不点名原串即红。
#[test]
fn out_of_table_values_read_back_as_specific_errors() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();

    let owner = intent("i-1");
    record_root(&tx, owner.clone(), all(10), 0).unwrap();
    let key = owner.as_key();

    // (a) `dimension` 列的表外取值。
    tx.execute(
        "INSERT INTO budget_ledger (owner, dimension, kind, amount, at) \
         VALUES (?1, 'banana', 'reserve', 1, 1)",
        &[Value::text(key.clone())],
    )
    .unwrap();
    match dangling_reservations(&tx) {
        Err(BudgetError::Persist(PersistError::Database(message))) => {
            assert!(
                message.contains("banana"),
                "错误消息应带库里那一个原始串，实得: {message}"
            );
        }
        other => panic!("表外的 dimension 取值应读回具体 Err，实得: {other:?}"),
    }
    tx.execute("DELETE FROM budget_ledger WHERE dimension = 'banana'", &[])
        .unwrap();

    // (b) `kind` 列的表外取值。
    tx.execute(
        "INSERT INTO budget_ledger (owner, dimension, kind, amount, at) \
         VALUES (?1, 'token', 'burn', 1, 1)",
        &[Value::text(key.clone())],
    )
    .unwrap();
    match dangling_reservations(&tx) {
        Err(BudgetError::Persist(PersistError::Database(message))) => {
            assert!(
                message.contains("burn"),
                "错误消息应带库里那一个原始串，实得: {message}"
            );
        }
        other => panic!("表外的 kind 取值应读回具体 Err，实得: {other:?}"),
    }
    tx.execute("DELETE FROM budget_ledger WHERE kind = 'burn'", &[])
        .unwrap();

    // (c) `owner` 列的表外取值（复合键坏了）。
    tx.execute(
        "INSERT INTO budget_ledger (owner, dimension, kind, amount, at) \
         VALUES ('{\"kind\":\"mystery\"}', 'token', 'reserve', 1, 1)",
        &[],
    )
    .unwrap();
    match dangling_reservations(&tx) {
        Err(BudgetError::Persist(PersistError::Database(message))) => {
            assert!(
                message.contains("mystery"),
                "错误消息应带库里那一个原始串，实得: {message}"
            );
        }
        other => panic!("表外的 owner 取值应读回具体 Err，实得: {other:?}"),
    }
}
