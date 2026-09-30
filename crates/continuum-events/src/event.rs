//! §310 Event Stream 的事件信封。
//!
//! 信封字段与版本策略取 P0 设计第 5 节的 v0.1 默认，
//! A12 / OPEN-006 的完整答案仍开放。

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// 九个事件类型，名称逐字符取 §310。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventType {
    #[serde(rename = "intent.created")]
    IntentCreated,
    #[serde(rename = "plan.review_required")]
    PlanReviewRequired,
    #[serde(rename = "node.started")]
    NodeStarted,
    #[serde(rename = "node.completed")]
    NodeCompleted,
    #[serde(rename = "artifact.created")]
    ArtifactCreated,
    #[serde(rename = "verification.failed")]
    VerificationFailed,
    #[serde(rename = "decision.required")]
    DecisionRequired,
    #[serde(rename = "effect.committed")]
    EffectCommitted,
    #[serde(rename = "intent.completed")]
    IntentCompleted,
}

impl EventType {
    pub const ALL: [EventType; 9] = [
        EventType::IntentCreated,
        EventType::PlanReviewRequired,
        EventType::NodeStarted,
        EventType::NodeCompleted,
        EventType::ArtifactCreated,
        EventType::VerificationFailed,
        EventType::DecisionRequired,
        EventType::EffectCommitted,
        EventType::IntentCompleted,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            EventType::IntentCreated => "intent.created",
            EventType::PlanReviewRequired => "plan.review_required",
            EventType::NodeStarted => "node.started",
            EventType::NodeCompleted => "node.completed",
            EventType::ArtifactCreated => "artifact.created",
            EventType::VerificationFailed => "verification.failed",
            EventType::DecisionRequired => "decision.required",
            EventType::EffectCommitted => "effect.committed",
            EventType::IntentCompleted => "intent.completed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Event {
    pub event_id: String,
    pub event_type: EventType,
    pub schema_version: u32,
    /// Unix 毫秒。
    pub occurred_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
    /// 未知事件类型在旧构建中是否可跳过（P0 设计第 5.2 节）。
    /// 九类既有事件恒为 false；后续新增类型按需置 true。
    #[serde(default)]
    pub ignorable: bool,
    pub payload: Value,
}

impl Event {
    pub fn new(
        event_type: EventType,
        event_id: impl Into<String>,
        occurred_at: i64,
        payload: Value,
    ) -> Self {
        Self {
            event_id: event_id.into(),
            event_type,
            schema_version: CURRENT_SCHEMA_VERSION,
            occurred_at,
            intent_id: None,
            node_id: None,
            ignorable: false,
            payload,
        }
    }

    /// 只有新增的事件类型才需要置为 true。九类既有事件不得调用本方法。
    pub fn with_ignorable(mut self, ignorable: bool) -> Self {
        self.ignorable = ignorable;
        self
    }

    pub fn with_intent(mut self, intent_id: impl Into<String>) -> Self {
        self.intent_id = Some(intent_id.into());
        self
    }

    pub fn with_node(mut self, node_id: impl Into<String>) -> Self {
        self.node_id = Some(node_id.into());
        self
    }
}
