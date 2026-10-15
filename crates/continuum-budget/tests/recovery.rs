//! Task 18 的「无读数即按预扣」**第二处**（§12.3：悬空的预留——进程在预留与
//! 结算之间死掉）与 §13.1 恢复钩子的两个否定式照片。
//!
//! | 用例 | 钉的判据 | 红条件（档位） |
//! |---|---|---|
//! | `a_dangling_reserve_is_settled_at_the_reserved_amount_by_the_recovery_hook` | 钩子把悬空的 `reserve` 按预扣结算：`settled == reserved` 且 `reserved == 0`；**并断言 `hook.phase()` 是 `MarkLostExecutions`**（不是别的态） | 钩子改成「释放悬空预扣」（**移除档**——账本里会出现 `release` 行、`settled` 不涨） |
//! | `every_dangling_reserve_is_settled_one_by_one_with_the_reserve_rows_own_at` | **N≥2 悬空**（同一 `(owner, money)` 两条）：钩子**逐条**写 `settle` 行（一条「和」的行在条数上留悬空）；且每条 `settle` 行的 `at` **沿用**它所消化那条 `reserve` 行自己的 `at`（钩子不收时钟） | 按「和」写一条 `settle` 行（**移除档**——收敛断言留残留）；`at` 取新时钟／别的值（**取反档**——逐条断言红） |
//! | `the_recovery_hook_does_not_repeat_what_mark_running_nodes_lost_did` | **否定式照片**：跑一次钩子后 `adfir_node` 表的**行内容逐行不变**——本层只读「哪些运行已 LOST」这个既成事实、只记账，**不写节点状态** | 钩子在结算之外顺手写了节点表（**取反档**——「行内容逐行不变」红） |
//! | `the_hook_is_not_registered_on_reconcile_incomplete_effects` | 签名层面 ＋ 一条否定式断言：`phase()` **不是** `ReconcileIncompleteEffects`（§13.1 的四步判据把三个备选逐条排除） | 挂到效应阶段（**取反档**）——那会让一处「效应语义」的改动波及预算记账 |
//!
//! # 钩子位置的来历（§13.1 的四步判据，写进代码注释的同一份）
//!
//! 挂 `MarkLostExecutions` 而**不是** `ReconcileRunningNodes`：后者已有的钩子
//! `MarkRunningNodesLost` **正是产出「哪些运行已 LOST」这个事实的那一步**，
//! 本层的结算要**消费**那个事实；两者挂同一阶段会让正确性依赖**阶段内的注册
//! 顺序**，而那不是一份被文档化的契约。挂到下一阶段就把这个依赖消掉：那时
//! LOST 已经落了库。**本层不重复 `MarkRunningNodesLost` 已经做掉的那一半**：
//! 那一步做的是**把节点状态标成 LOST**（`continuum_graph::mark_running_nodes_lost`），
//! 本层做的是**读那个既成事实、把悬空的 `reserve` 按预扣结算**——一个写状态、
//! 一个记账，不重不漏。（点名说「不是本层的事」；**本 crate 不 `use`
//! `continuum_graph`**——依赖边只有 `continuum-persist` 与 `continuum-semantics`
//! 两条。）
//!
//! # `adfir_node` 照片的自建申报
//!
//! 本 crate 不依赖 `continuum-graph`（上段），故本测试**自建一张同名的最小形状表**
//! （只在临时库里、由测试自己的迁移建出）——要钉的是「本层**一个字节都不写它**」
//! 这个事实；表本身的形状不是判据。**并配一条「钩子真的干活了」的断言**（悬空
//! 被结算、`settle` 行出现）——否则「表不变」可以来自「钩子什么都没干」这类
//! 与判据无关的坏法（恒绿的守卫比没有更坏）。
//!
//! # 注册不在本 task
//!
//! 钩子在此**直接以 `run(tx)` 调用**（它本来的用法：由 `RecoveryRegistry` 在
//! 阶段事务内调用）。**装配（注册进 `RecoveryRegistry`）是 Task 20 的项**。

use continuum_budget::{
    BudgetOwner, DimensionKind, Dimensions, BudgetReservationRecovery, dangling_reservations,
    p4_budget_migrations, record_root, remaining, reserve, settle,
};
use continuum_persist::{
    Db, Migration, RecoveryHook, RecoveryPhase, Tx, Value, builtin_migrations,
};
use continuum_semantics::IntentId;

