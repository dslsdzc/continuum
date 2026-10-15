//! Task 18 的「无读数即按预扣」**第一处**的照片（§12.3：一条规则、两处应用）。
//!
//! 第二处（悬空的预留——恢复钩子）在 `tests/recovery.rs`；本文件钉两条：
//!
//! | 用例 | 钉的判据 | 红条件（档位） |
//! |---|---|---|
//! | `a_streaming_call_without_a_reading_settles_at_the_reserved_amount` | 流式调用（G §14 第 8 条）**没有读数**：结算后 `settled == reserved` 且 `reserved == 0`（**余量不退**）；账本里**没有** `release` 行 | 把无读数结算实现成「释放预扣」（写 `release` 行、`settled` 不涨）——**移除档**，本用例红 |
//! | `a_call_with_a_reading_settles_at_the_actual_amount` | **另一侧，不能省**：有读数时 `settled == actual`、余量**退回** | 把结算实现成**恒按预扣**（**移除档**，本用例红）——只钉「无读数按预扣」那侧会让「一律按预扣」的实现照样绿 |
//!
//! # 为什么两处各有一条各自的照片
//!
//! §12.3 的规则是「一次执行如果没有实际读数，就按预扣额结算；不退余量」，
//! 两处应用（流式调用 / 悬空预留）的**理由一致**：无读数时「实际 ≤ 预扣」没有
//! 依据；按预扣结算**永不低记**（fail-closed），而释放余量是 fail-open（可能
//! 永久漏记一次真实消耗）。**代价写明**：连续多次无读数的执行会把额度用光，
//! 而实际可能没花那么多——这是一个**已知的保守偏差**，不是缺陷。
//!
//! # 夹具
//!
//! `new_db()` 与 `tests/tree.rs` / `tests/persist.rs` 同形（P0 内建迁移 ＋ 本 crate
//! 迁移 120）；树从 `record_root` 起。

use continuum_budget::{
    BudgetOwner, DimensionKind, Dimensions, SettleOutcome, dangling_reservations,
    p4_budget_migrations, record_root, remaining, reserve, settle, settle_without_a_reading,
};
use continuum_persist::{Db, Migration, Tx, Value, builtin_migrations};
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

/// 五维的固定顺序（与 `src/dimensions.rs` 的枚举序一致）。
const FIVE: [DimensionKind; 5] = [
    DimensionKind::Money,
    DimensionKind::WallTime,
    DimensionKind::Token,
    DimensionKind::GpuTime,
    DimensionKind::NetworkTransfer,
];

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

