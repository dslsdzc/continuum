//! 写策略侧自查（QA 摸底报告 §三 观察 1 的跟进）。
//!
//! # 背景
//!
//! 「一条写错的拦类规则是 fail-open 形状」——QA 对抗轮拍下两个组合：第 1 级
//! `Deny` 的条件写成**空 `in` 列表**（恒不成立），或**引用缺省事实**（在该上下文
//! 里不成立），于是最高级的拒绝静默失效、裁决落到更低层的 `Allow`。两处解析语义
//! 各由 `condition.rs` 的 `an_empty_in_list_never_matches` 与
//! `an_absent_fact_does_not_match` 钉住，但「组合到裁决上的后果」此前没有守卫。
//!
//! # 本文件是什么、不是什么
//!
//! 是：一个**写策略时的自查**（[`audit_rule`]）加一组用例。它不碰裁决器
//! （`engine::decide` 与 `Condition::matches` 一字不改）、不扩任何公开类型
//! （自查函数与报告类型都只活在本测试文件里）。它回答的问题是：
//!
//! > 给定**该规则将运行的调用路径会提供的事实集**，这条规则有没有
//! > 「永不可能生效 / 会静默失效」的形状？
//!
//! 若未来做写策略的 lint／CLI（例如保存前自检），把 [`audit_rule`] 与
//! [`Finding`] 搬过去即可——它们只用公开 API（`Condition::to_json`），
//! 不需要动 `continuum-policy` 的公开面。
//!
//! # 判别力的边界（写死，不是规范）
//!
//! - 只报**拦类决策**（`Deny` / `RequireApproval`）：它们的条件不成立 = 该拦的
//!   没拦（fail-open）。`Allow` 的条件不成立只是少放行（fail-closed），不报；
//! - 报两种形状：**空 `in` 列表**（整条条件恒不成立，与上下文无关）与
//!   **引用 `explicit_current` 之外、又不在给定事实集里的事实**（缺省即不成立，
//!   见 `rule.rs` 的可观察性表）；
//! - 不报别的：事实名写错、比较符不适用、取值类型不符在解析期已是 `Err`
//!   （`Condition::parse`），轮不到自查；`explicit_current` 恒可观察，
//!   取 `true` 或 `false` 都有其含义，不报。
//!
//! 调用路径的事实集（本仓现状，写用例时的参照）：
//! 集成裁决 = `{"explicit_current"}`（`policy_context`）；逐条效应裁决 =
//! `{"explicit_current", "effect_type"}`（`policy_context_for_effect`）。

use continuum_effect::EffectType;
use continuum_policy::{
    decide, Condition, Decision, ExplicitApproval, Level, Policy, PolicyContext, Scope,
};
use serde_json::{json, Value};

/// 一条规则的自查发现。只活在本文件（不是公开 API）。
#[derive(Debug, Clone, PartialEq, Eq)]
enum Finding {
    /// 条件里有空 `in` 列表 ⇒ 整条条件恒不成立（合取中一假即假）。
    EmptyInList,
    /// 条件引用了「缺省即不成立」的事实，而该事实不在给定的事实集里 ⇒
    /// 在这条调用路径上此条不成立。
    FactNotInContext { fact: String },
}

/// 写策略自查：按给定的事实集审一条规则，返回「静默失效」的形状清单。
///
/// `facts_in_play` 是**该规则将运行的调用路径会提供的事实名**（上面文档列了本仓
/// 现状的两处）。空清单表示「该路径只提供 `explicit_current`」。
///
/// 判定用的全是公开 API：`Policy` 的三个字段与 `Condition::to_json()` 的归一化
/// 形状（`{"all": [...]}`）。不调 `Condition::matches`、不构造 `PolicyContext`——
/// 自查是**静态**的，不扮演裁决器。
fn audit_rule(policy: &Policy, facts_in_play: &[&str]) -> Vec<Finding> {
    // 只报拦类决策；`Allow` 的条件不成立是 fail-closed（见文件文档）。
    if !matches!(policy.decision, Decision::Deny | Decision::RequireApproval) {
        return Vec::new();
    }

    let json = policy.condition.to_json();
    let predicates = json["all"]
        .as_array()
        .expect("`Condition::to_json` 恒产出 {\"all\": [...]}（其文档钉的归一化形状）");

    let mut findings = Vec::new();
    for predicate in predicates {
        let fact = predicate["fact"]
            .as_str()
            .expect("谓词恒有 `fact` 键（解析期保证）");

        if let Some(items) = predicate.get("in").and_then(Value::as_array) {
            if items.is_empty() {
                findings.push(Finding::EmptyInList);
            }
        }

        // `explicit_current` 总可观察；其余四个事实缺省即不成立
        // （`rule.rs` 的可观察性表）。
        if fact != "explicit_current" && !facts_in_play.contains(&fact) {
            findings.push(Finding::FactNotInContext {
                fact: fact.to_owned(),
            });
        }
    }
    findings
}

fn rule(level: Level, decision: Decision, condition: Value) -> Policy {
    Policy {
        level,
        condition: Condition::parse(&condition).expect("条件应可解析"),
        decision,
        scope: Scope::User,
    }
}

/// 组合 1（QA 对抗轮的照片）：空 `in` 列表的拦类规则必须被自查报告。
///
/// **红条件**：删掉 [`audit_rule`] 里 `EmptyInList` 的检查（移除档）——本用例红。
#[test]
fn an_empty_in_list_on_a_blocking_rule_is_reported() {
    let miswritten = rule(
        Level::SystemSafety,
        Decision::Deny,
        json!({"fact": "effect_type", "in": []}),
    );
    // 该规则跑在逐条效应裁决的路径上：`effect_type` 在场——即便如此，
    // 空 `in` 仍恒不成立。
    let findings = audit_rule(&miswritten, &["effect_type"]);
    assert!(
        findings.contains(&Finding::EmptyInList),
        "空 `in` 列表使整条条件恒不成立，自查必须报告——实际发现 {findings:?}"
    );
}

