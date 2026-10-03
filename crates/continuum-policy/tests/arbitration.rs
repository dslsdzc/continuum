//! 六级裁决（设计下篇第 5.3、5.5、5.7 节）。
//!
//! 本文件的重点是两条**方向相反、缺一不可**的守卫：第 1 级 `Deny` 不可被
//! `--approve` 越过，而第 3–5 级的 `Deny` 可以被越过。只钉前者的用例在
//! 「`--approve` 从不生效」的实现下也照过，只钉后者的用例在 fail-open 的实现下
//! 也照过；设计第 5.5 节的取舍正落在这两条的交界上。
//!
//! 另一条承重的前置条件：第 2 级那条规则**由调用方放进列表**，`decide` 不自行
//! 合成。故下面凡涉及 `--approve` 的用例都显式构造它（见 [`explicit_current_allow`]），
//! 而不是假设裁决函数知道 `--approve` 的存在。

use continuum_policy::{
    Condition, Decision, ExplicitApproval, Level, Policy, PolicyContext, Scope, decide,
};
use serde_json::json;

/// 一条条件恒真（空合取）的规则。
fn always(level: Level, decision: Decision) -> Policy {
    Policy {
        level,
        condition: Condition::parse(&json!({"all": []})).expect("空合取应可解析"),
        decision,
        scope: Scope::User,
    }
}

/// 第 2 级的那条规则：`--approve` 给出时成立的 `Allow`。
///
/// **它是普通条目，由调用方放进列表**——`decide` 不对规则集做任何增删（设计下篇
/// 第 5.2 节：第 2 级在下篇由驱动的显式确认占位）。带条件而非无条件，是这条规则
/// 唯一要紧的性质：无条件的第 2 级 `Allow` 会在没有 `--approve` 时照样成立，
/// 把第 3–5 级的 `Deny` 一律推翻，即 fail-open。
fn explicit_current_allow() -> Policy {
    Policy {
        level: Level::ExplicitCurrent,
        condition: Condition::parse(&json!({"fact": "explicit_current", "eq": true}))
            .expect("已知事实、已知比较符、类型相符，应可解析"),
        decision: Decision::Allow,
        scope: Scope::User,
    }
}

/// `--approve` 已给出。
fn approved() -> PolicyContext {
    PolicyContext {
        explicit_current: Some(ExplicitApproval),
        ..PolicyContext::default()
    }
}

/// `--approve` 未给出。其余事实也一律缺省。
fn unapproved() -> PolicyContext {
    PolicyContext::default()
}

/// 同层裁决的取值，并顺带钉住它与规则排列次序无关。
///
/// 单独断言两个次序：同层取严若写成「后一条覆盖前一条」，正的次序照样给出
/// `Deny`，只有反的次序会露出来。
fn same_level(a: Decision, b: Decision) -> Decision {
    let forward = vec![
        always(Level::UserPersistent, a),
        always(Level::UserPersistent, b),
    ];
    let backward = vec![
        always(Level::UserPersistent, b),
        always(Level::UserPersistent, a),
    ];

    let forward_result = decide(&forward, &unapproved());
    let backward_result = decide(&backward, &unapproved());
    assert_eq!(
        forward_result, backward_result,
        "同层裁决不应依赖规则的排列次序：{a:?} 与 {b:?}"
    );
    forward_result
}

/// 所有条件成立的规则中取最高层（设计下篇第 5.3 节）。
///
/// `Level` 的判别值越小越强（`Level` 的文档：数字即顺序，越大越弱），故「最高层」
/// 是判别值**最小**的那一级。写成取最大判别值的实现会在此处给出 `Deny`。
#[test]
fn the_highest_applicable_level_wins() {
    let policies = vec![
        always(Level::UserPersistent, Decision::Allow), // 第 3 级
        always(Level::RuntimeDefault, Decision::Deny),  // 第 5 级
    ];

    assert_eq!(
        decide(&policies, &unapproved()),
        Decision::Allow,
        "第 3 级高于第 5 级：两级的条件都成立时应由第 3 级的 Allow 获胜"
    );
}

/// 同层内 `Deny > RequireApproval > Allow`（设计下篇第 5.3 节）。
///
/// 三对都要断言：只断言其中一对的实现可能把三值只做成两档（例如把
/// `RequireApproval` 与 `Allow` 并列），此时未被断言的那一对才是缺口。
#[test]
fn within_a_level_the_stricter_decision_wins() {
    assert_eq!(
        same_level(Decision::Allow, Decision::Deny),
        Decision::Deny,
        "同层内 Deny 比 Allow 严"
    );
    assert_eq!(
        same_level(Decision::Allow, Decision::RequireApproval),
        Decision::RequireApproval,
        "同层内 RequireApproval 比 Allow 严"
    );
    assert_eq!(
        same_level(Decision::RequireApproval, Decision::Deny),
        Decision::Deny,
        "同层内 Deny 比 RequireApproval 严"
    );
}

