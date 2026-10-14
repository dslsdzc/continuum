//! Task 17 的运行期用例集（`tests/persist.rs` 是它的账本与表形状半边）：
//! 三角色三类型的运行期侧、I1／I2／I3 三条不变量的违规照片、结算的两支、
//! 四个吃 `Dimensions` 的入口里本 task 落地的三个（`check` 归 Task 18），
//! 以及 `BudgetOwner` 四变体的穷尽解构。
//! 设计 §12.1／§12.2／§12.2.1；规范 §333；ENG-005 §三。
//!
//! # 编译期用例不在这里
//!
//! 「三个角色各自私有、无公开构造函数」与「三者互不是同一个类型」是**编译期**命题
//! （trybuild **六份**样例 ＋ **三份**正控制），落在 `tests/type_level.rs` 与
//! `tests/compile_fail/` / `tests/compile_pass/`。**那一组就是 plan 点名的
//! `the_three_roles_are_three_types` 与 `each_role_has_exactly_one_constructor_and_no_second_public_path`**
//! ——它们没有运行期照片（纯数据声明），本文件不再重复它们。
//!
//! # 本文件钉的每一件，以及它的红条件
//!
//! | 用例 | 钉的判据 | 红条件（档位） |
//! |---|---|---|
//! | `dimension_none_is_not_some_zero` | 五维**各一条**：`None`（该量纲不构成约束）与 `Some(0)`（额度为零）可区分，**含 `None → None`**；另附约束侧一条（`None` 维的预扣放行、`Some(0)` 维的预扣拒） | 把 `Option<i64>` 换成默认 `0`（**移除档**）；或某条夹具里两者恰好同值（那时它是等价变异体——故五条夹具的输出必须不同） |
//! | `the_five_dimension_names_are_recorded_verbatim` | §333 的五个字段名逐项（穷尽解构，**不带 `..`**）：`money` / `wall_time` / `token` / `gpu_time` / `network_transfer`；字段可构造（`pub`） | 改一个字段名/加删一个字段即**编译不过** |
//! | `reserve_reports_the_specific_dimension` | **逐维五条**：某一维不足时 `Err(Insufficient { dimension })` 且 `dimension` **是那一维** | 返回笼统的 `Exceeds`（**取反档**——五条全红） |
//! | `i1_a_reserve_beyond_the_available_amount_is_rejected` | `available(n,d) ≥ 0` 的违规（fail-closed 侧）＋ 账本未变 | 预扣不检查可用额（**放宽档**）；失败留半写账（同断言） |
//! | `i2_is_enforced_at_allocation_time` | 超分配在 `allocate` 那一刻被拒；`dimension` 是超的那一维；**失败不落树不落账** | 把 I2 挪到 `reserve` 才判（**取反档**——超分配静默进树） |
//! | `i3_settled_plus_reserved_beyond_the_allocation_is_rejected` | `settled + reserved ≤ allocation` 的违规（在既有 `settled` 参与后）被拒 | 只看 `reserved` 不看 `settled`（**放宽档**） |
//! | `reserve_rejects_a_negative_dimension` / `allocate_...` / `settle_...` | 三个入口各一条：含负值的 `Dimensions` ⇒ `Err(NegativeAmount { dimension, value })`，且**各断言账本未变** | 某一入口按 `0` 截断或放行（**移除档**——那一条红、其余条仍绿；这正是逐入口各一条的理由） |
//! | `record_root_rejects_a_negative_dimension` | **第五个**吃 `Dimensions` 的入口（本 task 落地的根入账函数）：同一口径拒负值、**不进树不落账** | 负值放行（**移除档**） |
//! | `reserve_accepts_an_all_zero_dimension` / `allocate_...` / `settle_...` | 各维为 `0` **不是** `Err`（`0` 不是负值）——守卫的另一侧 | `0` 也被拒（**收紧档**——三条全红） |
//! | `settle_within_refunds_the_unused_amount` | `Within { refunded }`：`refunded` 逐维是差额 | 不退款/退错维（同断言） |
//! | `settle_over_reports_the_dimension_and_the_amount` | `Over { dimension, by }`：**是那一维、是那个数** | 笼统的「超了」（**取反档**） |
//! | `a_negative_actual_is_rejected_and_the_ledger_is_untouched` | §12.2 照片 (a)：负 `actual` ⇒ 具体 `Err` ＋ 账本**三件事**都未动（`reserve` 仍活跃、`settled` 逐维、账本行数） | 按 `0` 截断（**移除档**——返回 `Ok` 且账面被悄悄改小） |
//! | `a_zero_actual_settles_within_and_refunds_everything` | 反例（另一侧）：全 `0` 的 `actual` ⇒ `Within` 且**全额**退回 | 把 `0` 也拒（**收紧档**） |
//! | `the_tree_is_one_tree_with_five_owner_variants` | `BudgetOwner` 四个变体逐枚穷尽解构（不带 `..`）＋ 否定式照片：`src/` 里 `SystemMaintenance` 的出现点**逐条等于白名单**（全在类型/编解码层，没有写进表的点） | 加一个变体（编译不过）；新增出现点（清单对不上） |
//!
//! # 账本行语义（本 task 自定，申报）
//!
//! §12.2 只给了账本的五列 `(owner, dimension, kind, amount, at)` 与四个 `kind`，
//! **没给每一行 `amount` 的语义**。本实现写死为**名义量**：
//!
//! - `allocate` 行 = 该次分给该维的额度；`reserve` 行 = 该次预扣额；
//!   `settle` 行 = 该次结算的**实际用量**（无读数的维写预扣额——§12.3 按预扣结算）；
//! - `reserved`（在飞的预扣）由**行数配对**得出：每 `(owner, dimension)` 上，
//!   与「`settle` ＋ `release` 行条数」等量的**最早** `reserve` 行视为已消化（FIFO），
//!   其余 `reserve` 行的额之和 = `reserved`；
//! - **限度**：账本没有 `reservation_id` 列（设计给的五列写死），配对付不出
//!   「哪条 `reserve` 配哪条 `settle`」的精确答案——乱序结算并发预扣时**金额**可能
//!   错位；而「有没有悬空」（条数差 > 0）不受顺序影响。恢复钩子场景（死机后悬空的
//!   是最后一条 `reserve`）在 FIFO 下恰好正确。理由与申报见 `src/ledger.rs`。
//!
//! # 夹具的来历
//!
//! `new_db()` 与 `tests/persist.rs` 的 `new_db()` 同形（P0 内建迁移 ＋ 本 crate
//! 迁移 120，建库后必须 `migrate()`）；树的起点是 `record_root`（「入账函数」，
//! 见 `the_tree_is_one_tree_with_five_owner_variants` 的注释），此后 `allocate`
//! 逐级长出子树。夹具的取值让「哪一维」与「哪个数」在断言下可区分。

