//! P4 Task 19：形状乙的两条断言（裁定 §12.6.1）——**穷尽解构，无 `..`**。
//!
//! # 本文件只交付断言，不交付生产函数
//!
//! 裁定 §12.6.1 把 `Remaining → BudgetView` 的**生产投影**判给**驱动**，而驱动今天没有
//! 组装 `RoutingRequest` 的那一步（跨计划前置第四节已写）。故本文件的投影函数
//! （`project`）是**测试内的**，**不进生产面**——「不预先发明 API」（§2.3）在这一处的
//! 兑现方式。
//!
//! # 两个类型、一道恒等转换
//!
//! `Remaining` 在 `continuum-budget`（记账方产出；crate 归属按 §2.1／§2.2／§12.1 落
//! `continuum-budget`，与 §12.6.1 字面的相抵见 `## 遗留` 第一节第 1 条），`BudgetView`
//! 留在 `continuum-model-registry`（资源层的排序输入面），转换由驱动做。
//! **有断言的恒等转换与没有断言的恒等转换是两回事**（裁定判据 3）——本文件就是那道断言。
//!
//! # 三条用例与红条件
//!
//! | 用例 | 钉的判据 | 红条件（档位） |
//! |---|---|---|
//! | `remaining_and_budget_view_have_the_same_five_dimensions` | 两侧字段集**逐项**相同：两处**穷尽解构、一个 `..` 都不写** | 任一侧加字段／改维度名（**编译红**——实测见该用例）；字段互换（**取反档**——编译得过，靠逐维值断言逮） |
//! | `the_projection_is_value_preserving_dimension_by_dimension` | 五个量纲**各一条**：投影值＝`remaining` 值，含 `None → None`（**不是 `Some(0)`**） | 把某维投影到另一维（**取反档**——逐维值断言红）；把某维的 `None` 折成 `Some(0)`（**移除档**——`None` 侧红） |
//! | `none_is_not_some_zero_on_both_sides` | `None ≠ Some(0)`**两侧各一条**：本层 `Remaining` 侧 ＋ D 侧 `BudgetView` 两个直接构造的值 | 把某维的 `None` 折成 `Some(0)`（**移除档**）；D 侧字段不再是 `Option<i64>`（**编译红**——字面量/断言类型不符） |
//!
//! # 「谁保证」（§12.6.1 的表的照录与订正）
//!
//! 表与订正**逐格写死在** `remaining_and_budget_view_have_the_same_five_dimensions`
//! 的文档注释里（**不能由「编译器」三个字替一个约定背书**）。
//!
//! # 夹具
//!
//! `new_db()` 与 `crates/continuum-budget/tests/tree.rs` 的同名夹具同形（P0 内建迁移 ＋
//! 迁移 120，建库后必须 `migrate()`）。`Remaining` **只能由设计指定的入口得到**（§12.1
//! 三个角色私有、无公开构造函数）：`record_root`（根入账）＋ `remaining`（现算）——
//! 本文件不造第二个构造路径，也**造不出**（那正是 Task 17 的三份反例挡掉的东西）。
//! 夹具的五维取值**两两不同**（`Some(11) / Some(22) / …`）：字段互换一类变异只有在值
//! 不同时才会被值断言逮住（见第一条用例的等价变异体警告）。

// 一行成本：将来把 clippy 纳入门禁时它立刻生效；**今天不生效**——本仓的验证命令是
// `cargo test` 与 `cargo build`，**不含 clippy**，`clippy::` 的 tool lint 在 `cargo test`
// 下被忽略（§12.6.1 的「谁保证」表第 1 行，订正见第一条用例的文档注释）。
#![deny(clippy::rest_pat_in_fully_bound_structs)]

use continuum_budget::{
    BudgetOwner, DimensionKind, Dimensions, Remaining, p4_budget_migrations, record_root, remaining,
};
use continuum_model_registry::BudgetView;
use continuum_persist::{Db, Migration, Tx, builtin_migrations};

/// 五维的固定顺序（与 `DimensionKind` 的枚举声明序一致）——逐维遍历用。
const FIVE: [DimensionKind; 5] = [
    DimensionKind::Money,
    DimensionKind::WallTime,
    DimensionKind::Token,
    DimensionKind::GpuTime,
    DimensionKind::NetworkTransfer,
];

/// 建一个只装了 P0 内建迁移 ＋ 预算迁移 120 的临时库
/// （与 `crates/continuum-budget/tests/tree.rs` 的同名夹具同形）。
fn new_db() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    let mut migrations: Vec<Migration> = builtin_migrations();
    migrations.extend(p4_budget_migrations());
    let db = Db::open_with(&path, migrations).unwrap();
    db.migrate().unwrap();
    (dir, db)
}

