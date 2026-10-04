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

use continuum_effect::EffectType;
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
/// 「无匹配」有三条互不相同的路径，都断言：规则集为空、条件所引用的事实不在
/// 上下文中（`src/rule.rs:183` 的 `observe` 给出 `None`）、以及事实在上下文中但
/// **取值不符**（`src/rule.rs:189` 的比较给出 `false`）。后两条是 fail-open 的常见
/// 入口——若把「条件不成立」当成放行的理由，空规则集那条仍会过，只有它们会露出来。
/// 最后两条走的是不同的分支，故各写一条：只写其中一条时，另一条分支被改成什么
/// 都不会有对照片。
#[test]
fn no_matching_rule_denies() {
    assert_eq!(
        decide(&[], &unapproved()),
        Decision::Deny,
        "空规则集应默认拒绝"
    );

    // 同一条件，两个方向：事实缺省、事实有值但不符。
    let condition = Condition::parse(&json!({"fact": "effect_type", "eq": "charge"}))
        .expect("已知事实、已知比较符、类型相符，应可解析");
    let only_rule = |condition: Condition| {
        vec![Policy {
            level: Level::UserPersistent,
            condition,
            decision: Decision::Allow,
            scope: Scope::User,
        }]
    };

    assert_eq!(
        decide(&only_rule(condition.clone()), &unapproved()),
        Decision::Deny,
        "事实不在上下文中时该规则不成立，应默认拒绝而非取该规则的 Allow"
    );

    let differing = PolicyContext {
        effect_type: Some(EffectType::PushBranch),
        ..PolicyContext::default()
    };
    assert_eq!(
        decide(&only_rule(condition), &differing),
        Decision::Deny,
        "事实有值但取值不符时该规则同样不成立，应默认拒绝而非取该规则的 Allow"
    );
}

/// 与 `decide` 的文档说明互为对照：`--approve` 已给出时返回 `Deny` 只有两种含义。
///
/// - 第 1 级 `Deny` 成立（第 2 级规则已在表里）；
/// - 表里没有第 2 级规则——此时 `--approve` 不参与（`Deny` 是「无更高的规则放行」）。
///
/// 驱动据返回值决定是否铸造批准值。若把 `Deny` 按设计 §5.7 那一行的字面读法处理
/// （有 `--approve` 即铸造），第一种含义就会 fail-open：第 1 级 `Deny` 加
/// `--approve` 铸造出批准值，第 1 级失去不可越性。故这个「只有两种含义」必须被
/// 钉住，而不是只写在注释里——每一侧都有断言。
#[test]
fn with_an_explicit_approval_a_deny_comes_only_from_the_first_level() {
    // 第 1 级 Deny 在场：它高于第 2 级，故 Deny 留存——这正是「不可越」。
    let first_level_deny = vec![
        always(Level::SystemSafety, Decision::Deny),
        always(Level::UserPersistent, Decision::Allow),
        explicit_current_allow(),
    ];
    assert_eq!(
        decide(&first_level_deny, &approved()),
        Decision::Deny,
        "第 1 级的 Deny 高于第 2 级，应留存"
    );

    // 第 1 级 Deny 不在场：第 3 级的 Deny 被第 2 级越过，`--approve` 已给出时
    // 裁决不得留下 Deny。
    let lower_level_deny = vec![
        always(Level::UserPersistent, Decision::Deny), // 第 3 级
        always(Level::RuntimeDefault, Decision::RequireApproval), // 第 5 级
        explicit_current_allow(),
    ];
    assert_eq!(
        decide(&lower_level_deny, &approved()),
        Decision::Allow,
        "第 1 级 Deny 不在场时，第 3 级的 Deny 应被第 2 级越过，不留下 Deny"
    );

    // 第二种含义：表里没有第 2 级规则时本函数无从知道 `--approve` 的存在，故
    // 第 3 级的 Deny 照旧留存——这是「漏放第 2 级规则」的失误面，方向是
    // fail-closed（`--approve` 失效），与上一条的 fail-open 相反。
    assert_eq!(
        decide(
            &[always(Level::UserPersistent, Decision::Deny)],
            &approved()
        ),
        Decision::Deny,
        "表里没有第 2 级规则时，--approve 不参与，第 3 级的 Deny 留存"
    );

    // 同一含义的极端情形：表是空的。上面那条断言「有规则但不放行」，这条断言
    // 「根本没有规则」，两者走的是裁决里不同的两支（有胜者、无胜者）。
    assert_eq!(
        decide(&[], &approved()),
        Decision::Deny,
        "表里没有第 2 级规则时，--approve 不改变「无匹配即拒绝」的结论"
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

/// 落库的 `Deny` 可被 `--approve` 越过（设计下篇第 5.5、5.7 节）。
///
/// 用例体只摆**第 3 级**（`UserPersistent`）一条规则。**这不是缺照片**：
/// `engine.rs` 的比较靠 `Level` 派生的 `Ord`，没有按层级手写的臂，故一个代表臂在
/// 机制上就足以覆盖第 3–5 级这一整段。用例名原写「第 3–5 级」，名比体宽，已改为
/// 不代表具体级数的说法。
#[test]
fn an_explicit_approval_outranks_the_persisted_deny() {
    let policies = vec![
        always(Level::UserPersistent, Decision::Deny), // 第 3 级
        explicit_current_allow(),
    ];

    assert_eq!(
        decide(&policies, &approved()),
        Decision::Allow,
        "第 2 级高于落库的第 3 级：--approve 给出时应越过它那条 Deny"
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