use std::fs;
use std::path::{Path, PathBuf};

use continuum_budget::{
    BudgetError, BudgetOwner, DanglingReservation, DimensionKind, Dimensions, SettleOutcome,
    allocate, dangling_reservations, p4_budget_migrations, record_root, remaining, reserve, settle,
};
use continuum_persist::{Db, Migration, Tx, Value, builtin_migrations};
use continuum_semantics::IntentId;

/// 建一个只装了 P0 内建迁移 ＋ 本 crate 迁移 120 的临时库（与 `tests/persist.rs` 同形）。
fn new_db() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let mut migrations: Vec<Migration> = builtin_migrations();
    migrations.extend(p4_budget_migrations());
    let db = Db::open_with(&path, migrations).unwrap();
    db.migrate().unwrap();
    (dir, db)
}

/// 五维的固定顺序（与 `src/dimensions.rs` 的枚举序一致）——测试里逐维遍历用。
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

/// 一份五维 `Dimensions`（逐维显式给值，免得夹具自己把 `None` 与 `Some(0)` 混掉）。
fn dims(
    money: Option<i64>,
    wall_time: Option<i64>,
    token: Option<i64>,
    gpu_time: Option<i64>,
    network_transfer: Option<i64>,
) -> Dimensions {
    Dimensions {
        money,
        wall_time,
        token,
        gpu_time,
        network_transfer,
    }
}

/// 五维同值。
fn all(v: i64) -> Dimensions {
    dims(Some(v), Some(v), Some(v), Some(v), Some(v))
}

/// 五维全 `None`。
fn all_none() -> Dimensions {
    dims(None, None, None, None, None)
}

/// 只有 `d` 这一维取值 `v`，其余全 `None`。
fn only(d: DimensionKind, v: i64) -> Dimensions {
    let mut out = all_none();
    set(&mut out, d, Some(v));
    out
}

/// 把 `d` 维写成 `value`（测试夹具用）。
fn set(out: &mut Dimensions, d: DimensionKind, value: Option<i64>) {
    match d {
        DimensionKind::Money => out.money = value,
        DimensionKind::WallTime => out.wall_time = value,
        DimensionKind::Token => out.token = value,
        DimensionKind::GpuTime => out.gpu_time = value,
        DimensionKind::NetworkTransfer => out.network_transfer = value,
    }
}

