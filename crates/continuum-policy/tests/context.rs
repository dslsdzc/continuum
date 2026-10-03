//! `PolicyContext` 的构造与缺省（设计下篇第 5.6 节）。

use continuum_artifact::PrivacyClass;
use continuum_effect::EffectType;
use continuum_policy::{ExplicitApproval, PolicyContext};

/// 缺省上下文什么都不说：每个事实都缺省。
///
/// 逐字段断言而非只断言 `== Default::default()`：后者在默认实现把某个字段
/// 改成 `Some(..)` 时也照过（两边一起变），而「缺省即无事实」正是条件求值依赖的
/// 前提（设计下篇第 5.3 节）。
#[test]
fn a_default_context_carries_no_facts() {
    let ctx = PolicyContext::default();

    assert!(ctx.explicit_current.is_none(), "缺省不应有显式确认");
    assert!(ctx.privacy_class.is_none(), "缺省不应有隐私等级");
    assert!(ctx.effect_type.is_none(), "缺省不应有效应类型");
    assert!(ctx.task_class.is_none(), "缺省不应有任务类别");
    assert!(ctx.duration_ms.is_none(), "缺省不应有已持续时间");
}

/// 每个事实都装得下，且装什么读回什么。
#[test]
fn a_context_can_carry_every_fact() {
    let ctx = PolicyContext {
        explicit_current: Some(ExplicitApproval),
        privacy_class: Some(PrivacyClass::LocalOnly),
        effect_type: Some(EffectType::DeleteRemote),
        task_class: Some("build".into()),
        duration_ms: Some(1500),
    };

    assert_eq!(ctx.explicit_current, Some(ExplicitApproval));
    assert_eq!(ctx.privacy_class, Some(PrivacyClass::LocalOnly));
    assert_eq!(ctx.effect_type, Some(EffectType::DeleteRemote));
    assert_eq!(ctx.task_class.as_deref(), Some("build"));
    assert_eq!(ctx.duration_ms, Some(1500));

    // 字段可写，故结构体更新语法可用：求值时只填相关的那个事实。
    let only_effect = PolicyContext {
        effect_type: Some(EffectType::Charge),
        ..PolicyContext::default()
    };
    assert_eq!(only_effect.effect_type, Some(EffectType::Charge));
    assert_eq!(only_effect.privacy_class, None, "未指定的字段应保持缺省");
}

/// `ExplicitApproval` 参与上下文的相等性：给出显式确认与不给出是两个上下文。
#[test]
fn an_explicit_approval_participates_in_equality() {
    let given = PolicyContext {
        explicit_current: Some(ExplicitApproval),
        ..PolicyContext::default()
    };

    assert_ne!(
        given,
        PolicyContext::default(),
        "给出 --approve 与不给不是同一个上下文"
    );
    assert_eq!(given, given.clone(), "上下文可比较");
}
