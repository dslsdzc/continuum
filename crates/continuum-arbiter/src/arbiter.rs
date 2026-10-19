use crate::action::ResourceAction;
use crate::policy::{ArbiterPolicy, PolicyError};
use crate::pressure::{raw_pressure, ResourceObservation, ResourcePressure};
use crate::priority::{classify, PriorityTier};
use crate::state::{SuspensionError, SuspensionPhase, SuspensionState};
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

    /// 对一份工作计算动作并推进其状态机（设计 §5.4／§5.5）。
    /// `Suspend` 只把相位推进到 `Checkpointing`；`action=None` 表示无转移。
    pub fn direct(
        &self,
        declaration: crate::declaration::WorkloadDeclaration,
        state: &mut SuspensionState,
        now_millis: u64,
    ) -> Result<ResourceDecision, SuspensionError> {
        match state.phase() {
            SuspensionPhase::Checkpointing => Ok(ResourceDecision {
                summary: self.summary(),
                action: None,
                phase: SuspensionPhase::Checkpointing,
            }),
            SuspensionPhase::Suspended => {
                if self.pressure <= self.policy.resume_at
                    && now_millis.saturating_sub(state.last_transition_millis())
                        >= self.policy.cooldown_millis
                {
                    state.resume(now_millis);
                    Ok(ResourceDecision {
                        summary: self.summary(),
                        action: Some(ResourceAction::Resume),
                        phase: state.phase(),
                    })
                } else {
                    Ok(ResourceDecision {
                        summary: self.summary(),
                        action: None,
                        phase: SuspensionPhase::Suspended,
                    })
                }
            }
            SuspensionPhase::Running => {
                let (action, phase) =
                    match ladder_action(classify(declaration), self.pressure) {
                        ResourceAction::Run => (None, SuspensionPhase::Running),
                        ResourceAction::Throttle => {
                            state.throttle(now_millis);
                            (Some(ResourceAction::Throttle), state.phase())
                        }
                        ResourceAction::Suspend => {
                            state.enter_checkpointing(
                                now_millis,
                                self.policy.checkpoint_timeout_millis,
                            );
                            (Some(ResourceAction::Suspend), state.phase())
                        }
                        // `direct` 不发出 `Resume`：它只由 `Suspended` 分支在恢复条件满足时发出。
                        ResourceAction::Resume => (None, SuspensionPhase::Running),
                    };
                Ok(ResourceDecision {
                    summary: self.summary(),
                    action,
                    phase,
                })
            }
            SuspensionPhase::Throttled => {
                let (action, phase) =
                    match ladder_action(classify(declaration), self.pressure) {
                        ResourceAction::Run => {
                            state.run(now_millis);
                            (Some(ResourceAction::Run), state.phase())
                        }
                        ResourceAction::Throttle => (None, SuspensionPhase::Throttled),
                        ResourceAction::Suspend => {
                            state.enter_checkpointing(
                                now_millis,
                                self.policy.checkpoint_timeout_millis,
                            );
                            (Some(ResourceAction::Suspend), state.phase())
                        }
                        // `direct` 不发出 `Resume`：它只由 `Suspended` 分支在恢复条件满足时发出。
                        ResourceAction::Resume => (None, SuspensionPhase::Throttled),
                    };
                Ok(ResourceDecision {
                    summary: self.summary(),
                    action,
                    phase,
                })
            }
        }
    }
}

/// 设计 §5.4 的 4×4 动作阶梯。`Resume` 不在这张表里。
fn ladder_action(priority: PriorityTier, pressure: ResourcePressure) -> ResourceAction {
    match priority {
        PriorityTier::UserWorkload => ResourceAction::Run,
        PriorityTier::ForegroundRuntime => match pressure {
            ResourcePressure::Critical => ResourceAction::Throttle,
            _ => ResourceAction::Run,
        },
        PriorityTier::BackgroundRuntime => match pressure {
            ResourcePressure::Critical => ResourceAction::Suspend,
            ResourcePressure::High => ResourceAction::Throttle,
            _ => ResourceAction::Run,
        },
        PriorityTier::Maintenance => match pressure {
            ResourcePressure::Critical | ResourcePressure::High => ResourceAction::Suspend,
            ResourcePressure::Moderate => ResourceAction::Throttle,
            ResourcePressure::Low => ResourceAction::Run,
        },
    }
}

/// 一次 `direct` 的结论。对 P7D：summary（AI／后台两项）＋ action（四档之一或 None）＋ phase。
/// **没有优先级**；`action == None` 表示无转移，不是第五动作。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResourceDecision {
    summary: ResourceSummary,
    action: Option<ResourceAction>,
    phase: SuspensionPhase,
}

impl ResourceDecision {
    pub fn summary(&self) -> ResourceSummary {
        self.summary
    }

    pub fn action(&self) -> Option<ResourceAction> {
        self.action
    }

    pub fn phase(&self) -> SuspensionPhase {
        self.phase
    }
}
