//! 受限条件的解析与求值（设计下篇第 5.1、5.3 节）。
//!
//! 本文件的重点是「写错即拒绝」的六类用例（未知事实名、未知比较符、取值类型不符、
//! 取值不在封闭集合内、比较符不适用于该事实、结构不符）：设计下篇第 5.1 节要求
//! 写错的条件拒绝该规则，而不是「不匹配」——静默退化为「永不匹配」会让人以为
//! 策略生效了而实际没有。故每一类都断言到**具体的错误变体**，不只断言「返回了 Err」。

use continuum_artifact::PrivacyClass;
use continuum_effect::EffectType;
use continuum_policy::{Condition, ExplicitApproval, PolicyContext, PolicyError};
use serde_json::{Value, json};

fn ctx_with_effect(effect: Option<EffectType>) -> PolicyContext {
    PolicyContext {
        effect_type: effect,
        ..PolicyContext::default()
    }
}

/// `{"all": []}` 是空合取，恒真——级别为「对所有情形都适用」的规则用得上它。
#[test]
fn an_empty_conjunction_is_always_true() {
    let condition = Condition::parse(&json!({"all": []})).expect("空合取应可解析");

    assert!(
        condition.matches(&PolicyContext::default()),
        "空合取对什么都不说的上下文也应成立"
    );
    assert!(
        condition.matches(&PolicyContext {
            explicit_current: Some(ExplicitApproval),
            privacy_class: Some(PrivacyClass::Secret),
            effect_type: Some(EffectType::DeleteRemote),
            task_class: Some("build".into()),
            duration_ms: Some(1),
        }),
        "空合取对装满事实的上下文也应成立"
    );
}

/// 一条等式只在该事实等于指定值时为真。同一个条件的两个方向都要断言：
/// 只断言「匹配」的用例在恒真实现下也照过。
#[test]
fn a_single_equality_matches_only_the_named_value() {
    let condition = Condition::parse(&json!({"fact": "effect_type", "eq": "charge"}))
        .expect("已知事实、已知比较符、类型相符，应可解析");

    assert!(
        condition.matches(&ctx_with_effect(Some(EffectType::Charge))),
        "effect_type = Charge 应匹配"
    );
    assert!(
        !condition.matches(&ctx_with_effect(Some(EffectType::PushBranch))),
        "effect_type = PushBranch 不应匹配"
    );
}

/// 所引用的事实不在上下文中时，该条件不成立（设计下篇第 5.3 节）。
///
/// **不是**「匹配」：若缺省算成立，一条 `Allow` 会在事实未知时生效；
/// 若整条规则因此不匹配，则由「无匹配即 `Deny`」兜住，取的是严的一侧。
#[test]
fn an_absent_fact_does_not_match() {
    let condition = Condition::parse(&json!({"fact": "effect_type", "eq": "charge"}))
        .expect("同上一条");

    assert!(
        !condition.matches(&ctx_with_effect(None)),
        "事实缺省时该条件项不成立"
    );
}

/// 未知事实名在**解析期**即被拒，而不是留到求值时不匹配。
#[test]
fn an_unknown_fact_is_rejected_at_parse_time() {
    let error = Condition::parse(&json!({"fact": "nope", "eq": "x"}))
        .expect_err("未知事实名应被拒绝");

    assert_eq!(
        error,
        PolicyError::UnknownFact { name: "nope".into() },
        "应指名是哪个事实名未知"
    );
}

/// 未知比较符在**解析期**即被拒。`gt` 是 `gte` 的常见错拼，也是设计下篇
/// 第 5.1 节那个封闭集合之外的东西。
#[test]
fn an_unknown_operator_is_rejected_at_parse_time() {
    let error = Condition::parse(&json!({"fact": "effect_type", "gt": "x"}))
        .expect_err("未知比较符应被拒绝");

    assert_eq!(
        error,
        PolicyError::UnknownOperator { name: "gt".into() },
        "应指名是哪个比较符未知"
    );
}

/// 取值的 JSON 类型与事实不符，在**解析期**即被拒。
#[test]
fn a_value_of_the_wrong_type_is_rejected_at_parse_time() {
    let error = Condition::parse(&json!({"fact": "effect_type", "eq": 3}))
        .expect_err("effect_type 的取值应为字符串，数值应被拒绝");

    assert!(
        matches!(
            error,
            PolicyError::ValueTypeMismatch {
                fact: "effect_type",
                ..
            }
        ),
        "应报取值类型不符且指名事实，实得 {error:?}"
    );
}

