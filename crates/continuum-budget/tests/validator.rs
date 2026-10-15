//! Task 18 的 Budget Validator（§110、§12.4）的照片。
//!
//! | 用例 | 钉的判据 | 红条件（档位） |
//! |---|---|---|
//! | `within_and_exceeds_each_have_a_photo` | `Within` 一条、`Exceeds` 一条 | 只有一枝（**取反档**） |
//! | `check_rejects_a_negative_dimension` | `estimate` 含负值 ⇒ `Err(NegativeAmount{dimension, value})`，**并断言账本未变**（§12.2.1 的表把 `check` 也列进那四行） | 负值放行（**移除档**——比较恒为 `Within`，计划不因它被拦下） |
//! | `check_accepts_an_all_zero_dimension` | 反例（另一侧）：各维为 0 ⇒ `Ok(Within)`（`0` 不是负值） | `0` 也被拒（**收紧档**） |
//! | `an_unknown_owner_is_not_within` | 树上查不到 ⇒ `Err(UnknownOwner)`，**不是 `Within`**（§12.4 第 3 条，fail-closed 侧） | 查不到就返回 `Within`（**放宽档**——等于对一棵不存在的子树放行） |
//! | `exceeds_reports_the_specific_dimension` | **逐维五条**：`dimension` 是那一维、`by` 是那个数 | 笼统的「超了」（**取反档**——五条全红） |
//! | `the_verdict_compares_against_remaining_not_allocation` | 构造「按 `allocation` 判会放行、按 `remaining` 判超支」：断言得 `Exceeds` | 把判据换成 `allocation`（**取反档**——本用例红） |
//! | `the_validator_does_not_seek_low_cost_alternatives` | **否定式照片**：`check` 只返回裁决——返回类型恰两枚（穷尽解构），`src/`（剥注释）里没有任何「替代方案」一族的命名 | 实现替调用方去找替代方案（**取反档**）；或给返回值加第二个「替代方案」出口 |
//!
//! # 三条写死的口径（§12.4）
//!
//! 1. **超支不是 `Err`**，是 `BudgetVerdict::Exceeds`——§110 的流程要把「预计超支」
//!    当**一个可处置的结论**往下走（找替代方案 → 找不到则 Decision），做成 `Err`
//!    会让调用方把它当失败而中止。
//! 2. **量纲的单位／取值域不是 `Err`**（§333 未给单位，§16 第 19 条）——
//!    但「负值」不是「不合理」而是「不可能」，由 `NegativeAmount` 拒收。
//! 3. **`UnknownOwner` 是 fail-closed 的**：查不到分配记录时**不给 `Within`**。
//!
//! # 「先寻找合法低成本方案」不在本层（§12.4、§16 第 20 条）
//!
//! 那是一次**重新规划**（换模型/换节点/分块/延长时限），是执行层 Planner 的动作。
//! 分工是：`check` 返回 `Exceeds` → **调用方去找替代方案** → 找不到时调本层的
//! `Decision` 路径。「找不到」由谁判定、试几次算「找不到」**规范未给**。

use std::fs;
use std::path::{Path, PathBuf};

use continuum_budget::{
    BudgetError, BudgetOwner, BudgetVerdict, DimensionKind, Dimensions, allocate, check,
    p4_budget_migrations, record_root, remaining, reserve, settle,
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

// ===== `Within` 与 `Exceeds` 各一条 =====

/// `Within` 一条、`Exceeds` 一条——两枝都要有（只有一枝的实现会在另一条上红。
/// `BudgetVerdict` 没有派生 `PartialEq`，故逐变体 `match` 断言）。
#[test]
fn within_and_exceeds_each_have_a_photo() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();

    let owner = intent("i-validate");
    record_root(&tx, owner.clone(), all(100), 0).unwrap();

    // Within：估计全维 50 ≤ 余量 100。
    match check(&tx, &all(50), owner.clone()).unwrap() {
        BudgetVerdict::Within => {}
        other => panic!("50 ≤ 100 应得 Within：{other:?}"),
    }

    // Exceeds：估计全维 150 > 余量 100——五维同超时报**字段序第一处**（`Money`；
    // 设计未给多维多超的选择规则，本实现取「字段序第一处」，同 `settle` 的 `Over`）。
    match check(&tx, &all(150), owner.clone()).unwrap() {
        BudgetVerdict::Exceeds { dimension, by } => {
            assert_eq!(dimension, DimensionKind::Money, "五维同超：字段序第一处是 Money");
            assert_eq!(by, 50, "by 是超出的量：150 − 100");
        }
        other => panic!("150 > 100 应得 Exceeds：{other:?}"),
    }
}