/// 建一个临时库：P0 内建迁移 ＋ 本 crate 迁移 120 ＋ **本测试自己的**
/// 「同名最小 `adfir_node`」夹具迁移。
///
/// 夹具迁移的 version 取 **999**：它活在**这个测试自己的库**里，不进任何装配链
/// （同 `continuum-persist/tests/migrations.rs` 的 100 / 101 / 102 先例）。
fn new_db() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let mut migrations: Vec<Migration> = builtin_migrations();
    migrations.extend(p4_budget_migrations());
    migrations.push(Migration::new(
        999,
        "test_adfir_node_fixture",
        "CREATE TABLE adfir_node (
            node_id    TEXT PRIMARY KEY NOT NULL,
            state      TEXT NOT NULL
        );",
    ));
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

/// 该 owner 的账本行（判「已消化的 owner 不被钩子动」用）。
fn rows_of(tx: &Tx<'_>, owner: &str) -> Vec<(String, String, String, i64)> {
    ledger_rows(tx)
        .into_iter()
        .filter(|(o, _, _, _)| o == owner)
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

/// 该 owner 的 `settle` 行，按插入序：`(维度, 额, at)` 三列。
///
/// （`at` 是「`settle` 行沿用它所消化那条 `reserve` 行自己的 `at`」这一判据的
/// 唯一可读列。）
fn settle_rows(tx: &Tx<'_>, owner: &str) -> Vec<(String, i64, i64)> {
    tx.query(
        "SELECT dimension, amount, at FROM budget_ledger \
         WHERE owner = ?1 AND kind = 'settle' ORDER BY rowid",
        &[Value::text(owner)],
    )
    .unwrap()
    .into_iter()
    .map(|row| match (&row[0], &row[1], &row[2]) {
        (Value::Text(d), Value::Int(a), Value::Int(at)) => (d.clone(), *a, *at),
        other => panic!("settle 行的形状不符（dimension/amount/at）: {other:?}"),
    })
    .collect()
}

/// `adfir_node` 的全部行（`node_id, state`，按 `node_id` 排序）。
fn adfir_rows(tx: &Tx<'_>) -> Vec<(String, String)> {
    tx.query(
        "SELECT node_id, state FROM adfir_node ORDER BY node_id",
        &[],
    )
    .unwrap()
    .into_iter()
    .map(|row| match (&row[0], &row[1]) {
        (Value::Text(id), Value::Text(state)) => (id.clone(), state.clone()),
        other => panic!("adfir_node 行的形状不符（node_id/state）: {other:?}"),
    })
    .collect()
}

// ===== §12.3 第 2 处：悬空的预留 =====

/// **悬空的预留**（进程在预留与结算之间死掉）——钩子按预扣结算，**不释放**。
///
/// 三条断言（与第一处同形）：`settled == reserved`（账本 `settle` 行逐维 == 预扣额）、
/// `reserved == 0`（悬空检出器读回空）、`remaining == allocation − 预扣额`。
/// **并断言 `hook.phase()` 是 `RecoveryPhase::MarkLostExecutions`**（不是别的态）。
///
/// 另有一个**对照 owner**（预扣已正常结算）：跑钩子后它的账本行**一行不变**——
/// 钩子只动悬空的那部分，不重结算已消化的预扣。
#[test]
fn a_dangling_reserve_is_settled_at_the_reserved_amount_by_the_recovery_hook() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();

    // 悬空的 owner：预扣了 100、进程死掉，没结算。
    let dangling_owner = intent("i-crashed");
    record_root(&tx, dangling_owner.clone(), all(1000), 0).unwrap();
    reserve(&tx, dangling_owner.clone(), all(100), 1).unwrap();

    // 对照 owner：预扣后正常结算（已消化的预扣）。
    let digested_owner = intent("i-fine");
    record_root(&tx, digested_owner.clone(), all(1000), 0).unwrap();
    let digested_reservation = reserve(&tx, digested_owner.clone(), all(100), 1).unwrap();
    settle(&tx, digested_reservation, all(50), 2).unwrap();
    let digested_rows_before = rows_of(&tx, digested_owner.as_key().as_str());

    // 直接调钩子（注册是 Task 20 的装配项）。
    let hook = BudgetReservationRecovery;
    assert_eq!(
        hook.phase(),
        RecoveryPhase::MarkLostExecutions,
        "钩子挂在 `MarkLostExecutions`——不是别的态"
    );
    hook.run(&tx).unwrap();

    // (a) `settled == reserved`：悬空 owner 的 settle 行逐维 == 100。
    assert_eq!(
        settled_by_dimension(&tx, dangling_owner.as_key().as_str()),
        [100, 100, 100, 100, 100],
        "悬空预扣按预扣额结算（不是释放、不是 0）"
    );

    // (b) `reserved == 0`：悬空清零。
    assert!(
        dangling_reservations(&tx).unwrap().is_empty(),
        "钩子跑完之后没有悬空残留（含另一个 owner 的那份也没有被误结算成新的悬空）"
    );

    // (c) remaining == allocation − 预扣额（余量不退）。
    let r = remaining(&tx, &dangling_owner).unwrap();
    for kind in FIVE {
        assert_eq!(
            dim_of(r.dims(), kind),
            Some(900),
            "（{kind:?}）remaining == 1000 − 100——余量不退"
        );
    }

    // (d) 对照 owner 的账本一行不变（钩子不重结算已消化的预扣）。
    assert_eq!(
        rows_of(&tx, digested_owner.as_key().as_str()),
        digested_rows_before,
        "已消化的预扣不许被钩子再结算一遍"
    );
}