/// 按维读一份 `Dimensions`。
fn dim_of(dims: &Dimensions, kind: DimensionKind) -> Option<i64> {
    match kind {
        DimensionKind::Money => dims.money,
        DimensionKind::WallTime => dims.wall_time,
        DimensionKind::Token => dims.token,
        DimensionKind::GpuTime => dims.gpu_time,
        DimensionKind::NetworkTransfer => dims.network_transfer,
    }
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

/// 该 owner 的 `settle` 行按五维汇总。
fn settled_by_dimension(tx: &Tx<'_>, owner: &str) -> [i64; 5] {
    let mut out = [0i64; 5];
    for (o, d, k, amount) in ledger_rows(tx) {
        if o == owner && k == "settle" {
            let i = FIVE
                .iter()
                .position(|kind| kind.as_str() == d)
                .unwrap_or_else(|| panic!("表外的 dimension 取值: {d}"));
            out[i] += amount;
        }
    }
    out
}

// ===== §12.3 第 1 处：流式调用没有读数 =====

/// **流式调用**（G §14 第 8 条：`StreamChunk` 只有 `delta` / `done`，没有 usage）
/// 没有读数——结算后 `settled == reserved` **且** `reserved == 0`（余量**不退**）。
///
/// # 三条断言各自钉什么
///
/// - `settled == reserved`：账本 `settle` 行逐维等于预扣额（不是 0、也不是别的数）；
/// - `reserved == 0`：`dangling_reservations` **空**（悬空检出器读回「没有未消化的
///   预扣」——它是 `reserved > 0` 的直接公开读法）；
/// - 余量不退是**合并效果**：`remaining == allocation − 预扣额`（不是 `allocation`）。
///
/// 外加一条形态断言：账本里**没有 `release` 行**（本层没有任何释放入口——§12.3
/// 判「释放余量是 fail-open」；`release` 是设计写死的账本值域，但**没有产生方**）。
#[test]
fn a_streaming_call_without_a_reading_settles_at_the_reserved_amount() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();

    let owner = intent("i-stream");
    record_root(&tx, owner.clone(), all(1000), 0).unwrap();

    // 一次流式调用的预扣：五维各 100（流式连 `Usage` 都没有——G §14 第 8 条）。
    let reservation = reserve(&tx, owner.clone(), all(100), 1).unwrap();
    assert!(
        !dangling_reservations(&tx).unwrap().is_empty(),
        "上半段的前提：预扣在飞（否则下面的结算没有东西可消化）"
    );

    // 无读数结算：五维都按预扣。
    match settle_without_a_reading(&tx, reservation, 2).unwrap() {
        SettleOutcome::Within { refunded } => {
            for kind in FIVE {
                assert_eq!(
                    dim_of(&refunded, kind),
                    Some(0),
                    "（{kind:?}）无读数的维退 0——余量不退"
                );
            }
        }
        other => panic!("无读数结算的结局是 `Within`：{other:?}"),
    }

    // (a) `settled == reserved`：账本 settle 行逐维 == 预扣额 100。
    assert_eq!(
        settled_by_dimension(&tx, owner.as_key().as_str()),
        [100, 100, 100, 100, 100],
        "settled 逐维 == 预扣额（无读数按预扣）"
    );

    // (b) `reserved == 0`：结算把预扣消化掉之后不许有悬空残留。
    assert!(
        dangling_reservations(&tx).unwrap().is_empty(),
        "settle 之后 `reserved == 0`——悬空检出器必须读回空"
    );

    // (c) 余量按预扣扣掉、不退。
    let r = remaining(&tx, &owner).unwrap();
    for kind in FIVE {
        assert_eq!(
            dim_of(r.dims(), kind),
            Some(900),
            "（{kind:?}）remaining == allocation(1000) − 预扣(100)——余量不退"
        );
    }

    // (d) 形态：没有 `release` 行（无读数按预扣**不是**释放）。
    let rows = ledger_rows(&tx);
    assert_eq!(
        rows.iter().filter(|(_, _, k, _)| k == "release").count(),
        0,
        "本层没有释放入口——§12.3 判「释放余量是 fail-open」；账本里不许出现 release 行"
    );
    assert_eq!(
        rows.iter().filter(|(_, _, k, _)| k == "settle").count(),
        5,
        "五维各一条 settle 行（预扣的 Some 维逐维记账）"
    );
    assert_eq!(rows.len(), 15, "5 allocate ＋ 5 reserve ＋ 5 settle，一行不多");
}

// ===== §12.3 的另一侧：有读数时**不**按预扣 =====

/// **另一侧，不能省**：有读数时 `settled == actual`、余量**退回**。
///
/// 只钉「无读数按预扣」那侧，会让「**一律按预扣**」的实现（把实际读数整个丢掉）
/// 照样绿——本用例就是它的红条件（**移除档**：结算恒按预扣 ⇒ 这里的
/// `settled == 30` 与 `remaining == 970` 全红）。
#[test]
fn a_call_with_a_reading_settles_at_the_actual_amount() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();

    let owner = intent("i-measured");
    record_root(&tx, owner.clone(), all(1000), 0).unwrap();
    let reservation = reserve(&tx, owner.clone(), all(100), 1).unwrap();

    // 有读数：五维各有实际用量 30（远小于预扣 100）。
    match settle(&tx, reservation, all(30), 2).unwrap() {
        SettleOutcome::Within { refunded } => {
            for kind in FIVE {
                assert_eq!(
                    dim_of(&refunded, kind),
                    Some(70),
                    "（{kind:?}）退回额 == 预扣 − 实际"
                );
            }
        }
        other => panic!("实际 < 预扣的结局是 `Within`：{other:?}"),
    }

    // `settled == actual`：账本 settle 行逐维 == 30（不是预扣 100）。
    assert_eq!(
        settled_by_dimension(&tx, owner.as_key().as_str()),
        [30, 30, 30, 30, 30],
        "有读数时 settled == actual——一律按预扣的实现在这里红"
    );

    // 余量退回：remaining == allocation − actual。
    let r = remaining(&tx, &owner).unwrap();
    for kind in FIVE {
        assert_eq!(
            dim_of(r.dims(), kind),
            Some(970),
            "（{kind:?}）remaining == allocation(1000) − actual(30)——余量退回"
        );
    }
    assert!(
        dangling_reservations(&tx).unwrap().is_empty(),
        "结算消化预扣之后 `reserved == 0`"
    );
}