// ===== 负值拒收（§12.2.1 四行表的 `check` 那一行） =====

/// `estimate` 某一维为负 ⇒ `Err(NegativeAmount { dimension, value })`，
/// **并断言账本未变**（失败路径不但断言哪一种 `Err`，还断言没有半写的副作用）。
///
/// # 依据
///
/// §12.2.1 的表把 `check` 也列进那四行；负估计让 §12.4 的比较**恒为 `Within`**，
/// 计划不因它被拦下——与 `settle` 的负 `actual` 同形（负数是 fail-open 的入口）。
/// **拒收，不截断**：截断是静默的，它把产生方的一个 bug 吞掉。
#[test]
fn check_rejects_a_negative_dimension() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();

    let owner = intent("i-negative-estimate");
    record_root(&tx, owner.clone(), all(100), 0).unwrap();
    let rows_before = ledger_rows(&tx);

    let err = check(&tx, &only(DimensionKind::WallTime, -7), owner.clone())
        .err()
        .expect("负估计必须被拒");
    assert_eq!(
        err,
        BudgetError::NegativeAmount {
            dimension: DimensionKind::WallTime,
            value: -7
        },
        "dimension 是那一维、value 是那个负数"
    );

    // 账本未变（`check` 是只读路径——这一条守「未来给它加副作用」的坏法）。
    assert_eq!(ledger_rows(&tx), rows_before, "被拒的估计不许动账本");
}

/// 反例（另一侧）：各维为 `0` ⇒ `Ok(Within)`——**`0` 不是负值**。
///
/// 只钉拒绝那侧，会让「`0` 也被拒」这类**收紧档**漂过去（§12.2.1 的 (b) 照片
/// 是同款判据）。
#[test]
fn check_accepts_an_all_zero_dimension() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();

    let owner = intent("i-zero-estimate");
    record_root(&tx, owner.clone(), all(100), 0).unwrap();

    match check(&tx, &all(0), owner.clone()).unwrap() {
        BudgetVerdict::Within => {}
        other => panic!("各维为 0 是合法的估计，应得 Within：{other:?}"),
    }
}

// ===== fail-closed：未知 owner 不是 `Within` =====

/// 树上查不到该 owner 的分配记录 ⇒ `Err(UnknownOwner { owner })`，**不是 `Within`**。
///
/// # §12.4 第 3 条（fail-closed 侧）与「超支不是 `Err`」一起读
///
/// 给 `Within` 等于**对一棵不存在的子树放行**。而这条 `Err` 与「超支」不同侧：
/// **超支不是 `Err`**，是 `BudgetVerdict::Exceeds`——§110 的流程要把「预计超支」
/// 当**一个可处置的结论**往下走（找替代方案 → 找不到则 Decision），做成 `Err`
/// 会让调用方把它当失败而中止。红条件：查不到就返回 `Within`（**放宽档**）。
#[test]
fn an_unknown_owner_is_not_within() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();

    let owner = intent("i-absent");
    let err = check(&tx, &all(1), owner.clone())
        .err()
        .expect("不在树上必须被拒——不得给 Within");
    assert_eq!(
        err,
        BudgetError::UnknownOwner {
            owner: owner.clone()
        },
        "owner 是入参那一个"
    );
}

// ===== `Exceeds` 的逐维五条 =====

