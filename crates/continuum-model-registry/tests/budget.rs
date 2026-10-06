//! Task 9：§333 的只读预算投影 [`BudgetView`] 的用例。
//!
//! 每条断言的「红的条件」见各用例的注释——这些注释不是说明，是变异时的靶子。
//!
//! **本文件的用例全部在 crate 之外**：构造走结构体字面量，故「字段是 `pub` 的」这件事
//! 由编译器守，而不靠本层自觉。

use continuum_model_registry::BudgetView;

/// §333 的五个量纲**逐项**给值并读回：字段名、字段类型（`Option<i64>`）与取值各钉一次。
///
/// **五个字段各有一条照片**（枚举式断言逐项有照片）：五条 `let _: Option<i64> = …`
/// 各钉一个字段的**类型**，五条 `assert_eq!` 各钉一个字段的**取值**。取值互不相同
/// （11/22/33/44/55），故「读的时候串了字段」在运行期就会红，不会被「两个字段恰好相等」掩盖。
///
/// **红的条件**：任一字段改名或改类型 → 本用例**编译不过**。本类型的字段名与字段类型
/// 由编译器守（这是纯数据投影的承重形态，见 `the_view_is_a_plain_data_projection` 的注释）；
/// 运行期能观察的只有「读回时串了字段」这一类错误，它由上面那组互不相同的取值钉住。
#[test]
fn the_view_carries_the_five_dimensions_of_333() {
    let view = BudgetView {
        money: Some(11),
        wall_time: Some(22),
        token: Some(33),
        gpu_time: Some(44),
        network_transfer: Some(55),
    };

    // 类型逐项钉住：把任一字段从 `Option<i64>` 改成别的类型，本行编译不过。
    let money: Option<i64> = view.money;
    let wall_time: Option<i64> = view.wall_time;
    let token: Option<i64> = view.token;
    let gpu_time: Option<i64> = view.gpu_time;
    let network_transfer: Option<i64> = view.network_transfer;

    // 取值逐项读回（次序照 §333）。
    assert_eq!(money, Some(11), "money（§333 第 1 个量纲）");
    assert_eq!(wall_time, Some(22), "wall_time（§333 第 2 个量纲）");
    assert_eq!(token, Some(33), "token（§333 第 3 个量纲）");
    assert_eq!(gpu_time, Some(44), "gpu_time（§333 第 4 个量纲）");
    assert_eq!(
        network_transfer,
        Some(55),
        "network_transfer（§333 第 5 个量纲）"
    );
}