/// 取值是字符串，但不是该事实的封闭取值之一——同样在解析期被拒。
///
/// 这类错最像「手滑」：`{"eq": "charg"}` 或 `{"eq": "Charge"}` 若静默不匹配，
/// 一条本该拦住的规则就永远不会生效。
///
/// 两个枚举事实各占一行：它们走的是同一条代码路径（枚举自己的 `parse`），
/// 只钉一个会让另一条路径的退化无人过问。
#[test]
fn an_unknown_enum_value_is_rejected_at_parse_time() {
    for (condition, fact, value) in [
        (
            json!({"fact": "effect_type", "eq": "charg"}),
            "effect_type",
            "charg",
        ),
        (
            json!({"fact": "privacy_class", "eq": "Personal"}),
            "privacy_class",
            "Personal",
        ),
    ] {
        let error = Condition::parse(&condition).expect_err("取值不在封闭集合内应被拒绝");

        assert_eq!(
            error,
            PolicyError::UnknownValue {
                fact,
                value: value.into(),
            },
            "应指名事实与那个不在集合内的取值"
        );
    }
}

/// `gte` 的取值类型同样受检查：`Matcher::Gte` 的文档断言其值必是数值，
/// 这条断言的两半——事实一侧与取值一侧——各由本用例与下面那条守住。
#[test]
fn a_non_numeric_gte_value_is_rejected_at_parse_time() {
    for condition in [
        json!({"fact": "duration_ms", "gte": "1000"}),
        json!({"fact": "duration_ms", "gte": -1}),
        json!({"fact": "duration_ms", "gte": 1.5}),
    ] {
        let parsed = Condition::parse(&condition);
        assert!(
            matches!(
                parsed,
                Err(PolicyError::ValueTypeMismatch {
                    fact: "duration_ms",
                    ..
                })
            ),
            "{condition} 应报取值类型不符，实得 {parsed:?}"
        );
    }
}

/// 比较符不适用于该事实的类型：给字符串或布尔事实排序没有定义。
#[test]
fn an_inapplicable_operator_is_rejected_at_parse_time() {
    for condition in [
        json!({"fact": "privacy_class", "gte": "public"}),
        json!({"fact": "task_class", "gte": "build"}),
        json!({"fact": "explicit_current", "gte": true}),
    ] {
        let parsed = Condition::parse(&condition);
        assert!(
            matches!(parsed, Err(PolicyError::OperatorNotApplicable { .. })),
            "{condition} 应报「比较符不适用」，实得 {parsed:?}"
        );
    }
}

/// 结构不符一律拒绝：顶层不是对象、`all` 不是数组、`all` 与谓词混写、
/// 谓词缺键或多键。五条「结构合法」之外的形状都在这里。
#[test]
fn a_malformed_condition_is_rejected_at_parse_time() {
    for condition in [
        json!("charge"),
        json!([]),
        json!(null),
        json!({}),
        json!({"all": {}}),
        json!({"all": null}),
        json!({"all": [], "fact": "effect_type", "eq": "charge"}),
        json!({"fact": "effect_type"}),
        json!({"fact": "effect_type", "eq": "charge", "in": ["publish"]}),
        json!({"fact": 3, "eq": "charge"}),
        json!({"fact": "effect_type", "eq": "charge", "note": "x"}),
        json!({"all": [{"fact": "effect_type", "eq": "charge"}, 3]}),
    ] {
        let parsed = Condition::parse(&condition);
        assert!(
            matches!(parsed, Err(PolicyError::Malformed { .. })),
            "{condition} 应报结构不符，实得 {parsed:?}"
        );
    }
}

/// 合取要求每一项都成立。表驱动：逐项翻转，只有全真时结果才为真。
#[test]
fn a_conjunction_requires_every_term() {
    let condition = Condition::parse(&json!({"all": [
        {"fact": "effect_type", "eq": "charge"},
        {"fact": "task_class", "eq": "build"},
    ]}))
    .expect("两项合取应可解析");

    let effect = |effect: EffectType| PolicyContext {
        effect_type: Some(effect),
        ..PolicyContext::default()
    };
    let both = PolicyContext {
        effect_type: Some(EffectType::Charge),
        task_class: Some("build".into()),
        ..PolicyContext::default()
    };

    assert!(condition.matches(&both), "两项都成立时应成立");
    assert!(
        !condition.matches(&PolicyContext {
            task_class: Some("build".into()),
            ..PolicyContext::default()
        }),
        "缺 effect_type 时不应成立"
    );
    assert!(
        !condition.matches(&PolicyContext {
            effect_type: Some(EffectType::Charge),
            ..PolicyContext::default()
        }),
        "缺 task_class 时不应成立"
    );
    assert!(
        !condition.matches(&effect(EffectType::PushBranch)),
        "effect_type 不符时不应成立"
    );
}