/// **逐维五条**：每条只让一维超，断言 `dimension` 是那一维、`by` 是那个数
/// （`by` 用不同的数，免得「报错维」与「报对少」的漂移互相掩盖）。
#[test]
fn exceeds_reports_the_specific_dimension() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();

    let owner = intent("i-per-dim");
    record_root(&tx, owner.clone(), all(100), 0).unwrap();

    for (index, kind) in FIVE.iter().enumerate() {
        let by = 30 + 10 * index as i64;
        // 只有该维 Some(100 + by)：超出余量（100）→ `by`；其余维 None（不构成约束）。
        match check(&tx, &only(*kind, 100 + by), owner.clone()).unwrap() {
            BudgetVerdict::Exceeds {
                dimension,
                by: got,
            } => {
                assert_eq!(dimension, *kind, "（{kind:?}）dimension 是那一维");
                assert_eq!(got, by, "（{kind:?}）by 是那个数");
            }
            other => panic!("（{kind:?}）应得 Exceeds：{other:?}"),
        }
    }
}

// ===== 要害：判据是 `remaining` 不是 `allocation` =====

/// 构造「**按 `allocation` 判会放行、按 `remaining` 判超支**」的用例，断言得 `Exceeds`。
///
/// # 构造与红条件
///
/// `allocation(owner) = 100`；该 owner 自己执行过一轮（预扣并结算 40）⇒
/// `remaining = 100 − 40 = 60`；`estimate = 80`：`remaining(60) < estimate(80) ≤
/// allocation(100)` ⇒ `Exceeds { by: 20 }`。**红条件**：把判据换成 `allocation`
/// （**取反档**——80 ≤ 100 会得 `Within`，本用例红）。
///
/// # 「兄弟节点占用了额度」这句措辞与 §12.1 的裁定（据实）
///
/// brief 的场景措辞是「该 owner 已有一个兄弟节点占用了额度」。在本实现里
/// （§12.1 的裁定：**分配是上限、不转移**——父的可用额不因一次分配而减少），
/// `remaining(owner) = allocation(owner) − settled(owner) − reserved(owner)`：
/// **兄弟**的分配与消耗**不减少**该 owner 的 `remaining`。故「占用」由该 owner
/// **自己**的结算构造；`remaining < estimate ≤ allocation` 的判据逐字保持。
/// 兄弟节点在本用例里是**真**的（它真的预扣并结算了自己的 100）——它证明的
/// 恰是「兄弟吃满自己的额度**不改变**该 owner 的判定」（`remaining` 只减
/// 该 owner 自己的 `settled` / `reserved`）。
///
/// # 依据
///
/// `remaining` 已经把该 owner 的占用算进去了；按 `allocation` 比会把 ENG-005
/// 要的「一次全局判断」拆回「各层各自裁决」（§12.4）。
#[test]
fn the_verdict_compares_against_remaining_not_allocation() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();

    // 父 P 与两个运行：owner `a` 与它的兄弟 `b`。
    let parent = intent("i-root");
    record_root(&tx, parent.clone(), all(200), 0).unwrap();
    let owner = BudgetOwner::NodeRun {
        intent: IntentId::from_str("i-root"),
        node: "a".into(),
    };
    let sibling = BudgetOwner::NodeRun {
        intent: IntentId::from_str("i-root"),
        node: "b".into(),
    };
    allocate(&tx, parent.clone(), owner.clone(), all(100), 0).unwrap();
    allocate(&tx, parent.clone(), sibling.clone(), all(100), 0).unwrap();

    // 兄弟 b 执行过一轮：预扣并结算 100（把自己的额度吃完）。
    let sibling_reservation = reserve(&tx, sibling.clone(), all(100), 1).unwrap();
    settle(&tx, sibling_reservation, all(100), 2).unwrap();

    // 事实照片：兄弟的动作**不**改变该 owner 的 remaining
    //（§12.1 的裁定「分配是上限、不转移」）。
    let after_sibling = remaining(&tx, &owner).unwrap();
    for kind in FIVE {
        assert_eq!(
            dim_of(after_sibling.dims(), kind),
            Some(100),
            "（{kind:?}）兄弟吃满自己的额度，不改变该 owner 的 remaining"
        );
    }

    // owner a 自己执行过一轮：预扣并结算 40 ⇒ remaining = 100 − 40 = 60。
    let own_reservation = reserve(&tx, owner.clone(), all(40), 3).unwrap();
    settle(&tx, own_reservation, all(40), 4).unwrap();
    let r = remaining(&tx, &owner).unwrap();
    for kind in FIVE {
        assert_eq!(
            dim_of(r.dims(), kind),
            Some(60),
            "（{kind:?}）remaining == 100 − 40"
        );
    }

    // 80：remaining(60) < 80 ≤ allocation(100)——按 allocation 判会放行，
    // 按 remaining 判超支。
    match check(&tx, &all(80), owner.clone()).unwrap() {
        BudgetVerdict::Exceeds { dimension, by } => {
            assert_eq!(dimension, DimensionKind::Money, "五维同超：字段序第一处");
            assert_eq!(by, 20, "by == estimate − remaining == 80 − 60");
        }
        other => panic!(
            "remaining(60) < estimate(80) ≤ allocation(100) 必须得 Exceeds\
             （判据换 allocation 的实现会在这里红）：{other:?}"
        ),
    }
}