/// 按维读 `Remaining` 的公开读者（`money()` 一族）。
fn read(r: &continuum_budget::Remaining, kind: DimensionKind) -> Option<i64> {
    match kind {
        DimensionKind::Money => r.money(),
        DimensionKind::WallTime => r.wall_time(),
        DimensionKind::Token => r.token(),
        DimensionKind::GpuTime => r.gpu_time(),
        DimensionKind::NetworkTransfer => r.network_transfer(),
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

/// 该 owner 的 `settle` 行按五维汇总——判「`settled` 逐维未动」用。
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

/// `budget_node` 的行数。
fn node_count(tx: &Tx<'_>) -> i64 {
    match &tx.query("SELECT COUNT(*) FROM budget_node", &[]).unwrap()[0][0] {
        Value::Int(n) => *n,
        other => panic!("COUNT(*) 不是整数: {other:?}"),
    }
}

// ===== §333 的五维：None 与 Some(0) 不是同一件事（§12.1、D 的 BudgetView 同判） =====

/// **五维各一条**：`None`（该量纲不构成约束）与 `Some(0)`（额度为零）可区分，
/// 含 `None → None`。
///
/// # 判据与红条件
///
/// `BudgetView` 的模块文档（`crates/continuum-model-registry/src/budget.rs:31-33`）
/// 明写两者不是同一件事，本层的 `Remaining` 必须守同一条（§12.6 的引用）。
/// **红条件**：把 `Option<i64>` 换成默认 `0`（移除档——`None → None` 那半条红）；
/// 或让某条夹具里两者恰好同值（那时它是**等价变异体**——故五条夹具的
/// `None` 侧与 `Some(0)` 侧输出必须不同）。
///
/// # 附：约束侧的一条（同一括号语义的另一半）
///
/// `None` 不只在值上，它的语义是「**不构成约束**」——故一条 `None` 维的预扣放行、
/// 一条 `Some(0)` 维的同额预扣拒。没有这半条，`None → None` 只是值的保持，
/// 「不构成约束」这句话没有照片。
#[test]
fn dimension_none_is_not_some_zero() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();

    for kind in FIVE {
        // 该维 None、其余维 Some(100)：不构成约束。
        let owner_none = intent(&format!("i-none-{}", kind.as_str()));
        let mut none_fixture = all(100);
        set(&mut none_fixture, kind, None);
        record_root(&tx, owner_none.clone(), none_fixture, 0).unwrap();
        let r_none = remaining(&tx, &owner_none).unwrap();
        assert_eq!(
            read(&r_none, kind),
            None,
            "（{kind:?}）`None` 读回必须是 `None`——含 None → None 那一半"
        );

        // 该维 Some(0)、其余维 Some(100)：额度为零。
        let owner_zero = intent(&format!("i-zero-{}", kind.as_str()));
        let mut zero_fixture = all(100);
        set(&mut zero_fixture, kind, Some(0));
        record_root(&tx, owner_zero.clone(), zero_fixture, 0).unwrap();
        let r_zero = remaining(&tx, &owner_zero).unwrap();
        assert_eq!(
            read(&r_zero, kind),
            Some(0),
            "（{kind:?}）额度为零读回 `Some(0)`——它不等于 `None`"
        );

        assert_ne!(
            read(&r_none, kind),
            read(&r_zero, kind),
            "（{kind:?}）`None` 与 `Some(0)` 必须可区分——同值即等价变异体（纪律 1(b)）"
        );
    }

    // 约束侧：`None` 维放行。
    let owner_none = intent("i-none-constraint");
    record_root(&tx, owner_none.clone(), all_none(), 0).unwrap();
    let r = reserve(&tx, owner_none, only(DimensionKind::Token, 5), 0)
        .ok()
        .expect("`None` = 该量纲不构成约束——预扣应放行");
    settle(&tx, r, only(DimensionKind::Token, 5), 1).unwrap();

    // 约束侧：`Some(0)` 维拒。
    let owner_zero = intent("i-zero-constraint");
    record_root(&tx, owner_zero.clone(), only(DimensionKind::Token, 0), 0).unwrap();
    let err = reserve(&tx, owner_zero, only(DimensionKind::Token, 5), 0)
        .err()
        .expect("`Some(0)` 维的预扣必须被拒");
    assert_eq!(
        err,
        BudgetError::Insufficient {
            dimension: DimensionKind::Token
        },
        "`Some(0)` = 额度为零——不是『不构成约束』"
    );
}

// ===== §333 的五个字段名（穷尽解构，不带 `..`） =====

/// `Dimensions` 的字段清单逐项：`money` / `wall_time` / `token` / `gpu_time` /
/// `network_transfer`（§333）。
///
/// # 并写明
///
/// **五个量纲没有单位、没有取值域**（§16 第 19 条）——本层**不解释、不换算、
/// 不做跨量纲比较**。本用例只钉字段名与可构造性（`pub`），不钉任何语义。
///
/// # 红条件
///
/// 改一个字段名、加删一个字段即**编译不过**（穷尽解构，`..` 一个都不许写——
/// 带 `..` 的模式是穷尽性豁免，加字段它就照过）。
#[test]
fn the_five_dimension_names_are_recorded_verbatim() {
    let d = dims(Some(1), Some(2), Some(3), Some(4), Some(5));

    // 穷尽解构，不带 `..`——这五个名字就是 §333 的字段清单。
    let Dimensions {
        money,
        wall_time,
        token,
        gpu_time,
        network_transfer,
    } = d;

    assert_eq!(money, Some(1));
    assert_eq!(wall_time, Some(2));
    assert_eq!(token, Some(3));
    assert_eq!(gpu_time, Some(4));
    assert_eq!(network_transfer, Some(5));
}

// ===== I1：reserve 返回具体哪一维不足（逐维五条） =====

/// **逐维五条**：某一维不足时 `Err(Insufficient { dimension })`，且 `dimension`
/// 是**那一维**（不是笼统的 `Exceeds`）。
///
/// 依据：ENG-005 的「没有预留，§110 这一步就没有判据」落在这一行上（§12.2）。
/// **红条件**：返回笼统的 `Exceeds`（取反档——五条全红，因为拿不到具体维）。
#[test]
fn reserve_reports_the_specific_dimension() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();

    for kind in FIVE {
        let owner = intent(&format!("i-{}", kind.as_str()));
        record_root(&tx, owner.clone(), all(10), 0).unwrap();

        // 只有该维超过：可用 10、预扣 11。
        let err = reserve(&tx, owner, only(kind, 11), 0)
            .err()
            .unwrap_or_else(|| panic!("（{kind:?}）超过可用额必须被拒"));
        assert_eq!(
            err,
            BudgetError::Insufficient { dimension: kind },
            "（{kind:?}）reserve 的拒必须点名**那一维**"
        );
    }
}

