//! 事件解码与版本迁移链。
//!
//! 规则取 P0 设计第 5.1、5.2 节：版本迁移按相邻链组织，缺口链拒绝装配；
//! 未知事件类型默认为读时必需，只有 ignorable = true 才可跳过。

use crate::event::{Event, EventType};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EventLogError {
    #[error("未知事件类型 {event_type}，且未标记 ignorable")]
    UnknownEventType { event_type: String },
    #[error("解码器链缺少版本 {missing}")]
    CodecChainGap { missing: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkipReason {
    UnknownIgnorableType { event_type: String },
    MalformedPayload { message: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum DecodedEvent {
    Event(Event),
    Skippable(SkipReason),
}

type Decoder = Box<dyn Fn(&str) -> Result<DecodedEvent, EventLogError> + Send + Sync>;

/// 相邻版本迁移链。装配时校验版本号无缺口。
pub struct EventCodecChain {
    decoders: BTreeMap<u32, Decoder>,
}

impl EventCodecChain {
    pub fn new() -> Self {
        Self {
            decoders: BTreeMap::new(),
        }
    }

    pub fn register<F>(&mut self, version: u32, decoder: F)
    where
        F: Fn(&str) -> Result<DecodedEvent, EventLogError> + Send + Sync + 'static,
    {
        self.decoders.insert(version, Box::new(decoder));
    }

    /// 版本号必须从 1 开始连续。返回缺口处的最小缺失版本号。
    pub fn validate_contiguous(&self) -> Result<(), EventLogError> {
        for (i, version) in self.decoders.keys().enumerate() {
            let expected = i as u32 + 1;
            if *version != expected {
                return Err(EventLogError::CodecChainGap { missing: expected });
            }
        }
        Ok(())
    }

    pub fn decode(&self, version: u32, json: &str) -> Result<DecodedEvent, EventLogError> {
        match self.decoders.get(&version) {
            Some(decoder) => decoder(json),
            None => Err(EventLogError::CodecChainGap { missing: version }),
        }
    }
}

impl Default for EventCodecChain {
    fn default() -> Self {
        Self::new()
    }
}

/// 解码一条事件记录。
///
/// 返回 `Err` 只用于致命情形：事件类型未知且未标记 `ignorable`。
/// payload 畸形返回 `Ok(Skippable(..))`；是否致命由日志级调用方判定
/// （P0 设计第 4.1 节：可跳过事件之后存在 intent.completed 时升级为致命）。
pub fn decode_event(json: &str) -> Result<DecodedEvent, EventLogError> {
    let raw: Value = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(e) => {
            return Ok(DecodedEvent::Skippable(SkipReason::MalformedPayload {
                message: e.to_string(),
            }))
        }
    };

    let type_str = raw.get("event_type").and_then(Value::as_str).unwrap_or("");
    let ignorable = raw.get("ignorable").and_then(Value::as_bool).unwrap_or(false);

    if !EventType::ALL.iter().any(|t| t.as_str() == type_str) {
        if ignorable {
            return Ok(DecodedEvent::Skippable(SkipReason::UnknownIgnorableType {
                event_type: type_str.to_owned(),
            }));
        }
        return Err(EventLogError::UnknownEventType {
            event_type: type_str.to_owned(),
        });
    }

    match serde_json::from_value(raw) {
        Ok(event) => Ok(DecodedEvent::Event(event)),
        Err(e) => Ok(DecodedEvent::Skippable(SkipReason::MalformedPayload {
            message: e.to_string(),
        })),
    }
}