// ===== 否定式照片：validator 不找替代方案 =====

/// **否定式照片**（三侧）：`check` **只返回裁决**，本层不产出任何替代方案。
///
/// # 判据三条
///
/// 1. **类型侧**：`BudgetVerdict` **恰两枚**（穷尽解构，不带 `..`）——返回值里
///    **没有装替代方案的格子**（加变体即编译不过）。
/// 2. **签名侧**：`check` 的产物恰是 `Result<BudgetVerdict, BudgetError>`——
///    没有第二个返回位置。
/// 3. **文本侧**：`src/`（剥注释）里没有任何「替代方案」一族的命名
///    （`alternative` / `fallback` / `low_cost` / `seek`，大小写不敏感）。
///
/// # 依据
///
/// §12.4：「先寻找合法低成本方案」是一次**重新规划**（换模型/换节点/分块/延长
/// 时限），是执行层 Planner 的动作——**本层没有 Planner**。**「什么算找不到」的
/// 判定也不在本层**（§16 第 20 条，收件人：规范维护者 ＋ 执行层 Planner）。
#[test]
fn the_validator_does_not_seek_low_cost_alternatives() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    let owner = intent("i-no-alternatives");
    record_root(&tx, owner.clone(), all(100), 0).unwrap();

    // (1) 类型侧：穷尽解构（无 `..`、无通配臂）——加变体即编译不过。
    let verdicts = [
        BudgetVerdict::Within,
        BudgetVerdict::Exceeds {
            dimension: DimensionKind::Money,
            by: 1,
        },
    ];
    let mut names: Vec<&str> = Vec::new();
    for verdict in verdicts {
        match verdict {
            BudgetVerdict::Within => names.push("within"),
            BudgetVerdict::Exceeds { dimension, by } => {
                let _ = (dimension, by); // 字段逐项：两枚字段都在，无 `..`。
                names.push("exceeds");
            }
        }
    }
    assert_eq!(names, ["within", "exceeds"], "`check` 的返回值域恰两枚");

    // (2) 签名侧：产物恰是 `Result<BudgetVerdict, BudgetError>`。
    let typed: Result<BudgetVerdict, BudgetError> = check(&tx, &all(0), owner.clone());
    assert!(typed.is_ok(), "这一行同时钉签名形状");

    // (3) 文本侧：剥注释后 `src/` 里没有「替代方案」一族的命名。
    let mut hits: Vec<String> = Vec::new();
    for (path, body) in crate_sources() {
        let code = strip_comments(&body).to_ascii_lowercase();
        for needle in ["alternative", "fallback", "low_cost", "seek"] {
            if code.contains(needle) {
                hits.push(format!("{}: {needle}", path.display()));
            }
        }
    }
    assert!(
        hits.is_empty(),
        "`src/` 里不允许出现「替代方案」一族的命名（本层不做那件事，§16 第 20 条）——\
         命中：{hits:?}。若这是有意的，改的是这条用例与 `check` 的文档，不是命名本身"
    );
}

// ===== 源码扫描辅助（同 `tests/tree.rs` 的形状） =====

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

/// 递归收集 `dir` 下全部 `.rs`。**读不到的目录直接炸，不跳过**。
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