/// I1：`available(n,d) ≥ 0` 的违规一条（fail-closed 侧），**且失败不留半写账**。
#[test]
fn i1_a_reserve_beyond_the_available_amount_is_rejected() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();

    let owner = intent("i-1");
    record_root(&tx, owner.clone(), all(10), 0).unwrap();
    let before = ledger_rows(&tx);

    let err = reserve(&tx, owner, only(DimensionKind::Money, 11), 0)
        .err()
        .expect("可用 10、预扣 11——必须被拒");
    assert_eq!(
        err,
        BudgetError::Insufficient {
            dimension: DimensionKind::Money
        }
    );
    assert_eq!(
        ledger_rows(&tx),
        before,
        "失败路径不许留半写账（账本逐行不变）"
    );
}

// ===== I2：超分配在 allocate 那一刻被拒 =====

/// I2 的强制点（设计点名的入口是 `allocate`）：
/// `Σ 子节点分配额 ≤ 父的可用额`——**超分配在分配那一刻就被拒**，
/// 而不是等到某个子节点 `reserve` 时才拦（§12.2.1 末段）。
///
/// # 措辞差别（写进注释：ENG-005 原文 vs 本设计）
///
/// ENG-005 原文说「兄弟节点的活跃预留之和 ≤ 父的剩余」，本设计判的是
/// 「**兄弟的分配额之和 ≤ 父的可用额**」——即**更强的一条**（分配额 ≥ 该子树
/// 任何预留之和，由 I3 保证）。原句未述「分配是上限还是转移」，本设计取
/// 「**分配是上限、不转移**」（否则 I2 恒真、那条约束不成立）；协调者已裁：
/// 维持本取法（若复审推翻再换，替代式见 §12.2）。
///
/// # 夹具与红条件
///
/// 父可用 10；先分 8 通过；再分「money=3、其余维 1」——Σmoney = 11 超、
/// 其余维 Σ = 9 不超，故错误必须**恰是 Money 维**（「是那一维」有判别力）。
/// **红条件**：把 I2 挪到 `reserve` 才判（取反档——本用例红，且超分配会静默进树）。
#[test]
fn i2_is_enforced_at_allocation_time() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();

    let parent = intent("i-parent");
    let child_a = intent("i-child-a");
    let child_b = intent("i-child-b");
    record_root(&tx, parent.clone(), all(10), 0).unwrap();

    assert!(
        allocate(&tx, parent.clone(), child_a, all(8), 0).is_ok(),
        "8 ≤ 10，这一条应通过"
    );
    let nodes_before = node_count(&tx);
    let ledger_before = ledger_rows(&tx);

    let err = allocate(
        &tx,
        parent,
        child_b,
        dims(Some(3), Some(1), Some(1), Some(1), Some(1)),
        0,
    )
    .err()
    .expect("8 + 3 = 11 > 10——超分配必须被拒");
    assert_eq!(
        err,
        BudgetError::Insufficient {
            dimension: DimensionKind::Money
        },
        "超的是 money 那一维（其余维 8 + 1 = 9 不超）"
    );
    assert_eq!(node_count(&tx), nodes_before, "被拒的分配不许进树");
    assert_eq!(ledger_rows(&tx), ledger_before, "被拒的分配不许落账");
}

// ===== I3：settled + reserved ≤ allocation =====

/// I3：`settled(n,d) + reserved(n,d) ≤ allocation(n,d)` 的违规一条。
///
/// # 与 I1 的关系（写进注释，免得被读成重复）
///
/// 代换 `available = allocation − settled − reserved` 后，I1 与 I3 是同一个不等式；
/// 但两条用例的夹具不同：I1 那条是**无 `settled` 参与**时的单条越界，
/// 本条是**已有 `settled`** 时「新的预扣会把和推过 allocation」的越界——
/// plan 要求三条不变量**各自一条违规断言，不合并**（它们钉的是三条不同的恒等式）。
#[test]
fn i3_settled_plus_reserved_beyond_the_allocation_is_rejected() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();

    let owner = intent("i-1");
    record_root(&tx, owner.clone(), all(10), 0).unwrap();

    // 先预扣 6 并结算（settled = 6、reserved = 0、available = 4）。
    let r = reserve(&tx, owner.clone(), only(DimensionKind::Token, 6), 0)
        .ok()
        .expect("预扣 6 ≤ 10");
    settle(&tx, r, only(DimensionKind::Token, 6), 1).unwrap();

    // 再预扣 5：settled(6) + reserved(5) = 11 > allocation(10) ⇒ 拒。
    let err = reserve(&tx, owner, only(DimensionKind::Token, 5), 2)
        .err()
        .expect("6 + 5 = 11 > 10——必须被拒");
    assert_eq!(
        err,
        BudgetError::Insufficient {
            dimension: DimensionKind::Token
        }
    );
}

// ===== 四个入口 × 负值：本 task 落地的三个（check 归 Task 18） =====

