use std::fmt;

use crate::pressure::ResourcePressure;

/// 压力阈值、滞回与 checkpoint 策略。字段公开（配置面），`ResourceArbiter::new` 校验。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArbiterPolicy {
    pub moderate_at: u8,
    pub high_at: u8,
    pub critical_at: u8,
    pub resume_at: ResourcePressure,
    pub cooldown_millis: u64,
    pub checkpoint_timeout_millis: u64,
    pub max_checkpoint_attempts: u32,
}

impl Default for ArbiterPolicy {
    fn default() -> Self {
        Self {
            moderate_at: 50,
            high_at: 75,
            critical_at: 90,
            resume_at: ResourcePressure::Low,
            cooldown_millis: 30_000,
            checkpoint_timeout_millis: 60_000,
            max_checkpoint_attempts: 3,
        }
    }
}

impl ArbiterPolicy {
    /// 校验阈值次序与恢复阈值。`1 <= moderate_at < high_at < critical_at <= 100`，
    /// 且 `resume_at < ResourcePressure::High`（设计 §4.11）。
    pub(crate) fn validate(&self) -> Result<(), PolicyError> {
        if self.moderate_at == 0
            || self.moderate_at >= self.high_at
            || self.high_at >= self.critical_at
            || self.critical_at > 100
        {
            return Err(PolicyError::ThresholdsNotIncreasing);
        }
        if self.resume_at >= ResourcePressure::High {
            return Err(PolicyError::ResumeAtTooHigh);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyError {
    /// 需要 `1 <= moderate_at < high_at < critical_at <= 100`。
    ThresholdsNotIncreasing,
    /// 需要 `resume_at < ResourcePressure::High`。
    ResumeAtTooHigh,
}

impl fmt::Display for PolicyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ThresholdsNotIncreasing => {
                f.write_str("压力阈值必须满足 1 <= moderate_at < high_at < critical_at <= 100")
            }
            Self::ResumeAtTooHigh => {
                f.write_str("resume_at 必须低于 ResourcePressure::High")
            }
        }
    }
}

impl std::error::Error for PolicyError {}
