mod support;

use std::cell::{Cell, RefCell};

use continuum_graph::NodeId;
use continuum_intent_scheduler::{
    BackgroundSuitability, Deadline, DirectiveForError, ExecutionRequest, IntentScheduler,
    PrioritySignal, ResourceDirective, ResourceDirectiveError, ResourceDirectivePort,
};
use continuum_semantics::{IntentId, IntentStatus, Priority};

const RESOURCE_SRC: &str = include_str!("../src/resource.rs");

struct FakePort {
    directive: ResourceDirective,
    error: Option<ResourceDirectiveError>,
    calls: Cell<usize>,
}

impl FakePort {
    fn returning(directive: ResourceDirective) -> Self {
        Self { directive, error: None, calls: Cell::new(0) }
    }

    fn failing(message: &str) -> Self {
        Self {
            directive: ResourceDirective::new(None, None, None, None),
            error: Some(ResourceDirectiveError::new(message)),
            calls: Cell::new(0),
        }
    }

    fn calls(&self) -> usize {
        self.calls.get()
    }
}

impl ResourceDirectivePort for FakePort {
    fn direct(
        &self,
        _request: &ExecutionRequest<'_>,
    ) -> Result<ResourceDirective, ResourceDirectiveError> {
        self.calls.set(self.calls.get() + 1);
        match &self.error {
            Some(error) => Err(error.clone()),
            None => Ok(self.directive.clone()),
        }
    }
}

struct Seen {
    intent: String,
    node: Option<String>,
    priority: Option<String>,
    deadline: Option<i64>,
    background: String,
}

struct RecordingPort {
    seen: RefCell<Option<Seen>>,
}

impl RecordingPort {
    fn new() -> Self {
        Self { seen: RefCell::new(None) }
    }
}

impl ResourceDirectivePort for RecordingPort {
    fn direct(
        &self,
        request: &ExecutionRequest<'_>,
    ) -> Result<ResourceDirective, ResourceDirectiveError> {
        *self.seen.borrow_mut() = Some(Seen {
            intent: request.intent.id.as_str().to_owned(),
            node: request.node.map(|node| node.as_str().to_owned()),
            priority: request.signals.priority.map(|priority| priority.as_str().to_owned()),
            deadline: request.signals.deadline.map(|deadline| deadline.at_millis),
            background: request.signals.background_suitability.projection().to_owned(),
        });
        Ok(ResourceDirective::new(None, None, None, None))
    }
}

#[test]
fn no_intent_returns_none_without_calling_the_port() {
    let candidates = [support::intent("ready", IntentStatus::Ready)];
    let inputs = support::inputs(&candidates);
    let decision = support::no_intent("none");
    let port = FakePort::returning(ResourceDirective::new(None, None, None, None));

    let directive = IntentScheduler::directive_for(&inputs, &decision, None, &port)
        .expect("NoIntent 不应失败");
    assert!(directive.is_none());
    assert_eq!(port.calls(), 0);
}

#[test]
fn a_legal_advance_calls_the_port_once_and_returns_four_slots_verbatim() {
    let candidates = [support::intent("ready", IntentStatus::Ready)];
    let inputs = support::inputs(&candidates);
    let decision = support::advance("ready", "chosen");
    let directive = ResourceDirective::new(
        Some("node-a".to_owned()),
        Some("model-x".to_owned()),
        Some("tool-y".to_owned()),
        Some("8".to_owned()),
    );
    let port = FakePort::returning(directive.clone());

    let got = IntentScheduler::directive_for(&inputs, &decision, None, &port)
        .expect("合法 Advance 应成功")
        .expect("应返回指令");
    assert_eq!(got, directive);
    assert_eq!(got.node_placement(), Some("node-a"));
    assert_eq!(got.model(), Some("model-x"));
    assert_eq!(got.tool(), Some("tool-y"));
    assert_eq!(got.parallelism(), Some("8"));
    assert_eq!(port.calls(), 1);

    // 第二个指令值：未答的槽保持 None，不被默认值填满。
    let partial = ResourceDirective::new(Some("n".to_owned()), None, None, Some("2".to_owned()));
    let partial_port = FakePort::returning(partial);
    let got = IntentScheduler::directive_for(&inputs, &decision, None, &partial_port)
        .expect("合法 Advance 应成功")
        .expect("应返回指令");
    assert_eq!(got.model(), None);
    assert_eq!(got.tool(), None);
}

#[test]
fn a_port_error_is_passed_through() {
    let candidates = [support::intent("ready", IntentStatus::Ready)];
    let inputs = support::inputs(&candidates);
    let decision = support::advance("ready", "chosen");
    let port = FakePort::failing("rank unavailable");

    match IntentScheduler::directive_for(&inputs, &decision, None, &port) {
        Err(DirectiveForError::Port(error)) => {
            assert_eq!(error.message(), "rank unavailable")
        }
        other => panic!("期望 Port，实际 {other:?}"),
    }
    assert_eq!(port.calls(), 1);
}