/// `reserve` 拒收含负值的 `Dimensions`（§12.2.1 的同一口径），**且账本未变**。
///
/// # 口径（申报，写进注释）
///
/// 设计 §12.2.1 定「四个吃 `Dimensions` 的入口在同一口径下，**一律拒收、不做豁免**」——
/// 判据是「同一个口径管辖四个入口，逐个豁免会让口径退化成一张白名单」。
/// **红条件**：按 `0` 截断或放行（移除档——那一条红、其余条仍绿；
/// 这正是逐入口各写一条的理由）。
///
/// # 判据（负值为什么是 fail-open）
///
/// 负的预扣使 `reserved(n,d)` 变负、`available` 变大——与负的 `actual` **完全同形**
/// （§12.2.1 已写）。
#[test]
fn reserve_rejects_a_negative_dimension() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();

    let owner = intent("i-1");
    record_root(&tx, owner.clone(), all(10), 0).unwrap();

    // 先有一条**活跃**的正常预扣——失败后它必须仍在。
    let r = reserve(&tx, owner.clone(), only(DimensionKind::Token, 4), 0)
        .ok()
        .expect("正常预扣应通过");
    let dangling_before = dangling_reservations(&tx).unwrap();
    let rows_before = ledger_rows(&tx);
    let settled_before = settled_by_dimension(&tx, owner.as_key().as_str());

    let err = reserve(&tx, owner.clone(), only(DimensionKind::Token, -2), 1)
        .err()
        .expect("负值必须被拒");
    assert_eq!(
        err,
        BudgetError::NegativeAmount {
            dimension: DimensionKind::Token,
            value: -2
        },
        "dimension 是那一维、value 是那个负数"
    );

    // 账本三件事：行数、逐行内容、settled 逐维都不动；正常预扣仍活跃。
    assert_eq!(ledger_rows(&tx), rows_before, "账本逐行不变");
    assert_eq!(
        settled_by_dimension(&tx, owner.as_key().as_str()),
        settled_before,
        "settled 逐维未动"
    );
    assert_eq!(
        dangling_reservations(&tx).unwrap(),
        dangling_before,
        "先前的预扣仍活跃（dangling 列表不变）"
    );
    // 那条预扣还可以正常结算（说明它确实还在飞）。
    settle(&tx, r, only(DimensionKind::Token, 4), 2).unwrap();
}

/// `allocate` 拒收含负值的 `Dimensions`——「负分配让 I2 更易满足」不是豁免的理由
/// （§12.2.1：「三处都拒、唯独 `allocate` 收下负值会是一条无理由的不一致」）。
#[test]
fn allocate_rejects_a_negative_dimension() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();

    let parent = intent("i-parent");
    record_root(&tx, parent.clone(), all(10), 0).unwrap();
    let nodes_before = node_count(&tx);
    let rows_before = ledger_rows(&tx);
    let settled_before = settled_by_dimension(&tx, parent.as_key().as_str());

    let err = allocate(
        &tx,
        parent.clone(),
        intent("i-child"),
        only(DimensionKind::Money, -5),
        1,
    )
    .err()
    .expect("负分配额必须被拒");
    assert_eq!(
        err,
        BudgetError::NegativeAmount {
            dimension: DimensionKind::Money,
            value: -5
        }
    );

    assert_eq!(node_count(&tx), nodes_before, "被拒的分配不许进树");
    assert_eq!(ledger_rows(&tx), rows_before, "账本逐行不变");
    assert_eq!(
        settled_by_dimension(&tx, parent.as_key().as_str()),
        settled_before,
        "settled 逐维未动"
    );
}

/// `settle` 拒收含负值的 `actual`——**拒收，不截断**（§12.2）。
///
/// 截断是**静默**的：它把产生方的一个 bug 吞掉，账面上看不出发生过什么。
#[test]
fn settle_rejects_a_negative_dimension() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();

    let owner = intent("i-1");
    record_root(&tx, owner.clone(), all(10), 0).unwrap();
    let r = reserve(&tx, owner.clone(), only(DimensionKind::Token, 4), 0)
        .ok()
        .expect("正常预扣应通过");

    let rows_before = ledger_rows(&tx);
    let settled_before = settled_by_dimension(&tx, owner.as_key().as_str());
    let dangling_before = dangling_reservations(&tx).unwrap();

    let err = settle(&tx, r, only(DimensionKind::Token, -3), 1)
        .err()
        .expect("负 actual 必须被拒");
    assert_eq!(
        err,
        BudgetError::NegativeAmount {
            dimension: DimensionKind::Token,
            value: -3
        }
    );

    assert_eq!(ledger_rows(&tx), rows_before, "账本逐行不变");
    assert_eq!(
        settled_by_dimension(&tx, owner.as_key().as_str()),
        settled_before,
        "settled 逐维未动"
    );
    assert_eq!(
        dangling_reservations(&tx).unwrap(),
        dangling_before,
        "预扣仍活跃（没有被半结算）"
    );
}

// ===== 另一侧：各维为 0 不是负值（三个入口各一条） =====

