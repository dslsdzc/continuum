//! Task 9：§333 的只读预算投影 [`BudgetView`] 的用例。
//!
//! # 本文件只钉**类型与签名**，不写运行期用例
//!
//! 设计 §10 第 2 条（`docs/superpowers/specs/2026-10-05-p3d-model-registry-router-design.md:1218-1220`）
//! 明写：语义层未建，预算视图的真实语义在本阶段**不可观察**，**故不为它写运行期用例，只钉类型与签名**。
//! 根因是**本层的这个类型没有实现体**——它是一个纯数据投影（字段 `pub`、无方法、无判定、无派生），
//! **从构造到读回之间没有一行本 crate 的代码**。故任何「读回来还是原值吗」的运行期断言都**必然恒真**：
//! 它验的是 `Option<i64>` 自己的语义，**不是本 crate 的任何东西**，任何不导致编译错误的实现改动都改不动它。
//! 那些断言已删——**不是「用例没用」，是恒真的断言不构成证据**（删掉的来历即上一句）。
//!
//! # 本文件里真正的照片是编译期
//!
//! `tests/` 是**独立的 crate**。它能用**结构体字面量**构造出 [`BudgetView`]，
//! 就证明了「五个字段都是 `pub`」——**这是一条真的编译期照片**（去掉一个 `pub` 即 `E0616`）。
//! 三条用例全部靠**编译通过 / 编译不过**取证据：
//!
//! 1. 字面量里**逐项**写出五个字段名、再用 `let _: Option<i64> = …` 逐项钉类型，
//!    故字段名、字段类型、以及「字段**恰好是这五个**」都被编译器钉住；
//! 2. 字段名 / 类型 / 可见性 / 字段个数任改一处 → 本文件**编译不过**。
//!
//! **于是本 task 的红的形态是编译错误，不是断言失败**。按本项目纪律「编译不过的变异不算变红」，
//! **本 task 因此没有一条「变红」的变异**——据实标注，见实现报告 §3。

use continuum_model_registry::BudgetView;

/// §333 的五个量纲**名与类型逐项**钉住。
///
/// 做法：字面量里逐项写字段名（字段名写错即编译不过），再用 `let _: Option<i64> = …` 逐项钉**类型**
/// （类型改掉即编译不过）。**本用例没有运行期断言**——本类型无实现体，任何「读回是否相等」的断言
/// 都恒真（文件头已写明来历），故不写。能红的只有编译期：字段改名、改类型、少一个、多一个。
#[test]
fn the_view_carries_the_five_dimensions_of_333() {
    let view = BudgetView {
        money: None,
        wall_time: None,
        token: None,
        gpu_time: None,
        network_transfer: None,
    };

    // 五个量纲逐项：名写在字面量里，类型钉在这里（次序照 §333）。
    let _: Option<i64> = view.money;
    let _: Option<i64> = view.wall_time;
    let _: Option<i64> = view.token;
    let _: Option<i64> = view.gpu_time;
    let _: Option<i64> = view.network_transfer;
}

/// `None`（该量纲当前**不构成约束**）与 `Some(0)`（**额度为零**）是两件事。
///
/// # 本条判据的守卫**不在本 crate**
///
/// 本 crate 只保证**装得下**这个区别，**不保证没人把它抹掉**：折叠会发生在**驱动侧的投影**里
/// （设计 §6.1：投影由驱动做，不由语义层做），本 crate 里**没有可施加该变异的实现体**。
/// 真正必须守住「不把 `None` 读成额度为零」的是**投影方与消费方**——计划 `## 遗留` 的同名条
/// 已把这一点逐条记下，收件人是驱动侧的投影实现与子项目 G。
///
/// # 故本用例只钉编译期
///
/// 钉的是「两种取值都装得下、且五个量纲都装得下」：两个字面量各构造一次，逐项钉 `Option<i64>`。
/// **这里不写 `assert_ne!(view.money, Some(0))` 之类的断言**——那是恒真的
/// （`Option` 自己的偏等，中间没有本 crate 的代码），删掉的来历见文件头。
#[test]
fn none_is_not_the_same_as_zero() {
    let unconstrained = BudgetView {
        money: None,
        wall_time: None,
        token: None,
        gpu_time: None,
        network_transfer: None,
    };
    let zeroed = BudgetView {
        money: Some(0),
        wall_time: Some(0),
        token: Some(0),
        gpu_time: Some(0),
        network_transfer: Some(0),
    };

    // 「不构成约束」的五项都装得下，类型都是 `Option<i64>`。
    let _: Option<i64> = unconstrained.money;
    let _: Option<i64> = unconstrained.wall_time;
    let _: Option<i64> = unconstrained.token;
    let _: Option<i64> = unconstrained.gpu_time;
    let _: Option<i64> = unconstrained.network_transfer;

    // 「额度为零」的五项也都装得下，类型同样是 `Option<i64>`。
    let _: Option<i64> = zeroed.money;
    let _: Option<i64> = zeroed.wall_time;
    let _: Option<i64> = zeroed.token;
    let _: Option<i64> = zeroed.gpu_time;
    let _: Option<i64> = zeroed.network_transfer;
}

/// 这是一个**纯数据投影**：五个字段 `pub`、由驱动直接构造、**不经任何 trait**。
///
/// **唯一的照片是编译期**：本用例在 crate 之外用**结构体字面量**列出**全部五个字段**。能这么写就证明：
///
/// 1. 五个字段**都是 `pub`**（去掉一个 `pub` 即 `E0616`）；
/// 2. 字段集合**恰好是这五个**——字面量**不能少写**一个（`E0063`），也没有第六个字段可写；
/// 3. **构造不需要任何 trait 在作用域里**（本文件通篇没有一个 `impl` / 一个 trait 名）。
///
/// **订正（错误说法的来历留在原地）**：本用例先前写「（有闸门就只能经闸门构造）」——**那句是错的**。
/// 一个显式闸门（如 `try_new`）**不拦**结构体字面量；字面量构造不过去，唯一的原因是**字段私有**。
/// 故本用例的 (1)(2) 是字面量能证出来的，而「本层不另设构造便利函数」**证不出来**，先前那句已删，
/// 不以它作为本条判据的一部分。
///
/// **不经校验**：字面量里给什么值就是什么值，本层不解释、不换算、不记账（§333 未给取值域与单位）。
/// 这同样是**编译期事实**（`pub` 字段的读写之间没有本 crate 的代码），故**不用运行期断言去证**。
#[test]
fn the_view_is_a_plain_data_projection() {
    let _ = BudgetView {
        money: Some(11),
        wall_time: Some(22),
        token: Some(33),
        gpu_time: Some(44),
        network_transfer: Some(55),
    };
}