/// `in` 命中列表中的任一项即为真。
#[test]
fn an_in_list_matches_any_listed_value() {
    let condition = Condition::parse(&json!({"fact": "effect_type", "in": ["charge", "deploy"]}))
        .expect("in 应可解析");

    assert!(condition.matches(&ctx_with_effect(Some(EffectType::Charge))));
    assert!(condition.matches(&ctx_with_effect(Some(EffectType::Deploy))));
    assert!(!condition.matches(&ctx_with_effect(Some(EffectType::Publish))));
    assert!(!condition.matches(&ctx_with_effect(None)), "事实缺省即不成立");
}

/// `{"in": []}` 结构合法（解析期接受），但**恒不成立**。
///
/// 单独一条用例，因为它是一个刻意的取法而不是顺带的行为：设计下篇第 5.1 节要防的
/// 正是「永不匹配」的静默退化，而空 `in` 就是这种形态。之所以仍接受它，是因为该节
/// 列的「不符」只有三类（事实名不命中、比较符不在封闭集合、取值类型不符），空列表
/// 不占任何一类，多立一类拒绝规则就是发明规范。**既然接受了，就必须让它可见**：
/// 这条用例就是它的对照片，将来若改成解析期拒绝，本用例会先红。
#[test]
fn an_empty_in_list_never_matches() {
    let empty = Condition::parse(&json!({"fact": "effect_type", "in": []}))
        .expect("空 in 结构合法，解析期接受");

    for ctx in [
        ctx_with_effect(Some(EffectType::Charge)),
        ctx_with_effect(Some(EffectType::PushBranch)),
        ctx_with_effect(None),
    ] {
        assert!(
            !empty.matches(&ctx),
            "空列表没有可命中的项，对任何上下文都不成立"
        );
    }
}

/// `gte` 比较数值；事实缺省即不成立。
#[test]
fn a_gte_compares_numbers() {
    let condition =
        Condition::parse(&json!({"fact": "duration_ms", "gte": 1000})).expect("gte 应可解析");
    let elapsed = |ms: Option<u64>| PolicyContext {
        duration_ms: ms,
        ..PolicyContext::default()
    };

    assert!(condition.matches(&elapsed(Some(1000))), "等于下界应成立");
    assert!(condition.matches(&elapsed(Some(1001))), "大于下界应成立");
    assert!(!condition.matches(&elapsed(Some(999))), "小于下界不应成立");
    assert!(!condition.matches(&elapsed(None)), "事实缺省即不成立");
}

/// `explicit_current` 是布尔事实，且**恒可观察**：上下文总知道 `--approve`
/// 给没给，故「没给」是假，不是「事实不在上下文中」。
///
/// 这条差别有后果：若把 `None` 当作缺省事实，`{"eq": false}` 将永不成立，
/// 一条 `Deny` 规则静默失效就是 fail-open。两个方向都断言。
#[test]
fn explicit_current_is_an_always_observable_boolean_fact() {
    let approved =
        Condition::parse(&json!({"fact": "explicit_current", "eq": true})).expect("应可解析");
    let not_approved =
        Condition::parse(&json!({"fact": "explicit_current", "eq": false})).expect("应可解析");

    let given = PolicyContext {
        explicit_current: Some(ExplicitApproval),
        ..PolicyContext::default()
    };
    let not_given = PolicyContext::default();

    assert!(approved.matches(&given), "给了 --approve 时 eq true 应成立");
    assert!(
        !approved.matches(&not_given),
        "没给 --approve 时 eq true 不应成立"
    );
    assert!(
        !not_approved.matches(&given),
        "给了 --approve 时 eq false 不应成立"
    );
    assert!(
        not_approved.matches(&not_given),
        "没给 --approve 时 eq false 应成立——这正是缺省事实语义会丢掉的那一半"
    );
}

/// 顶层的两种写法同义：裸谓词与 `{"all": [该谓词]}`。
///
/// 两者都必须被接受，且行为一致；`Value` 用一个共同的构造函数取，避免两处
/// 手抄的 JSON 走样。
#[test]
fn a_bare_predicate_equals_a_one_term_conjunction() {
    let bare: Value = json!({"fact": "privacy_class", "eq": "personal"});
    let wrapped: Value = json!({"all": [{"fact": "privacy_class", "eq": "personal"}]});

    let bare = Condition::parse(&bare).expect("裸谓词应可解析");
    let wrapped = Condition::parse(&wrapped).expect("一元合取应可解析");

    let personal = PolicyContext {
        privacy_class: Some(PrivacyClass::Personal),
        ..PolicyContext::default()
    };
    let secret = PolicyContext {
        privacy_class: Some(PrivacyClass::Secret),
        ..PolicyContext::default()
    };

    assert_eq!(bare, wrapped, "两种写法的条件应当相等");
    for ctx in [&personal, &secret, &PolicyContext::default()] {
        assert_eq!(bare.matches(ctx), wrapped.matches(ctx), "两种写法行为应一致");
    }
}