/// `record_root` 拒收含负值的 `Dimensions`——它是**本 task 落地的第五个**吃
/// `Dimensions` 的入口（`record_root` / `allocate` / `reserve` / `settle`；
/// `check` 归 Task 18），**同一口径、不做豁免**。
///
/// 设计 §12.2.1 的表列四个入口，而 `record_root` 与 `allocate` 同刻落地、`dims`
/// 的产生方相同（规划方）——「同一个口径管辖所有吃 `Dimensions` 的入口」这句话
/// 要在这第五个入口上也真的成立（否则它是一条有例外的绝对措辞）。
#[test]
fn record_root_rejects_a_negative_dimension() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();

    let owner = intent("i-1");
    let nodes_before = node_count(&tx);
    let rows_before = ledger_rows(&tx);

    let err = record_root(&tx, owner, only(DimensionKind::GpuTime, -1), 0)
        .err()
        .expect("负值必须被拒");
    assert_eq!(
        err,
        BudgetError::NegativeAmount {
            dimension: DimensionKind::GpuTime,
            value: -1
        }
    );

    assert_eq!(node_count(&tx), nodes_before, "被拒的根不许进树");
    assert_eq!(ledger_rows(&tx), rows_before, "账本逐行不变");
}

/// 各维为 `0` 的预扣**合法**（额度为零的预扣是合法的）。
#[test]
fn reserve_accepts_an_all_zero_dimension() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();

    let owner = intent("i-1");
    record_root(&tx, owner.clone(), all(10), 0).unwrap();
    let r = reserve(&tx, owner.clone(), all(0), 0)
        .ok()
        .expect("0 不是负值——应通过");
    assert_eq!(
        remaining(&tx, &owner).unwrap().money(),
        Some(10),
        "零预扣不影响可用额"
    );
    settle(&tx, r, all(0), 1).unwrap();
}

/// 各维为 `0` 的分配**合法**。
#[test]
fn allocate_accepts_an_all_zero_dimension() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();

    let parent = intent("i-parent");
    record_root(&tx, parent.clone(), all(10), 0).unwrap();
    assert!(
        allocate(&tx, parent, intent("i-child"), all(0), 0).is_ok(),
        "0 不是负值——应通过"
    );
}

/// 各维为 `0` 的 `actual` ⇒ `Within`（`0` 不是负值；§12.2 拒的是**负**值）。
#[test]
fn settle_accepts_an_all_zero_dimension() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();

    let owner = intent("i-1");
    record_root(&tx, owner.clone(), all(10), 0).unwrap();
    let r = reserve(&tx, owner.clone(), all(2), 0)
        .ok()
        .expect("正常预扣应通过");
    let outcome = settle(&tx, r, all(0), 1).unwrap();
    assert!(
        matches!(outcome, SettleOutcome::Within { .. }),
        "全 0 的 actual 是 Within，实得 {outcome:?}"
    );
}

// ===== 结算：两支与两条反例 =====

/// `Within` 一支：`refunded` **逐维**是差额。
#[test]
fn settle_within_refunds_the_unused_amount() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();

    let owner = intent("i-1");
    record_root(&tx, owner.clone(), all(10), 0).unwrap();
    let r = reserve(&tx, owner.clone(), dims(Some(5), None, Some(7), None, None), 0)
        .ok()
        .expect("正常预扣应通过");

    let outcome = settle(&tx, r, dims(Some(3), None, Some(4), None, None), 1).unwrap();
    match outcome {
        SettleOutcome::Within { refunded } => {
            // 逐维（五维全列）：两维有差额、其余维不在本次预扣的定义域里。
            assert_eq!(refunded.money, Some(2), "money 5 − 3 = 2");
            assert_eq!(refunded.wall_time, None, "本次预扣没有 wall_time");
            assert_eq!(refunded.token, Some(3), "token 7 − 4 = 3");
            assert_eq!(refunded.gpu_time, None);
            assert_eq!(refunded.network_transfer, None);
        }
        other => panic!("应为 Within，实得 {other:?}"),
    }

    // 余量立刻反映：10 − 3 = 7（money）、10 − 4 = 6（token）。
    let rest = remaining(&tx, &owner).unwrap();
    assert_eq!(rest.money(), Some(7));
    assert_eq!(rest.token(), Some(6));
}

/// `Over` 一支：报**哪一维**、**超多少**（不是笼统的「超了」）。
#[test]
fn settle_over_reports_the_dimension_and_the_amount() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();

    let owner = intent("i-1");
    record_root(&tx, owner.clone(), all(10), 0).unwrap();
    let r = reserve(&tx, owner.clone(), only(DimensionKind::Token, 7), 0)
        .ok()
        .expect("正常预扣应通过");

    let outcome = settle(&tx, r, only(DimensionKind::Token, 10), 1).unwrap();
    match outcome {
        SettleOutcome::Over { dimension, by } => {
            assert_eq!(dimension, DimensionKind::Token, "超的是 token 那一维");
            assert_eq!(by, 3, "10 − 7 = 3");
        }
        other => panic!("应为 Over，实得 {other:?}"),
    }
}