/// 造一枚 `BudgetOwner::NodeRun`——**经 `parse` 而不是结构体字面量**：`NodeRun` 的
/// `intent` 字段类型是 `continuum_semantics::IntentId`，而本 task 只登记
/// `continuum-budget` 一条 dev 边（计划登记过的边，不多不少），测试不命名那个类型；
/// `BudgetOwner::parse` 是它自己的公开读法（`as_key` 的逆）。
fn node(tag: &str) -> BudgetOwner {
    let key = serde_json::json!({
        "kind": "node_run",
        "intent": format!("i-{tag}"),
        "node": format!("n-{tag}"),
    })
    .to_string();
    BudgetOwner::parse(&key).expect("夹具自造的键必须是合法编码")
}

/// 一份五维 `Dimensions`（逐维显式给值——免得夹具自己把 `None` 与 `Some(0)` 混掉）。
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

/// 经设计指定的入口得到一枚 `Remaining`：`record_root`（§12.1 的根入账函数）＋
/// `remaining`（§12.2 的现算）。**没有第二条构造路径**（§12.1 三个角色字段私有、
/// 无公开构造函数）。
fn record_remaining(tx: &Tx<'_>, owner: BudgetOwner, fixture: Dimensions) -> Remaining {
    record_root(tx, owner.clone(), fixture, 0).expect("夹具的根入账必然成功");
    remaining(tx, &owner).expect("刚入账的根必然算得出余量")
}

/// `BudgetView` 的逐维读者（那个类型没有按维访问的方法）。
fn budget_view_field(view: &BudgetView, kind: DimensionKind) -> Option<i64> {
    match kind {
        DimensionKind::Money => view.money,
        DimensionKind::WallTime => view.wall_time,
        DimensionKind::Token => view.token,
        DimensionKind::GpuTime => view.gpu_time,
        DimensionKind::NetworkTransfer => view.network_transfer,
    }
}

/// `Remaining` 的逐维读者（`money()` 一族的按维形式）。
fn remaining_field(r: &Remaining, kind: DimensionKind) -> Option<i64> {
    match kind {
        DimensionKind::Money => r.money(),
        DimensionKind::WallTime => r.wall_time(),
        DimensionKind::Token => r.token(),
        DimensionKind::GpuTime => r.gpu_time(),
        DimensionKind::NetworkTransfer => r.network_transfer(),
    }
}

/// **测试内的**恒等投影——生产投影由驱动的接线 task 写（裁定 §12.6.1），本 plan
/// 不预先发明它。本函数的用途只有一个：让「两侧字段集逐项相同」这件事**可断言**，
/// 从而让漂移变成编译错误。
///
/// # 没有 `unproject`，而这是设计的一处必然后果
///
/// §12.1 写死三个角色**各自私有、无公开构造函数**，故**在另一个 crate 的测试里造不出
/// 一个 `Remaining`**——「反向映射」在类型上不可写。于是「两个方向都写」这句话的
/// **可落地形态**是：`BudgetView` 侧穷尽解构一次（本函数的返回值），`Dimensions` 侧
/// 穷尽解构一次（经 `r.dims()`，`r` 由设计指定的入口得到），**两处都不带 `..`**。
/// **这不是把设计要求做小了**：两处解构各自都是编译器强制的穷尽性检查，而「反向造一个
/// `Remaining`」正是 Task 17 那三份反例要挡掉的东西——**若这里能写出 `unproject`，
/// 那三份反例就有一条会通过**。
fn project(r: &Remaining) -> BudgetView {
    BudgetView {
        money: r.money(),
        wall_time: r.wall_time(),
        token: r.token(),
        gpu_time: r.gpu_time(),
        network_transfer: r.network_transfer(),
    }
}

