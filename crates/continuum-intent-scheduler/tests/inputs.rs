use continuum_intent_scheduler::{
    BackgroundSuitability, Blocking, Deadline, PrioritySignal, SchedulerInputs, UserFocus,
};
use continuum_semantics::{IntentId, Priority};

fn empty_inputs() -> SchedulerInputs<'static> {
    SchedulerInputs {
        candidates: &[],
        priority: &[],
        deadline: &[],
        blocking: &[],
        user_focus: UserFocus::None,
        background_suitability: BackgroundSuitability::from_projection("background"),
    }
}

#[test]
fn scheduler_inputs_carries_candidates_and_the_five_302_inputs() {
    let inputs = empty_inputs();
    // 不写 `..`：字段名/数量与 §302 的五个名字逐字绑定；改名、加字段、删字段都编译失败。
    let SchedulerInputs {
        candidates,
        priority,
        deadline,
        blocking,
        user_focus,
        background_suitability,
    } = &inputs;
    assert!(candidates.is_empty());
    assert!(priority.is_empty());
    // 显式绑定字段类型，同时避免未使用导入（全量门要求 0 warning）。
    let _: &[Deadline] = deadline;
    let _: &[Blocking] = blocking;
    assert!(deadline.is_empty());
    assert!(blocking.is_empty());
    assert!(matches!(user_focus, UserFocus::None));
    assert_eq!(background_suitability.projection(), "background");
}

#[test]
fn background_suitability_round_trips_projection_verbatim() {
    for value in ["", " ", " 后台 适合 ", "适合\u{1F642}"] {
        let carrier = BackgroundSuitability::from_projection(value);
        assert_eq!(carrier.projection(), value, "投影值不得改写、裁剪或归一");
    }
}

#[test]
fn priority_signal_carries_the_opaque_priority_verbatim() {
    let signal = PrioritySignal {
        intent: IntentId::from_str("i1"),
        priority: Priority::from_str("P0"),
    };
    assert_eq!(signal.priority.as_str(), "P0");
    assert_eq!(signal.intent.as_str(), "i1");
}