/// §12.2 照片 (a)：负 `actual` ⇒ 具体 `Err`，**且账本未变**——
/// `reserve` 仍活跃（`dangling_reservations` 里还在）、`settled` 逐维未动、
/// `budget_ledger` 行数不变。
///
/// # 判据（写进注释）：负数是 **fail-open 的入口**
///
/// `settled` 会因此变小、`available` 反而变大，而 **I1 只约束 `available`、
/// 不约束 `actual` 的符号**——一个负的 `actual` 能把已经花掉的额度「还回来」
/// 且没有任何断言会拦它。
///
/// # 红条件
///
/// 按 `0` 截断（移除档——那时返回 `Ok` 且账面被悄悄改小；**本仓口径：截断是静默的**，
/// 它把产生方的一个 bug 吞掉）。
#[test]
fn a_negative_actual_is_rejected_and_the_ledger_is_untouched() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();

    let owner = intent("i-1");
    record_root(&tx, owner.clone(), all(10), 0).unwrap();
    // 三维各有预扣，负值填在**中间**那一维（token），好让「是哪一维」可区分。
    let r = reserve(
        &tx,
        owner.clone(),
        dims(Some(3), None, Some(6), None, Some(2)),
        0,
    )
    .ok()
    .expect("正常预扣应通过");

    let rows_before = ledger_rows(&tx);
    let settled_before = settled_by_dimension(&tx, owner.as_key().as_str());

    let err = settle(&tx, r, dims(Some(1), None, Some(-4), None, Some(2)), 1)
        .err()
        .expect("负的 actual 必须被拒");
    assert_eq!(
        err,
        BudgetError::NegativeAmount {
            dimension: DimensionKind::Token,
            value: -4
        },
        "dimension 是那一维、value 是那个负数"
    );

    assert_eq!(ledger_rows(&tx), rows_before, "budget_ledger 行数/内容不变");
    assert_eq!(
        settled_by_dimension(&tx, owner.as_key().as_str()),
        settled_before,
        "settled 逐维未动"
    );
    assert_eq!(
        dangling_reservations(&tx).unwrap(),
        vec![
            DanglingReservation {
                owner: owner.clone(),
                dimension: DimensionKind::Money,
                amount: 3,
            },
            DanglingReservation {
                owner: owner.clone(),
                dimension: DimensionKind::Token,
                amount: 6,
            },
            DanglingReservation {
                owner: owner.clone(),
                dimension: DimensionKind::NetworkTransfer,
                amount: 2,
            },
        ],
        "预扣仍活跃——三处都在（dangling 按维列出，逐条断言）"
    );
}

/// 反例（另一侧）：全 `0` 的 `actual` ⇒ `Within` 且**全额**退回。
///
/// **不能省**——只钉拒绝那侧会让「`0` 也被拒」漂过去（`0` 不是负数，
/// §12.2 拒的是**负**值）。
#[test]
fn a_zero_actual_settles_within_and_refunds_everything() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();

    let owner = intent("i-1");
    record_root(&tx, owner.clone(), all(10), 0).unwrap();
    let r = reserve(&tx, owner.clone(), dims(Some(5), None, Some(7), None, None), 0)
        .ok()
        .expect("正常预扣应通过");

    let outcome = settle(&tx, r, dims(Some(0), None, Some(0), None, None), 1).unwrap();
    match outcome {
        SettleOutcome::Within { refunded } => {
            assert_eq!(refunded.money, Some(5), "全额退回");
            assert_eq!(refunded.token, Some(7), "全额退回");
        }
        other => panic!("应为 Within（0 不是负值），实得 {other:?}"),
    }

    // 余量回到预扣前。
    let rest = remaining(&tx, &owner).unwrap();
    assert_eq!(rest.money(), Some(10));
    assert_eq!(rest.token(), Some(10));
}

// ===== 树：四个 owner 变体（穷尽解构）＋ SystemMaintenance 的否定式照片 =====