/// **裁定的全部防线**：两侧字段集**逐项相等**——不是「数量相等」（数量相等会让
/// 「`money` 换成 `cost`」漂过去）。
///
/// # 照片机制：编译器强制穷尽的解构，不用 serde，`..` 一个都不许写
///
/// 判据逐条：
///
/// （a）**`..` 一个都不许写**——带 `..` 的模式是**穷尽性豁免**，任一侧加字段它就照过，
/// 那时这条断言与手抄名单**没有区别**（Rust 没有反射：手抄名单恒真，它钉的只是
/// 「抄的时候两边一样」——裁定文件已把这一条记为初版漏项，§12.6.1 有订正）；
/// （b）**不选 serde**：那条路要给 `BudgetView` 加 `#[derive(Serialize)]`，而它的模块文档
/// （`crates/continuum-model-registry/src/budget.rs:36-37`）明写「没有当场消费方的派生
/// （`Debug` / `PartialEq` 等）在这里一律不加」——**为一条测试去改被断言类型的公开形状，
/// 是把断言的成本转嫁给它**；而穷尽解构**不需要任何派生**，且把红的时点从**跑测试**
/// 提前到**编译**；
/// （c）**「两侧字段名逐项相等」不是「数量相等」**（见上）；
/// （d）**为什么要两步**：§12.1 写死「三个角色各自私有、无公开构造函数」，而 §12.6.1 的
/// `let Remaining { money, … } = …` 是**按名解构一个 newtype 的字段**——**两者不能同真**
/// （按名解构 newtype 的字段写不出来）。**本计划按 §12.1 办**：`Remaining` 侧经 `dims()`
/// 读法取 `&Dimensions` 再穷尽解构它。**跨 crate 的 `let Remaining(dims) = r;` 同样不成立**
/// （字段私有），故 §12.1 那两行也只在本 crate 内是照片；**这一处已记 `## 遗留`
/// 第一节第 3、4 条**；
/// （e）**逐维比而不比整个结构体**（§12.6.1 已写死）：`assert_eq!(project(&unproject(&view)), view)`
/// 需要 `BudgetView: PartialEq`，**而本类型不加任何派生**——两处不能同真；逐维比对
/// **不需要派生**，顺带把五个绑定都用上（否则 `unused_variables` 会报）。
///
/// # 「谁保证」逐格写清（§12.6.1 已补这张表——**不能由「编译器」三个字替一个约定背书**）
///
/// | 保证 | 由谁 | 强度 |
/// |---|---|---|
/// | 全绑 ＋ `..` | 「编译器」**——但见下行订正** | 名义上编译不过 |
/// | **部分绑 ＋ `..`**（删掉一个绑定再加 `..`） | **不由编译器保证**（该 lint 只管「全绑了还写 `..`」） | **仅由站点注释 ＋ 复审守** |
/// | 任一侧加字段（在两行都不带 `..` 的前提下） | **编译器**：模式不再穷尽 | 编译不过 |
///
/// **本计划对第一行的订正要说准**：设计写「编译器（需 crate 级
/// `#![deny(clippy::rest_pat_in_fully_bound_structs)]`）」，而**本仓的验证命令是
/// `cargo test` 与 `cargo build`，不含 clippy**——`clippy::` 那条 tool lint 在
/// `cargo test` 下**不生效**，故第一格**实际也回落到「约定」**，与第二格同强。
/// **本计划据此办两件事**：（i）把 `#![deny(clippy::rest_pat_in_fully_bound_structs)]`
/// 写在**本文件头**（成本一行；若将来把 clippy 纳入门禁，它立刻生效），并在那里写明它
/// **今天不生效**；（ii）**不建**那条「读这两行源码、断言不含 `..`」的自定义检查——
/// 设计 §16 第 34 条把它留给实现者，**本计划取「不建」**：它会引入一处读源码的测试，
/// 且本仓已有一处人工复核点（Task 21 Step 3 的源码面复核），把「那两行不得带 `..`」
/// 加进那张清单比再造一个自指的断言便宜。**逐条见 `## 遗留` 第五节第 34 条。**
///
/// # 红的时点（实测）
///
/// 任一侧加一个字段、或改一个维度名 ⇒ **编译不过**（不是运行期红）。
/// 实测（2026-10-15，rustc 1.95.0）：
///
/// - **给 `BudgetView` 加一个字段**（`cpu_time`）：`error[E0027]: pattern does not
///   mention field `cpu_time``（本用例解构那一行）＋ `error[E0063]: missing field
///   `cpu_time` in initializer of `BudgetView``（`project` 的字面量）——**编译器在那条
///   help 里主动提议加 `..`**（「or always ignore missing fields here」），
///   那正是判据 (a) 拦的写法；
/// - **把 `money` 改名为 `cost`**：`error[E0026]: struct `BudgetView` does not have a
///   field named `money`` 与 `error[E0027]: pattern does not mention field `cost``
///   （都在本用例解构那一行），另加 `E0560`（字面量）与 `E0609`（按字段读）。
///
/// 两轮的完整输出：`.tmp/t19-mut-a-add-field.log` 与 `.tmp/t19-mut-b-rename-field.log`。
///
/// # 等价变异体警告
///
/// 把两个 `Option<i64>` 字段**互换**（如 `money` 与 `wall_time` 对调）**在穷尽解构下
/// 编译得过**——那正是下一条用例（`the_projection_is_value_preserving_dimension_by_dimension`）
/// 的作用，**故它不可省**。实测（2026-10-15）：互换后**编译过**、两个用例的逐维值
/// 断言都红（`left: Some(22), right: Some(11)`；日志 `.tmp/t19-mut-c2-swap.log`）。
#[test]
fn remaining_and_budget_view_have_the_same_five_dimensions() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();
    let r = record_remaining(
        &tx,
        node("five-dims"),
        dims(Some(11), Some(22), Some(33), Some(44), Some(55)),
    );

    // 第一行：BudgetView 侧穷尽解构（无 `..`）。任一侧加字段、或改一个维度名，即**编译不过**。
    let BudgetView { money, wall_time, token, gpu_time, network_transfer } = project(&r);
    // 第二行：Remaining 侧——**分两步**解构（三个角色的字段私有，§12.1 的 `let Remaining(dims) = r;`
    // 只在 **crate 内**成立；跨 crate 的读法是 `dims()`），两步都不带 `..`。
    let Dimensions { money: rm, wall_time: rw, token: rt, gpu_time: rg, network_transfer: rn } = r.dims();
    // 逐维比，不比较整个结构体 —— 故**不需要 PartialEq**（见判据 (e)）。五维各一行。
    assert_eq!(money, *rm);
    assert_eq!(wall_time, *rw);
    assert_eq!(token, *rt);
    assert_eq!(gpu_time, *rg);
    assert_eq!(network_transfer, *rn);
}

