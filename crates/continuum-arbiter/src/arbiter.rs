use crate::policy::{ArbiterPolicy, PolicyError};
use crate::pressure::{raw_pressure, ResourceObservation, ResourcePressure};
use crate::summary::ResourceSummary;
use crate::workload::WorkloadClass;

/// 仲裁入口。字段私有；`new` 是唯一构造点。
#[derive(Debug)]
pub struct ResourceArbiter {
    policy: ArbiterPolicy,
    pressure: ResourcePressure,
    last_pressure_change_millis: u64,
    workload_class: Option<WorkloadClass>,
}

impl ResourceArbiter {
    pub fn new(policy: ArbiterPolicy) -> Result<Self, PolicyError> {
        policy.validate()?;
        Ok(Self {
            policy,
            pressure: ResourcePressure::Low,
            last_pressure_change_millis: 0,
            workload_class: None,
        })
    }

    /// 更新压力与类别，返回当前结论。全 `None` 的读数不改变压力（设计 §5.1）。
    pub fn observe(
        &mut self,
        observation: &ResourceObservation,
        now_millis: u64,
    ) -> ResourceSummary {
        if let Some(raw) = raw_pressure(observation, &self.policy) {
            if raw > self.pressure {
                // 升压立即。
                self.pressure = raw;
                self.last_pressure_change_millis = now_millis;
            } else if raw < self.pressure
                && raw <= self.policy.resume_at
                && now_millis.saturating_sub(self.last_pressure_change_millis)
                    >= self.policy.cooldown_millis
            {
                // 降压只在跌到 resume_at 或以下且 cooldown 已过时生效。
                self.pressure = raw;
                self.last_pressure_change_millis = now_millis;
            }
        }

        // 每次 observation 都覆盖类别；None 就清空，不保留旧类别（设计 §5.1 第 5 步）。
        self.workload_class = observation.workload_class;
        self.summary()
    }

    /// 只读当前结论；供端口实现／装配方取快照。
    pub fn summary(&self) -> ResourceSummary {
        ResourceSummary::from_parts(self.pressure, self.workload_class)
    }
}