#[test]
fn an_invalid_advance_is_rejected_without_calling_the_port() {
    let candidates = [support::intent("ready", IntentStatus::Ready)];
    let inputs = support::inputs(&candidates);
    let port = FakePort::returning(ResourceDirective::new(None, None, None, None));
    let ghost = support::advance("ghost", "chosen");

    match IntentScheduler::directive_for(&inputs, &ghost, None, &port) {
        Err(DirectiveForError::InvalidDecision { intent }) => {
            assert_eq!(intent.as_str(), "ghost")
        }
        other => panic!("期望 InvalidDecision，实际 {other:?}"),
    }
    assert_eq!(port.calls(), 0);

    let completed = [support::intent("done", IntentStatus::Completed)];
    let completed_inputs = support::inputs(&completed);
    let done = support::advance("done", "chosen");
    match IntentScheduler::directive_for(&completed_inputs, &done, None, &port) {
        Err(DirectiveForError::InvalidDecision { intent }) => {
            assert_eq!(intent.as_str(), "done")
        }
        other => panic!("期望 InvalidDecision，实际 {other:?}"),
    }
    assert_eq!(port.calls(), 0);
}

#[test]
fn the_port_receives_first_priority_earliest_deadline_and_round_background() {
    let candidates = [support::intent("ready", IntentStatus::Ready)];
    let decision = support::advance("ready", "chosen");
    let priority = [
        PrioritySignal {
            intent: IntentId::from_str("ready"),
            priority: Priority::from_str("first"),
        },
        PrioritySignal {
            intent: IntentId::from_str("ready"),
            priority: Priority::from_str("second"),
        },
    ];
    let deadline = [
        Deadline { intent: IntentId::from_str("ready"), at_millis: 500 },
        Deadline { intent: IntentId::from_str("ready"), at_millis: 100 },
    ];
    let mut inputs = support::inputs(&candidates);
    inputs.priority = &priority;
    inputs.deadline = &deadline;
    inputs.background_suitability = BackgroundSuitability::from_projection("bg-value");
    let node = NodeId::new("n1");
    let port = RecordingPort::new();

    let directive = IntentScheduler::directive_for(&inputs, &decision, Some(&node), &port)
        .expect("合法 Advance 应成功");
    assert!(directive.is_some());

    let seen = port.seen.borrow();
    let seen = seen.as_ref().expect("端口应被调用");
    assert_eq!(seen.intent, "ready");
    assert_eq!(seen.node.as_deref(), Some("n1"));
    assert_eq!(seen.priority.as_deref(), Some("first"));
    assert_eq!(seen.deadline, Some(100));
    assert_eq!(seen.background, "bg-value");
}

#[test]
fn resource_directive_fields_are_private_and_default_is_absent() {
    let start = RESOURCE_SRC
        .find("pub struct ResourceDirective {")
        .expect("ResourceDirective 定义应存在");
    let open = start + RESOURCE_SRC[start..].find('{').expect("结构体应有花括号");
    let close = open + RESOURCE_SRC[open..].find('}').expect("结构体应闭合");
    let fields = &RESOURCE_SRC[open + 1..close];
    for line in fields.lines() {
        assert!(
            !line.trim_start().starts_with("pub "),
            "ResourceDirective 的字段必须是私有：{line}"
        );
    }
    assert!(
        !RESOURCE_SRC.contains("impl Default for ResourceDirective"),
        "ResourceDirective 不得实现 Default"
    );
}

#[test]
fn there_is_no_production_port_implementation_or_router_placement_surface() {
    let sources = [
        ("lib.rs", include_str!("../src/lib.rs")),
        ("decision.rs", include_str!("../src/decision.rs")),
        ("inputs.rs", include_str!("../src/inputs.rs")),
        ("graph.rs", include_str!("../src/graph.rs")),
        ("resource.rs", RESOURCE_SRC),
    ];
    let mut all = String::new();
    for (_, source) in sources {
        all.push_str(&support::code_without_comments(source));
        all.push('\n');
    }
    assert!(!all.contains("impl ResourceDirectivePort"));
    for forbidden in [
        "rank(",
        "place(",
        "RoutingRequest",
        "PlacementRequest",
        "ComputeNode",
        "ExecutionCandidate",
        "continuum_model_registry",
        "continuum_node",
    ] {
        assert!(!all.contains(forbidden), "src/ 不得出现 {forbidden}");
    }
}

#[test]
fn directive_for_only_transfers_and_makes_no_new_decisions() {
    let code = support::code_without_comments(RESOURCE_SRC);
    let body = support::function_body(&code, "pub fn directive_for");
    assert!(body.contains("Ok(None)"));
    assert!(body.contains("port.direct"));
    assert!(body.contains("is_advanceable"));
    for forbidden in [
        "continuum_graph",
        "select_runnable",
        "ResourceDirective::new",
        "apply_intent_transition",
        "transition(",
    ] {
        assert!(!body.contains(forbidden), "directive_for 不得包含 {forbidden}");
    }
}