/// 组合 2（QA 对抗轮的照片）：引用**调用路径不提供**的事实，必须被报告。
///
/// **红条件**：删掉 [`audit_rule`] 里 `FactNotInContext` 的检查（移除档）——本用例红。
#[test]
fn a_deny_on_a_fact_outside_the_calling_context_is_reported() {
    let on_absent = rule(
        Level::SystemSafety,
        Decision::Deny,
        json!({"fact": "privacy_class", "eq": "secret"}),
    );
    // 集成裁决路径只提供 `explicit_current`。
    let findings = audit_rule(&on_absent, &[]);
    assert_eq!(
        findings,
        vec![Finding::FactNotInContext {
            fact: "privacy_class".to_owned()
        }],
        "引用缺省事实的 Deny 在该路径上不成立，自查必须报出是哪一个事实"
    );
}

/// 对照：**写对的**拦类规则不被报告——自查不是「所有 Deny 都喊」。
///
/// 两枚写对的样子：非空 `in` 且事实在场；`explicit_current`（总可观察）。
/// `RequireApproval` 同理（同属拦类）。
///
/// **红条件**：把自查改成「Deny 一律报告」（放宽档）——本用例红。
#[test]
fn a_blocking_rule_that_can_fire_is_not_reported() {
    let fine = rule(
        Level::SystemSafety,
        Decision::Deny,
        json!({"fact": "effect_type", "in": ["charge"]}),
    );
    assert_eq!(audit_rule(&fine, &["effect_type"]), Vec::new());

    let always_observable = rule(
        Level::SystemSafety,
        Decision::Deny,
        json!({"fact": "explicit_current", "eq": true}),
    );
    assert_eq!(
        audit_rule(&always_observable, &[]),
        Vec::new(),
        "explicit_current 总可观察，不报缺省"
    );

    let approval = rule(
        Level::Project,
        Decision::RequireApproval,
        json!({"fact": "explicit_current", "eq": false}),
    );
    assert_eq!(audit_rule(&approval, &[]), Vec::new());
}

/// 对照的另一半：`Allow` 不在自查范围内——它的条件不成立是「少放行」，
/// 方向是 fail-closed，不是本自查要拦的形状。
///
/// **红条件**：把自查扩到 `Allow`（扩档）——本用例红。
#[test]
fn an_allow_is_out_of_scope_of_this_audit() {
    let allow_with_empty_in = rule(
        Level::UserPersistent,
        Decision::Allow,
        json!({"fact": "effect_type", "in": []}),
    );
    assert_eq!(
        audit_rule(&allow_with_empty_in, &[]),
        Vec::new(),
        "Allow 的永不匹配是 fail-closed 一侧，自查不报"
    );
}

/// 后果锚点：自查报告的两个形状**确实**会在裁决器上静默失效——把 QA 的照片
/// 收进版控，闭合「自查报的东西是真的」。
///
/// 与 QA 对抗轮的 `qa_a_miswritten_first_level_deny_silently_loses` /
/// `qa_a_first_level_deny_on_an_absent_fact_silently_loses` 同题：那两条在
/// `.tmp/` 的临时 harness 里（摸底材料），这一条把结论固定进树。
#[test]
fn the_reported_shapes_really_do_lose_to_a_lower_allow() {
    let with_effect = PolicyContext {
        effect_type: Some(EffectType::Charge),
        ..PolicyContext::default()
    };
    let allow = rule(Level::UserPersistent, Decision::Allow, json!({"all": []}));

    // 形状一：空 `in` 的第 1 级 Deny —— 裁决落到 Allow。
    let empty_in = rule(
        Level::SystemSafety,
        Decision::Deny,
        json!({"fact": "effect_type", "in": []}),
    );
    assert!(audit_rule(&empty_in, &["effect_type"]).contains(&Finding::EmptyInList));
    assert_eq!(
        decide(&[empty_in, allow.clone()], &with_effect),
        Decision::Allow,
        "空 in 恒不成立，更高的 Deny 因此失效"
    );

    // 形状二：引用缺省事实的同形规则 —— 同样落到 Allow。
    let on_absent = rule(
        Level::SystemSafety,
        Decision::Deny,
        json!({"fact": "privacy_class", "eq": "secret"}),
    );
    assert!(audit_rule(&on_absent, &[]).contains(&Finding::FactNotInContext {
        fact: "privacy_class".to_owned()
    }));
    assert_eq!(
        decide(&[on_absent, allow.clone()], &PolicyContext::default()),
        Decision::Allow,
        "事实缺省时该条不成立，更高的 Deny 不生效"
    );

    // 对照：写对的 Deny 把低层 Allow 压回去（含 --approve 出席时）。
    let correct = rule(
        Level::SystemSafety,
        Decision::Deny,
        json!({"fact": "effect_type", "in": ["charge"]}),
    );
    assert_eq!(audit_rule(&correct, &["effect_type"]), Vec::new());
    assert_eq!(decide(&[correct, allow.clone()], &with_effect), Decision::Deny);

    let correct_on_approval = rule(
        Level::SystemSafety,
        Decision::Deny,
        json!({"fact": "explicit_current", "eq": true}),
    );
    let approved = PolicyContext {
        explicit_current: Some(ExplicitApproval),
        ..PolicyContext::default()
    };
    assert_eq!(
        decide(&[correct_on_approval, allow], &approved),
        Decision::Deny
    );
}