/// 无任何规则匹配时默认 `Deny`（设计下篇第 5.3 节）。
///
/// 两种「无匹配」都要断言：规则集为空，以及规则集非空但条件全不成立。后者是
/// fail-open 的常见入口——若把「条件不成立」与「条件无从求值」当成放行的理由，
/// 空规则集那条仍会过，只有这一条会露出来。
#[test]
fn no_matching_rule_denies() {
    assert_eq!(
        decide(&[], &unapproved()),
        Decision::Deny,
        "空规则集应默认拒绝"
    );

    // 条件引用 effect_type，而上下文不含该事实：该条不成立（设计下篇第 5.3 节），
    // 不退化为「全部放行」。
    let unmatched = vec![Policy {
        level: Level::UserPersistent,
        condition: Condition::parse(&json!({"fact": "effect_type", "eq": "charge"}))
            .expect("已知事实、已知比较符、类型相符，应可解析"),
        decision: Decision::Allow,
        scope: Scope::User,
    }];
    assert_eq!(
        decide(&unmatched, &unapproved()),
        Decision::Deny,
        "唯一的规则条件不成立时，应默认拒绝而非取该规则的 Allow"
    );
}

/// 第 1 级 `Deny` 不可被 `--approve` 越过（设计下篇第 5.5 节）。
///
/// 列表里同时放入第 2 级那条 `Allow` 且它的条件成立，第 1 级仍是 `Deny`：
/// 这样断言的才是「第 1 级高于第 2 级」，而不是「列表里没有第 2 级的规则」。
#[test]
fn the_first_level_cannot_be_overridden() {
    let policies = vec![
        always(Level::SystemSafety, Decision::Deny),
        explicit_current_allow(),
    ];

    assert_eq!(
        decide(&policies, &approved()),
        Decision::Deny,
        "--approve 已给出、第 2 级的 Allow 成立，仍不能越过第 1 级的 Deny"
    );
}

/// 第 3–5 级的 `Deny` 可被 `--approve` 越过（设计下篇第 5.5、5.7 节）。
#[test]
fn an_explicit_approval_outranks_levels_three_to_five() {
    let policies = vec![
        always(Level::UserPersistent, Decision::Deny), // 第 3 级
        explicit_current_allow(),
    ];

    assert_eq!(
        decide(&policies, &approved()),
        Decision::Allow,
        "第 2 级高于第 3 级：--approve 给出时应越过第 3 级的 Deny"
    );
}

/// 上一条的反面：`--approve` 未给出时，第 2 级那条规则**不成立**，第 3 级的
/// `Deny` 原样站立。
///
/// 这条与上一条构成本项目要求的两侧守卫。缺了它，「第 2 级规则写成无条件
/// `Allow`」——即 `--approve` 永远生效——这一实现会全绿，而它正是设计第 5.5 节
/// 明说的 fail-open 边界。
#[test]
fn without_an_explicit_approval_the_level_two_rule_does_not_apply() {
    let policies = vec![
        always(Level::UserPersistent, Decision::Deny), // 第 3 级
        explicit_current_allow(),
    ];

    assert_eq!(
        decide(&policies, &unapproved()),
        Decision::Deny,
        "--approve 未给出时第 2 级的 Allow 不成立，第 3 级的 Deny 应保持不变"
    );
}

/// `scope` 不参与裁决（设计下篇第 5.3 节只按层级与决策取严；`scope` 记的是规则的
/// 来源，供存储与审计用）。
///
/// 两个方向都断言：同层内把 Scope 对调，结果不变。只断言一个方向的用例在
/// 「User 优先」的实现下会红，但在「Project 优先」的实现下照过。
#[test]
fn scope_does_not_participate_in_arbitration() {
    let rule = |decision: Decision, scope: Scope| Policy {
        level: Level::UserPersistent,
        condition: Condition::parse(&json!({"all": []})).expect("空合取应可解析"),
        decision,
        scope,
    };

    let user_allow = vec![
        rule(Decision::Allow, Scope::User),
        rule(Decision::Deny, Scope::Project),
    ];
    let project_allow = vec![
        rule(Decision::Allow, Scope::Project),
        rule(Decision::Deny, Scope::User),
    ];

    assert_eq!(
        decide(&user_allow, &unapproved()),
        Decision::Deny,
        "同层内 Scope 为 User 的 Allow 不应压过 Scope 为 Project 的 Deny"
    );
    assert_eq!(
        decide(&project_allow, &unapproved()),
        Decision::Deny,
        "同层内 Scope 为 Project 的 Allow 不应压过 Scope 为 User 的 Deny"
    );
}