/// **N≥2 悬空 ＋ `settle` 行沿用 `reserve` 行自己的 `at`**（终审 Minor 12，两件事一条用例）。
///
/// # 为什么是同一 `(owner, 维度)` 上的两条
///
/// 这是「逐条结算」与「按和结算」分岔的唯一位置：只写一条和为 300 的 `settle` 行
/// 在**条数**上仍留一条 `reserve` 悬着（`balances` 的消化按条数配对）、
/// `dangling_reservations` 读到残留——下面的收敛断言因此有红法。两条悬空若落在两个
/// 维度上，两种实现都各写两条行，这个分岔消失。
///
/// # 为什么 `at` 也要钉
///
/// 恢复钩子不收时钟（`RecoveryHook::run` 没有 `at` 入参）：它写的 `settle` 行的时刻是
/// **被消化那条 `reserve` 行自己的 `at`**（`src/ledger.rs` 的 `undigested_reserve_rows`）。
/// 同维度、金额可相同时，`at` 是能把 `settle` 行对回它所消化那条 `reserve` 行的列。
#[test]
fn every_dangling_reserve_is_settled_one_by_one_with_the_reserve_rows_own_at() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();

    // 同一 owner、同一维度（money）上两条悬空预扣，`at` 各不相同（7、9）。
    let owner = intent("i-two-dangling");
    record_root(&tx, owner.clone(), all(1000), 0).unwrap();
    reserve(
        &tx,
        owner.clone(),
        Dimensions {
            money: Some(100),
            wall_time: None,
            token: None,
            gpu_time: None,
            network_transfer: None,
        },
        7,
    )
    .unwrap();
    reserve(
        &tx,
        owner.clone(),
        Dimensions {
            money: Some(200),
            wall_time: None,
            token: None,
            gpu_time: None,
            network_transfer: None,
        },
        9,
    )
    .unwrap();

    // 前提：两条都悬空（同 (owner, money) 聚合后恰一条、额 300）。
    let dangling = dangling_reservations(&tx).unwrap();
    assert_eq!(dangling.len(), 1, "同 (owner, money) 的悬空聚合为一条");
    assert_eq!(dangling[0].amount, 300, "两条悬空的额合计 300");

    let hook = BudgetReservationRecovery;
    hook.run(&tx).unwrap();

    // (a) 逐条：恰两条 `settle` 行（不是一条和 300 的行）——额与 `at` 各自回指
    // 它消化的那条 `reserve` 行（插入序：100@7、200@9）。
    assert_eq!(
        settle_rows(&tx, owner.as_key().as_str()),
        vec![
            ("money".to_owned(), 100, 7),
            ("money".to_owned(), 200, 9),
        ],
        "每条 reserve 行对应一条同额 settle 行，且 settle 行的 at 沿用 reserve 行自己的"
    );

    // (b) 一次恢复即收敛：按和结算的实现会在这里留一条悬空。
    assert!(
        dangling_reservations(&tx).unwrap().is_empty(),
        "逐条结清后没有悬空残留"
    );
    assert_eq!(
        settled_by_dimension(&tx, owner.as_key().as_str()),
        [300, 0, 0, 0, 0],
        "money 维的 settle 合计 = 100 + 200"
    );

    // (c) 余量：1000 − 300。
    let r = remaining(&tx, &owner).unwrap();
    assert_eq!(
        dim_of(r.dims(), DimensionKind::Money),
        Some(700),
        "remaining == 1000 − (100 + 200)——按预扣结算、余量不退"
    );
}

// ===== 否定式照片：本层不重复 MarkRunningNodesLost 的那一半 =====