/// 五个量纲**各一条**：逐维断言投影后的值与 `remaining` 相等，**含 `None` 那一侧**
/// （`None → None`，**不是 `Some(0)`**）。
///
/// # 照片机制（写死）
///
/// **不许对结构体写 `assert_eq!`**——`BudgetView` **不派生 `PartialEq`**
/// （`crates/continuum-model-registry/src/budget.rs:38-44` 无 `#[derive]`），
/// `assert_eq!(project(&unproject(&view)), view)` **编译不过**。故往返断言**逐维比较
/// 五个 `Option<i64>`**（每维一次 `assert_eq!`，比较的是字段值不是结构体）。
/// **这一处是设计 §12.6.1 的代码块与本仓既有代码的相抵**（设计写了那个 `assert_eq!`），
/// **本计划按模块文档办**——这一条**已由设计闭合**（§12.6.1 改为逐维比，见 `## 遗留`
/// 第一节的已闭清单）。
///
/// # 红条件（实测）
///
/// 把 `Remaining` 的 `money` 投影到 `BudgetView` 的 `wall_time`（**取反档**——实测：
/// 编译过；本用例与上一条的逐维值断言都红，`left: Some(22), right: Some(11)`，
/// `.tmp/t19-mut-c2-swap.log`）；含 `None` 那一侧的红条件是「把某维的 `None` 折成
/// `Some(0)`」（**移除档**——实测：编译过；本用例 `left: Some(0), right: None` 与
/// 第三条用例（a）都红，`.tmp/t19-mut-d2-none-to-zero.log`）。夹具取值**两两不同**
/// （`11 / 22 / 33 / 44 / 55`），否则字段互换一类变异成为等价变异体（纪律 1(b)）。
#[test]
fn the_projection_is_value_preserving_dimension_by_dimension() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();

    // 五维全 `Some`、且两两不同。
    let r_some = record_remaining(
        &tx,
        node("five-values"),
        dims(Some(11), Some(22), Some(33), Some(44), Some(55)),
    );
    let view = project(&r_some);

    // 五个量纲各一条：投影后的值与 `remaining` 逐维相等。
    assert_eq!(view.money, r_some.money());
    assert_eq!(view.wall_time, r_some.wall_time());
    assert_eq!(view.token, r_some.token());
    assert_eq!(view.gpu_time, r_some.gpu_time());
    assert_eq!(view.network_transfer, r_some.network_transfer());

    // 含 `None` 那一侧：全 `None` 的分配——`remaining` 五维都不构成约束。
    let r_none = record_remaining(
        &tx,
        node("all-none"),
        dims(None, None, None, None, None),
    );
    let view_none = project(&r_none);

    // 先钉上游事实（本层这侧）：`Remaining` 给的是 `None`（不是 `Some(0)`）——
    // 那一侧的守卫在 `continuum-budget` 的 `dimension_none_is_not_some_zero`。
    assert_eq!(r_none.money(), None);
    assert_eq!(r_none.wall_time(), None);
    assert_eq!(r_none.token(), None);
    assert_eq!(r_none.gpu_time(), None);
    assert_eq!(r_none.network_transfer(), None);

    // 投影保持：`None → None`，**不是 `Some(0)`**。
    assert_eq!(view_none.money, None);
    assert_eq!(view_none.wall_time, None);
    assert_eq!(view_none.token, None);
    assert_eq!(view_none.gpu_time, None);
    assert_eq!(view_none.network_transfer, None);
}

