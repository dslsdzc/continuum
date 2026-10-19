use crate::pressure::ResourcePressure;
use crate::workload::WorkloadClass;

/// 设计 §4.4：交给 AI 的仅是 `resource_pressure` 与 `workload_class`。
/// 字段私有；唯一公开构造点是 `ResourceArbiter::observe`／`summary`。
/// **没有优先级、进程名、PID、逐进程 CPU／GPU。**
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceSummary {
    resource_pressure: ResourcePressure,
    workload_class: Option<WorkloadClass>,
}

impl ResourceSummary {
    pub fn resource_pressure(&self) -> ResourcePressure {
        self.resource_pressure
    }

    pub fn workload_class(&self) -> Option<WorkloadClass> {
        self.workload_class
    }

    /// 仅供 `arbiter.rs` 的 `observe`／`summary` 使用；不是公开构造点。
    pub(crate) fn from_parts(
        resource_pressure: ResourcePressure,
        workload_class: Option<WorkloadClass>,
    ) -> Self {
        Self {
            resource_pressure,
            workload_class,
        }
    }
}