/// `None` 与 `Some(0)` 是**两件事**，五个量纲**逐项**各钉两侧。
///
/// `None` = 该量纲当前**不构成约束**；`Some(0)` = **额度为零**。二者混同会让
/// 「这一项不参与筛选」变成「一分钱都不能花」——后果不是少一个筛选项，是候选全被挡。
///
/// **两侧各有一条断言，且逐项各来一遍**：不构成约束的一侧判 [`Option::is_none`]，
/// 额度为零的一侧判相等 `Some(0)`。**「不得相等」那一对是这条判据的直接照片**——
/// 它正是「把 `None` 折成 `0`（或反之）就会红」这句的落点。
///
/// **夹具让两者可分辨**：`unconstrained` 五个字段全 `None`，`zeroed` 五个字段全
/// `Some(0)`，两者在 `Option<i64>` 的 `PartialEq` 下必然不等，故不存在
/// 「`None` 与 `Some(0)` 在这条断言下同结果」的等价变异体。
///
/// **红的条件**：任一把 `None` 折成 `Some(0)`（反之亦然）的改动落到本类型可观察的取值上，
/// 都会让「不得相等」那一对断言里的一条红；只写一侧（例如只判 `is_none`、不判 `Some(0)`）
/// 则等于只钉住一半，另一半的回归无人看得见。
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

    // 一侧：该量纲**不构成约束**。
    assert!(unconstrained.money.is_none(), "money：None = 当前不构成约束");
    assert!(
        unconstrained.wall_time.is_none(),
        "wall_time：None = 当前不构成约束"
    );
    assert!(unconstrained.token.is_none(), "token：None = 当前不构成约束");
    assert!(
        unconstrained.gpu_time.is_none(),
        "gpu_time：None = 当前不构成约束"
    );
    assert!(
        unconstrained.network_transfer.is_none(),
        "network_transfer：None = 当前不构成约束"
    );

    // 另一侧：**额度为零**。
    assert_eq!(zeroed.money, Some(0), "money：Some(0) = 额度为零");
    assert_eq!(zeroed.wall_time, Some(0), "wall_time：Some(0) = 额度为零");
    assert_eq!(zeroed.token, Some(0), "token：Some(0) = 额度为零");
    assert_eq!(zeroed.gpu_time, Some(0), "gpu_time：Some(0) = 额度为零");
    assert_eq!(
        zeroed.network_transfer,
        Some(0),
        "network_transfer：Some(0) = 额度为零"
    );

    // 「这是两件事」的直接照片：逐项不得互相折合。
    assert_ne!(
        unconstrained.money,
        Some(0),
        "money：「不构成约束」不得被折成「额度为零」"
    );
    assert_ne!(
        unconstrained.wall_time,
        Some(0),
        "wall_time：「不构成约束」不得被折成「额度为零」"
    );
    assert_ne!(
        unconstrained.token,
        Some(0),
        "token：「不构成约束」不得被折成「额度为零」"
    );
    assert_ne!(
        unconstrained.gpu_time,
        Some(0),
        "gpu_time：「不构成约束」不得被折成「额度为零」"
    );
    assert_ne!(
        unconstrained.network_transfer,
        Some(0),
        "network_transfer：「不构成约束」不得被折成「额度为零」"
    );
    assert_ne!(
        zeroed.money,
        None,
        "money：「额度为零」不得被折成「不构成约束」"
    );
    assert_ne!(
        zeroed.wall_time,
        None,
        "wall_time：「额度为零」不得被折成「不构成约束」"
    );
    assert_ne!(
        zeroed.token,
        None,
        "token：「额度为零」不得被折成「不构成约束」"
    );
    assert_ne!(
        zeroed.gpu_time,
        None,
        "gpu_time：「额度为零」不得被折成「不构成约束」"
    );
    assert_ne!(
        zeroed.network_transfer,
        None,
        "network_transfer：「额度为零」不得被折成「不构成约束」"
    );
}

/// 这是一个**纯数据投影**：字段 `pub`、驱动直接构造、**不经 trait、不经校验**。
///
/// 本用例在 crate 之外用**结构体字面量**构造它，这一条同时钉住三件事：
///
/// 1. 五个字段都是 `pub`——有一个私有，字面量就构造不出来；
/// 2. **没有构造闸门**——没有 `try_new`、没有校验（有闸门就只能经闸门构造）；
/// 3. **没有「实现者」这一说**——本层不为它定义 trait，故没有东西要「实现」它
///    （**不定义 trait 是刻意的**：没有实现者的 trait 是**假接口**，设计 §6.1）。
///    第 3 条**写不出直接断言**（不存在的名字写不进用例），能给出的照片是
///    **这个类型照常可用**：构造、读字段、按值/按引用传递，全程不需要任何 trait 在作用域里。
///
/// **「不经校验」不是一句空话**：取值域规范未定义（§333 只给字段名），本层不解释、不换算、
/// 不记账，故负值与 `i64::MAX` 都原样穿过——下方断言读回的正是原值。
///
/// **红的条件**：任一把字段收成私有、或把构造挪到某个 `try_new` / 校验函数之后，
/// 本用例**编译不过**。本类型的承重形态是编译期，运行期可观察的只有「值原样读回」。
#[test]
fn the_view_is_a_plain_data_projection() {
    // 「驱动直接构造」：无 trait、无校验、无中间函数。负值与极大值都不被拒、不被夹紧，
    // 因为本层不解释取值域。
    let view = BudgetView {
        money: Some(-1),
        wall_time: None,
        token: Some(i64::MAX),
        gpu_time: Some(0),
        network_transfer: None,
    };

    // 值原样穿过：证明「不经校验」不是一句空话。
    assert_eq!(view.money, Some(-1), "未经校验：负值原样读回");
    assert_eq!(view.token, Some(i64::MAX), "未经校验：i64::MAX 原样读回");

    // 使用这个类型不需要任何 trait：按值与按引用都只是复制 / 借用。
    fn takes_by_value(view: BudgetView) -> BudgetView {
        view
    }
    fn takes_by_ref(view: &BudgetView) -> Option<i64> {
        view.gpu_time
    }
    assert_eq!(
        takes_by_ref(&takes_by_value(view)),
        Some(0),
        "传参与读字段都不经任何 trait"
    );
}
