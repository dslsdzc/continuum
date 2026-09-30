use continuum_events::{
    decode_event, DecodedEvent, Event, EventCodecChain, EventLogError, EventType, SkipReason,
    CURRENT_SCHEMA_VERSION,
};
use serde_json::json;

#[test]
fn nine_event_types_match_spec_310() {
    let names: Vec<&str> = EventType::ALL.iter().map(|t| t.as_str()).collect();
    assert_eq!(
        names,
        vec![
            "intent.created",
            "plan.review_required",
            "node.started",
            "node.completed",
            "artifact.created",
            "verification.failed",
            "decision.required",
            "effect.committed",
            "intent.completed",
        ]
    );
}

#[test]
fn all_event_types_round_trip() {
    for t in EventType::ALL {
        let ev = Event::new(t, "ev-1", 1_700_000_000_000, json!({"k": "v"}));
        let text = serde_json::to_string(&ev).expect("可序列化");
        let back: Event = serde_json::from_str(&text).expect("可反序列化");
        assert_eq!(back, ev, "事件 {t:?} 往返不一致");
        assert_eq!(back.schema_version, CURRENT_SCHEMA_VERSION);
    }
}

#[test]
fn missing_optional_fields_deserialize_as_none() {
    let text = r#"{
        "event_id": "ev-1",
        "event_type": "node.started",
        "schema_version": 1,
        "occurred_at": 1700000000000,
        "payload": {}
    }"#;
    let ev: Event = serde_json::from_str(text).expect("缺省可选字段应可反序列化");
    assert_eq!(ev.intent_id, None);
    assert_eq!(ev.node_id, None);
    assert_eq!(ev.event_type, EventType::NodeStarted);
}

#[test]
fn unknown_optional_field_does_not_break_deserialization() {
    let text = r#"{
        "event_id": "ev-1",
        "event_type": "node.started",
        "schema_version": 1,
        "occurred_at": 1700000000000,
        "intent_id": "i-1",
        "node_id": "n-1",
        "payload": {},
        "future_field": 42
    }"#;
    let ev: Event = serde_json::from_str(text).expect("新增可选字段不得破坏既有反序列化");
    assert_eq!(ev.intent_id.as_deref(), Some("i-1"));
}

#[test]
fn nine_types_are_never_ignorable() {
    for t in EventType::ALL {
        let ev = Event::new(t, "ev-1", 1, json!({}));
        assert!(!ev.ignorable, "既有事件类型 {t:?} 的 ignorable 必须为 false");
    }
}

#[test]
fn unknown_non_ignorable_type_is_fatal() {
    let text = r#"{"event_id":"e","event_type":"future.thing","schema_version":1,
                   "occurred_at":1,"ignorable":false,"payload":{}}"#;
    match decode_event(text) {
        Err(EventLogError::UnknownEventType { event_type }) => {
            assert_eq!(event_type, "future.thing")
        }
        other => panic!("未知且非 ignorable 的类型必须致命，实际 {other:?}"),
    }
}

#[test]
fn unknown_ignorable_type_is_skippable() {
    let text = r#"{"event_id":"e","event_type":"future.thing","schema_version":1,
                   "occurred_at":1,"ignorable":true,"payload":{}}"#;
    match decode_event(text) {
        Ok(DecodedEvent::Skippable(SkipReason::UnknownIgnorableType { event_type })) => {
            assert_eq!(event_type, "future.thing")
        }
        other => panic!("未知且 ignorable 的类型必须可跳过，实际 {other:?}"),
    }
}

#[test]
fn malformed_payload_is_skippable() {
    // schema_version 给了字符串而非整数，解码失败但不是未知类型
    let broken = r#"{"event_id":"e","event_type":"node.started","schema_version":"one",
                     "occurred_at":1,"ignorable":false,"payload":{}}"#;
    match decode_event(broken) {
        Ok(DecodedEvent::Skippable(SkipReason::MalformedPayload { .. })) => {}
        other => panic!("畸形 payload 必须可跳过，实际 {other:?}"),
    }
}

#[test]
fn codec_chain_rejects_gaps() {
    let mut chain = EventCodecChain::new();
    chain.register(1, |_| Ok(DecodedEvent::Skippable(SkipReason::MalformedPayload {
        message: "占位解码器".into(),
    })));
    chain.register(3, |_| Ok(DecodedEvent::Skippable(SkipReason::MalformedPayload {
        message: "占位解码器".into(),
    })));
    match chain.validate_contiguous() {
        Err(EventLogError::CodecChainGap { missing }) => assert_eq!(missing, 2),
        other => panic!("版本号缺口必须被拒绝，实际 {other:?}"),
    }
}

#[test]
fn codec_chain_accepts_contiguous_versions() {
    let mut chain = EventCodecChain::new();
    chain.register(1, |_| Ok(DecodedEvent::Skippable(SkipReason::MalformedPayload {
        message: "占位解码器".into(),
    })));
    chain.register(2, |_| Ok(DecodedEvent::Skippable(SkipReason::MalformedPayload {
        message: "占位解码器".into(),
    })));
    chain.validate_contiguous().expect("相邻版本应通过校验");
}
