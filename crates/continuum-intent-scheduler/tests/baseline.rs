mod support;

use continuum_intent_scheduler::{
    BaselineIntentPolicy, BackgroundSuitability, Blocking, Deadline, IntentDecision,
    IntentScheduler, PrioritySignal, UserFocus,
};
use continuum_semantics::{IntentId, IntentStatus, Priority};

fn ready(id: &str) -> continuum_semantics::Intent {
    support::intent(id, IntentStatus::Ready)
}

#[test]
fn empty_candidates_yield_no_intent() {
    let inputs = support::inputs(&[]);
    let decision = IntentScheduler::decide(&inputs, &BaselineIntentPolicy).expect("基线应成功");

    assert!(matches!(decision, IntentDecision::NoIntent { .. }));
    assert!(decision.advanced().is_none());
    assert_eq!(decision.basis().as_str(), "no_advanceable_candidate");
}

#[test]
fn user_focus_beats_deadline_and_blocking() {
    let candidates = [ready("a"), ready("b")];
    let deadline = [
        Deadline { intent: IntentId::from_str("a"), at_millis: 10 },
        Deadline { intent: IntentId::from_str("b"), at_millis: 20 },
    ];
    let blocking = [Blocking {
        intent: IntentId::from_str("a"),
        blocks: vec![
            IntentId::from_str("x"),
            IntentId::from_str("y"),
            IntentId::from_str("z"),
        ],
    }];
    let mut inputs = support::inputs(&candidates);
    inputs.deadline = &deadline;
    inputs.blocking = &blocking;
    inputs.user_focus = UserFocus::Intent(IntentId::from_str("b"));

    let decision = IntentScheduler::decide(&inputs, &BaselineIntentPolicy).expect("基线应成功");
    assert_eq!(decision.advanced().map(|id| id.as_str()), Some("b"));
    assert_eq!(decision.basis().as_str(), "user_focus");
}

#[test]
fn the_earliest_deadline_wins_and_multiple_entries_take_the_minimum() {
    let candidates = [ready("a"), ready("b")];
    let deadline = [
        Deadline { intent: IntentId::from_str("a"), at_millis: 50 },
        Deadline { intent: IntentId::from_str("a"), at_millis: 20 },
        Deadline { intent: IntentId::from_str("b"), at_millis: 30 },
    ];
    let mut inputs = support::inputs(&candidates);
    inputs.deadline = &deadline;

    let decision = IntentScheduler::decide(&inputs, &BaselineIntentPolicy).expect("基线应成功");
    assert_eq!(decision.advanced().map(|id| id.as_str()), Some("a"));
    assert_eq!(decision.basis().as_str(), "earliest_deadline");
}

#[test]
fn the_largest_deduped_blocking_set_wins() {
    let candidates = [ready("a"), ready("b")];
    let blocking = [
        Blocking {
            intent: IntentId::from_str("a"),
            blocks: vec![
                IntentId::from_str("x"),
                IntentId::from_str("x"),
                IntentId::from_str("y"),
            ],
        },
        Blocking {
            intent: IntentId::from_str("a"),
            blocks: vec![IntentId::from_str("y"), IntentId::from_str("z")],
        },
        Blocking {
            intent: IntentId::from_str("b"),
            blocks: vec![IntentId::from_str("x"), IntentId::from_str("y")],
        },
    ];
    let mut inputs = support::inputs(&candidates);
    inputs.blocking = &blocking;

    let decision = IntentScheduler::decide(&inputs, &BaselineIntentPolicy).expect("基线应成功");
    assert_eq!(decision.advanced().map(|id| id.as_str()), Some("a"));
    assert_eq!(decision.basis().as_str(), "blocking_degree");
}

#[test]
fn candidate_order_is_the_last_tiebreak() {
    let candidates = [
        support::intent("proposed", IntentStatus::Proposed),
        ready("ready"),
    ];
    let inputs = support::inputs(&candidates);

    let decision = IntentScheduler::decide(&inputs, &BaselineIntentPolicy).expect("基线应成功");
    assert_eq!(decision.advanced().map(|id| id.as_str()), Some("ready"));
    assert_eq!(decision.basis().as_str(), "candidate_order");
}

#[test]
fn priority_and_background_suitability_do_not_change_the_decision() {
    let candidates = [ready("a"), ready("b")];
    let priority = [
        PrioritySignal {
            intent: IntentId::from_str("a"),
            priority: Priority::from_str("z"),
        },
        PrioritySignal {
            intent: IntentId::from_str("b"),
            priority: Priority::from_str("a"),
        },
    ];
    let mut inputs = support::inputs(&candidates);
    inputs.priority = &priority;

    let first = IntentScheduler::decide(&inputs, &BaselineIntentPolicy).expect("基线应成功");
    assert_eq!(first.advanced().map(|id| id.as_str()), Some("a"));

    // 互换两条 PrioritySignal，并换一个 BackgroundSuitability 值：仍选 A。
    let swapped = [
        PrioritySignal {
            intent: IntentId::from_str("a"),
            priority: Priority::from_str("a"),
        },
        PrioritySignal {
            intent: IntentId::from_str("b"),
            priority: Priority::from_str("z"),
        },
    ];
    inputs.priority = &swapped;
    inputs.background_suitability = BackgroundSuitability::from_projection("other");

    let second = IntentScheduler::decide(&inputs, &BaselineIntentPolicy).expect("基线应成功");
    assert_eq!(second.advanced().map(|id| id.as_str()), Some("a"));
}

#[test]
fn signals_pointing_outside_the_candidates_are_ignored() {
    let candidates = [ready("a")];
    let deadline = [Deadline { intent: IntentId::from_str("ghost"), at_millis: 1 }];
    let blocking = [Blocking {
        intent: IntentId::from_str("ghost"),
        blocks: vec![IntentId::from_str("x")],
    }];
    let mut inputs = support::inputs(&candidates);
    inputs.deadline = &deadline;
    inputs.blocking = &blocking;

    let decision = IntentScheduler::decide(&inputs, &BaselineIntentPolicy).expect("基线应成功");
    assert_eq!(decision.advanced().map(|id| id.as_str()), Some("a"));
    assert_eq!(decision.basis().as_str(), "candidate_order");
}

#[test]
fn duplicate_candidate_ids_are_not_deduplicated() {
    let candidates = [ready("dup"), ready("dup"), ready("other")];
    let inputs = support::inputs(&candidates);

    let decision = IntentScheduler::decide(&inputs, &BaselineIntentPolicy).expect("基线应成功");
    // 不断言选中哪一枚的元数据；两枚 id 相同，选中的 id 相同。
    assert_eq!(decision.advanced().map(|id| id.as_str()), Some("dup"));
}

#[test]
fn the_same_inputs_yield_the_same_decision() {
    let candidates = [ready("a"), ready("b")];
    let deadline = [Deadline { intent: IntentId::from_str("b"), at_millis: 5 }];
    let mut inputs = support::inputs(&candidates);
    inputs.deadline = &deadline;

    let first = IntentScheduler::decide(&inputs, &BaselineIntentPolicy).expect("基线应成功");
    let second = IntentScheduler::decide(&inputs, &BaselineIntentPolicy).expect("基线应成功");

    assert_eq!(
        first.advanced().map(|id| id.as_str()),
        second.advanced().map(|id| id.as_str())
    );
    assert_eq!(first.basis().as_str(), second.basis().as_str());
}