/// `BudgetOwner` **四个变体**逐枚穷尽解构（不带 `..`）。
///
/// # 用例名里的「五」与正文的四枚（照实交代）
///
/// 设计 §12.1 的代码块标题写「**五个** owner 变体齐了才叫『单一树』」，
/// 而其正文与 §12.2.1、§16 第 18 条、本计划 Step 2 一致列**四枚**：
/// `SystemMaintenance` / `Intent(IntentId)` / `NodeRun { intent, node }` /
/// `Exploration { intent, of_node }`。用例名沿用计划（`five`），钉的是那四枚——
/// **本 task 不发明第五枚**。
///
/// # `SystemMaintenance` 的创建方与「本层不创建」（否定式照片）
///
/// `SystemMaintenance` 的**创建方是第 7 层**，**今天没有任何一方会创建它**——
/// 本层提供该变体与它的**入账函数**（`record_root`：把一枚 owner 记为一棵（子）树的
/// 根），但本层**没有任何流程**调用它（第 7 层未建；测试是本层的调用方）。
///
/// 故本层**没有**任何「造一个 `SystemMaintenance` 节点」的生产路径——
/// **第一侧照片即本用例的扫描半边**：`src/` 里 `SystemMaintenance` 的出现点
/// **逐条等于白名单**，而白名单里的每一处都在**类型与编解码层**（定义 / 编码 / 解码 /
/// 相等判定），**没有一处把它写进表**。**第二侧**是 Task 21 的源码面复核。
///
/// # 红条件
///
/// 加一个变体（穷尽解构编译不过）；新增一个出现点（清单对不上、红）。
#[test]
fn the_tree_is_one_tree_with_five_owner_variants() {
    // (1) 穷尽解构：四个变体逐枚（不带 `..`）——加/删变体即编译不过。
    fn exhaustive(owner: &BudgetOwner) -> &'static str {
        match owner {
            BudgetOwner::SystemMaintenance => "system_maintenance",
            BudgetOwner::Intent(_) => "intent",
            BudgetOwner::NodeRun { intent: _, node: _ } => "node_run",
            BudgetOwner::Exploration {
                intent: _,
                of_node: _,
            } => "exploration",
        }
    }
    let all_four = [
        BudgetOwner::SystemMaintenance,
        intent("i-1"),
        BudgetOwner::NodeRun {
            intent: IntentId::from_str("i-1"),
            node: "n-1".to_owned(),
        },
        BudgetOwner::Exploration {
            intent: IntentId::from_str("i-1"),
            of_node: "n-1".to_owned(),
        },
    ];
    let names: Vec<&str> = all_four.iter().map(exhaustive).collect();
    assert_eq!(
        names,
        vec!["system_maintenance", "intent", "node_run", "exploration"],
        "四个变体逐枚（不抽代表）"
    );

    // (2) 否定式照片：`src/` 里 `SystemMaintenance` 的出现点逐条等于白名单。
    let sources = crate_sources();
    let mut occurrences: Vec<String> = Vec::new();
    for (path, body) in &sources {
        let code = strip_comments(body);
        for line in code.lines() {
            if line.contains("SystemMaintenance") {
                let normalized = line.split_whitespace().collect::<Vec<_>>().join(" ");
                let normalized = normalized.trim();
                if !normalized.is_empty() {
                    occurrences.push(format!("{}: {normalized}", path.display()));
                }
            }
        }
    }
    occurrences.sort();
    occurrences.dedup();

    // 白名单（**按字符串序**——`occurrences` 排过序；每条都写明属于哪一层）——
    // **都在类型与编解码层**：
    let expected = [
        // 解码层：`parse` 的变体分支（从库里读回时构造——它在读路径，不写表）。
        r#"src/tree.rs: "system_maintenance" => Some(BudgetOwner::SystemMaintenance),"#,
        // 相等判定（`PartialEq` 的手写实现）——两个根同一。
        "src/tree.rs: (BudgetOwner::SystemMaintenance, BudgetOwner::SystemMaintenance) => true,",
        // 编码层：`as_key` 的变体分支。
        r#"src/tree.rs: BudgetOwner::SystemMaintenance => "{\"kind\":\"system_maintenance\"}".to_owned(),"#,
        // 类型层：枚举的变体定义。
        "src/tree.rs: SystemMaintenance,",
    ];
    let expected: Vec<String> = expected.iter().map(|s| (*s).to_owned()).collect();
    assert_eq!(
        occurrences, expected,
        "`src/` 里含 `SystemMaintenance` 的代码出现点应逐条等于白名单：新增出现点要么\
         是类型/编解码层的一处（更新清单），要么是一个**把它写进表**的构造点——\
         后者与「本层不创建它」相抵，先人工复核再决定"
    );
}

// ===== 源码扫描的辅助（同 `continuum-semantics/tests/decision.rs` 的形状） =====

/// `src/` 下全部 `.rs` 的 `(相对路径, 内容)`，路径排序。
fn crate_sources() -> Vec<(PathBuf, String)> {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    collect_rs(&src, &mut files);
    files.sort();
    files
        .into_iter()
        .map(|path| {
            let rel = Path::new("src")
                .join(path.strip_prefix(&src).expect("收集自 `src/` 下的路径应以它开头"));
            let source = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("读不到源文件 {}：{e}", path.display()));
            (rel, source)
        })
        .collect()
}

/// 递归收集 `dir` 下全部 `.rs`。**读不到的目录直接炸，不跳过**——静默跳过只会让守卫变窄。
fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("列不出目录 {}：{e}", dir.display()))
    {
        let path = entry
            .unwrap_or_else(|e| panic!("列目录 {} 时出错：{e}", dir.display()))
            .path();
        if path.is_dir() {
            collect_rs(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// 剥掉注释后的源代码（字符串字面量里的 `//` 不算注释——记录的是**代码行**）。
fn strip_comments(src: &str) -> String {
    // 四种状态：0 = 代码，1 = 行注释，2 = 块注释，3 = 字符串字面量。
    let chars: Vec<char> = src.chars().collect();
    let mut out = String::with_capacity(src.len());
    let mut state = 0u8;
    let mut i = 0usize;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        match state {
            0 if c == '/' && next == Some('/') => {
                state = 1;
                i += 2;
            }
            0 if c == '/' && next == Some('*') => {
                state = 2;
                i += 2;
            }
            0 => {
                if c == '"' {
                    state = 3;
                }
                out.push(c);
                i += 1;
            }
            1 => {
                if c == '\n' {
                    state = 0;
                    out.push(c);
                }
                i += 1;
            }
            2 => {
                if c == '*' && next == Some('/') {
                    state = 0;
                    i += 2;
                } else {
                    i += 1;
                }
            }
            3 => {
                if c == '\\' {
                    out.push(c);
                    if let Some(escaped) = next {
                        out.push(escaped);
                        i += 2;
                    } else {
                        i += 1;
                    }
                } else {
                    if c == '"' {
                        state = 0;
                    }
                    out.push(c);
                    i += 1;
                }
            }
            _ => unreachable!(),
        }
    }
    out
}