/// **否定式照片**：跑一次钩子后 `adfir_node` 表的**行内容逐行不变**。
///
/// # 这条判据的来历（照 brief 要求写进注释）
///
/// 本层挂钩子的位置之所以是 `MarkLostExecutions` 而非 `ReconcileRunningNodes`，
/// 正是为了**读得到**「LOST 已落库」这个事实（§13.1 的四步判据第 2 条：
/// 同阶段会让正确性依赖阶段内的注册顺序，挂下一阶段把依赖消掉）。而
/// `MarkRunningNodesLost` 做的是**把节点标 `LOST`**——那一步写状态，**本层记账**，
/// 两者不重不漏。本层今天读的是 `budget_ledger`（悬空检出），**不读也不写**
/// 节点表；这条照片把「不写」钉成事实。
///
/// # 两条断言互相兜底
///
/// （1）**钩子真的干活了**：悬空被结算、`settle` 行出现——否则「表不变」
/// 可以来自「钩子什么都没干」；（2）**节点表逐行不变**。
#[test]
fn the_recovery_hook_does_not_repeat_what_mark_running_nodes_lost_did() {
    let (_dir, db) = new_db();

    // 铺「哪些运行已 LOST」这个既成事实（恢复链上游的产物）。
    let tx = db.begin().unwrap();
    for (node_id, state) in [("n-1", "LOST"), ("n-2", "RUNNING"), ("n-3", "COMPLETED")] {
        tx.execute(
            "INSERT INTO adfir_node (node_id, state) VALUES (?1, ?2)",
            &[Value::text(node_id), Value::text(state)],
        )
        .unwrap();
    }
    tx.commit().unwrap();

    // 悬空的预留（钩子有活可干）。
    let tx = db.begin().unwrap();
    let owner = intent("i-lost");
    record_root(&tx, owner.clone(), all(1000), 0).unwrap();
    reserve(&tx, owner.clone(), all(100), 1).unwrap();

    let nodes_before = adfir_rows(&tx);
    assert_eq!(nodes_before.len(), 3, "夹具前提：三行");

    let hook = BudgetReservationRecovery;
    hook.run(&tx).unwrap();

    // (1) 钩子真的干活了：悬空被按预扣结算。
    assert_eq!(
        settled_by_dimension(&tx, owner.as_key().as_str()),
        [100, 100, 100, 100, 100],
        "钩子必须真的结了悬空——否则下面的「不变」可能是没干活"
    );
    assert!(dangling_reservations(&tx).unwrap().is_empty());

    // (2) 节点表**行内容逐行不变**——本层只记账、不写节点状态。
    assert_eq!(
        adfir_rows(&tx),
        nodes_before,
        "本层不重复 `MarkRunningNodesLost` 已经做掉的那一半：节点表一个字节都不许动"
    );
}

// ===== 否定式：不挂效应阶段 =====

/// `phase()` **不是** `ReconcileIncompleteEffects`（签名层面 ＋ 一条否定式断言），
/// 并顺带把 §13.1 的另外两个备选逐条排除：不是 `ReconcileRunningNodes`、
/// 不是 `ResumeEligibleTasks`（**三个备选逐条排除，不是只挑一个**）。
///
/// # 依据（写进注释）
///
/// 效应阶段的钩子 `MarkExecutingAsUnknown` 处理的是**效应**——把停在 `EXECUTING`
/// 的外部效应标 `UNKNOWN`（`crates/continuum-effect/src/recovery.rs:47-49`）。
/// 而悬空的**预算预留不是一次效应**：§12.3 的触发是「**这次执行没有读数**」，
/// 与「外部做没做成不知道」是两件事。把预算结算挂进效应阶段，会让一处
/// 「效应语义」的改动波及预算记账（反之亦然）。
///
/// 不是 `ReconcileRunningNodes`：那是 `MarkRunningNodesLost` 产出现成事实的地方，
/// 同阶段会让正确性依赖阶段内的注册顺序（见用例文档）。
/// 不是 `ResumeEligibleTasks`：那时额度已被后续任务读走，结算太晚。
#[test]
fn the_hook_is_not_registered_on_reconcile_incomplete_effects() {
    let hook = BudgetReservationRecovery;

    // 「签名层面」：`phase` 的返回类型由 `RecoveryHook` 固定为 `RecoveryPhase`
    // （编译期命题——换了类型本行就编译不过）。
    let phase: RecoveryPhase = hook.phase();

    assert_ne!(
        phase,
        RecoveryPhase::ReconcileIncompleteEffects,
        "预算结算不是效应：`MarkExecutingAsUnknown` 管的是「外部做没做成不知道」，\
         而悬空预扣的触发是「这次执行没有读数」——两件事"
    );
    assert_ne!(
        phase,
        RecoveryPhase::ReconcileRunningNodes,
        "那一步（`MarkRunningNodesLost`）正在产出「哪些运行已 LOST」这个事实；\
         同阶段会让正确性依赖阶段内的注册顺序——本钩子挂下一阶段去读它"
    );
    assert_ne!(
        phase,
        RecoveryPhase::ResumeEligibleTasks,
        "那时额度已被后续任务读走，结算太晚"
    );
}