/// `None ≠ Some(0)` **两侧各一条，合起来才算钉住**（裁定 §12.6.1）：
///
/// - （a）**本层侧**：`Remaining` 的 `None` 表示「该维不构成约束」——`None` 读回仍是
///   `None`；
/// - （b）**D 侧**：`BudgetView` 的 `None` 同义（D 已在
///   `crates/continuum-model-registry/src/budget.rs:31-33` 写明）——构造
///   `BudgetView { money: None, … }` 与 `{ money: Some(0), … }` 两个值，断言两者
///   **不被当成同一件事**（逐维比较 `is_none()` / `== Some(0)`）。
///
/// **只钉本层那侧，D 侧把 `None` 当 `Some(0)` 仍会绿**——这是裁定点名的两侧。
/// 转换侧另加一条（`None` 投影成 `None`，不是 `Some(0)`）——含在
/// `the_projection_is_value_preserving_dimension_by_dimension` 的五维各一条里。
///
/// # 红条件与等价变异体警告（实测）
///
/// 把某维的 `None` 折成 `Some(0)`（**移除档**——实测：编译过；本用例（a）红，
/// `left: Some(0), right: None`，`.tmp/t19-mut-d2-none-to-zero.log`）；D 侧把字段类型从
/// `Option<i64>` 换掉（**编译红**——字面量与断言的类型不符）。若夹具里 `None` 与
/// `Some(0)` 恰好输出相同（例如只断言「不 panic」），这条就是**等价变异体**——
/// 故两个值必须让断言结果不同（下面的 `assert_ne!` 就是那个「结果不同」的判据）。
#[test]
fn none_is_not_some_zero_on_both_sides() {
    let (_dir, db) = new_db();
    let tx = db.begin().unwrap();

    // （a）本层侧：`Remaining` 的 `None`（该维不构成约束）——`None` **读回**仍是 `None`。
    let r_none = record_remaining(
        &tx,
        node("none-side"),
        dims(None, None, None, None, None),
    );
    let view_none = project(&r_none);
    for kind in FIVE {
        // 本层读者读回：`None` 仍是 `None`（不是 `Some(0)`）。
        assert_eq!(
            remaining_field(&r_none, kind),
            None,
            "本层侧（{kind:?}）：`Remaining` 的 `None` 读回必须是 `None`"
        );
        // 投影后从 `BudgetView` 读回：仍是 `None`，不是 `Some(0)`。
        assert_eq!(
            budget_view_field(&view_none, kind),
            None,
            "本层侧（{kind:?}）：投影后读回必须是 `None`，不是 `Some(0)`"
        );
    }

    // （b）D 侧：直接构造两个 `BudgetView` 值——两行字面量本身就是编译期照片
    // （五维缺一个即 `E0063`；某维不是 `Option<i64>` 即类型错）。
    let none_side = BudgetView {
        money: None,
        wall_time: None,
        token: None,
        gpu_time: None,
        network_transfer: None,
    };
    let zero_side = BudgetView {
        money: Some(0),
        wall_time: Some(0),
        token: Some(0),
        gpu_time: Some(0),
        network_transfer: Some(0),
    };
    for kind in FIVE {
        // `None` 侧：该维不构成约束。
        assert!(
            budget_view_field(&none_side, kind).is_none(),
            "D 侧（{kind:?}）：`None` 侧该维必须 is_none()"
        );
        // `Some(0)` 侧：额度为零。
        assert_eq!(
            budget_view_field(&zero_side, kind),
            Some(0),
            "D 侧（{kind:?}）：`Some(0)` 侧该维必须恰是 `Some(0)`"
        );
        // 两者不被当成同一件事：同一个判别对两个值必须给出**不同结果**。
        assert_ne!(
            budget_view_field(&none_side, kind).is_none(),
            budget_view_field(&zero_side, kind).is_none(),
            "D 侧（{kind:?}）：`None` 与 `Some(0)` 必须做出不同判别——同值即等价变异体"
        );
    }
}
