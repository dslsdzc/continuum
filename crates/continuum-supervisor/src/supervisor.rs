//! Background Supervisor：任务过滤、选择与资源控制面（设计 §4.4／§4.7）。

use std::collections::HashMap;

use continuum_arbiter::{
    ResourceArbiter, ResourceDecision, SuspensionError, SuspensionPhase, SuspensionState,
};
use continuum_intent_scheduler::{
    DecisionBasis, IntentDecision, IntentScheduler, IntentSelectionPolicy, SchedulerInputs,
};
use continuum_semantics::{Intent, IntentId, IntentStatus};

use crate::error::SupervisorError;
use crate::resource::{declaration_for, CheckpointPort};
use crate::task::AuthorizedTask;

/// 后台治理入口。持有每任务的 `SuspensionState` 内存表。
/// 不建任务内容表：`AuthorizedTask` 不存；表只存相位／尝试数／截止。
#[derive(Debug, Default)]
pub struct BackgroundSupervisor {
    states: HashMap<TaskKey, SuspensionState>,
}

/// 状态表的私有键。不是公开身份类型，不出现在公开 API。
/// `IntentId` 没有 `Hash`／`Eq`，故 Intent 任务用 `as_str()` 的 `String`；
/// policy／maintenance 用其不透明引用。
///
/// **一 ref 一任务契约**：同一 `PolicyGrantRef`／`MaintenanceOccurrenceRef` 在一个
/// supervisor 实例里只应标识一个任务。两枚任务共用同一引用会共享同一
/// `SuspensionState`；这是调用方契约违反，P7D 不检测、不去重。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum TaskKey {
    Intent(String),
    Policy { issuer: String, grant_id: String },
    Maintenance {
        schedule_id: String,
        occurrence_id: String,
    },
}

impl TaskKey {
    fn for_task(task: &AuthorizedTask) -> Self {
        if let Some(intent) = task.intent() {
            return Self::Intent(intent.id.as_str().to_owned());
        }
        if let Some(grant) = task.policy_grant() {
            return Self::Policy {
                issuer: grant.issuer().to_owned(),
                grant_id: grant.grant_id().to_owned(),
            };
        }
        let schedule = task
            .maintenance_schedule()
            .expect("AuthorizedTask 的不变量：无 Intent 时必有 policy 或 maintenance 引用");
        Self::Maintenance {
            schedule_id: schedule.schedule_id().to_owned(),
            occurrence_id: schedule.occurrence_id().to_owned(),
        }
    }
}

impl BackgroundSupervisor {
    /// 本轮可运行的任务集合。只过滤，不排序：保持 `tasks` 的输入序。
    /// 有 Intent 的任务只保留「候选集中存在同 id 候选且该候选当前 READY」的；
    /// policy／maintenance 任务无条件保留。
    pub fn next_tasks<'a>(
        &self,
        candidates: &[Intent],
        tasks: &'a [AuthorizedTask],
    ) -> Vec<&'a AuthorizedTask> {
        tasks
            .iter()
            .filter(|task| {
                let Some(intent) = task.intent() else {
                    return true;
                };
                candidates
                    .iter()
                    .find(|candidate| candidate.id.as_str() == intent.id.as_str())
                    .is_some_and(|candidate| candidate.status == IntentStatus::Ready)
            })
            .collect()
    }

    /// 只读当前相位；无状态返回 `None`。
    pub fn suspension_phase(&self, task: &AuthorizedTask) -> Option<SuspensionPhase> {
        self.states
            .get(&TaskKey::for_task(task))
            .map(SuspensionState::phase)
    }

    /// 把任务交给 P7C 的 `ResourceArbiter::direct`；返回结论原样。
    /// 首次调用插入新状态，之后复用同一枚。
    pub fn admit(
        &mut self,
        task: &AuthorizedTask,
        arbiter: &ResourceArbiter,
        now_millis: u64,
    ) -> Result<ResourceDecision, SupervisorError> {
        let key = TaskKey::for_task(task);
        let state = self
            .states
            .entry(key)
            .or_insert_with(SuspensionState::new);
        arbiter
            .direct(declaration_for(task), state, now_millis)
            .map_err(SupervisorError::Resource)
    }

    /// 任务终止时丢弃它的状态；返回此前是否存在。P7D 不推断终止。
    pub fn release(&mut self, task: &AuthorizedTask) -> bool {
        self.states.remove(&TaskKey::for_task(task)).is_some()
    }

    /// 执行一次 checkpoint 并推进 P7C 的相位。三个出口：
    /// 截止前成功进入 `Suspended`；晚到成功与端口失败都回 `Throttled`。
    pub fn checkpoint(
        &mut self,
        task: &AuthorizedTask,
        port: &dyn CheckpointPort,
        now_millis: u64,
    ) -> Result<SuspensionPhase, SupervisorError> {
        let key = TaskKey::for_task(task);
        let Some(state) = self.states.get_mut(&key) else {
            return Err(SupervisorError::NoSuspensionState {
                source: task.source(),
            });
        };
        if state.phase() != SuspensionPhase::Checkpointing {
            return Err(SupervisorError::Resource(
                SuspensionError::NotCheckpointing {
                    phase: state.phase(),
                },
            ));
        }
        match port.checkpoint(task) {
            Ok(()) => match state.checkpoint_succeeded(now_millis) {
                Ok(phase) => Ok(phase),
                Err(error) => Err(SupervisorError::Resource(error)),
            },
            Err(port_error) => match state.checkpoint_failed(now_millis) {
                SuspensionError::CheckpointFailed => Err(SupervisorError::Checkpoint {
                    error: port_error,
                    phase: SuspensionPhase::Throttled,
                }),
                other => Err(SupervisorError::Resource(other)),
            },
        }
    }

    /// 定时器出口：调 P7C 的 `checkpoint_timed_out`；没有 `Ok` 路径。
    pub fn checkpoint_timed_out(
        &mut self,
        task: &AuthorizedTask,
        now_millis: u64,
    ) -> Result<(), SupervisorError> {
        let key = TaskKey::for_task(task);
        let Some(state) = self.states.get_mut(&key) else {
            return Err(SupervisorError::NoSuspensionState {
                source: task.source(),
            });
        };
        let error = state.checkpoint_timed_out(now_millis);
        Err(SupervisorError::Resource(error))
    }

    /// P7C 的 checkpoint 截止时刻；无状态或不在 `Checkpointing` 返回 `None`。
    pub fn checkpoint_deadline(&self, task: &AuthorizedTask) -> Option<u64> {
        self.states
            .get(&TaskKey::for_task(task))
            .and_then(SuspensionState::checkpoint_deadline_millis)
    }

    /// 唯一的选择入口：调 P7B 的 `IntentScheduler::decide`，按候选当前状态分派。
    /// P7D 不实现 `IntentSelectionPolicy`，不做第二套候选排序。
    pub fn decide(
        &self,
        inputs: &SchedulerInputs<'_>,
        policy: &dyn IntentSelectionPolicy,
        tasks: &[AuthorizedTask],
        existing_sessions: &[IntentId],
    ) -> Result<SupervisorDecision, SupervisorError> {
        let decision =
            IntentScheduler::decide(inputs, policy).map_err(SupervisorError::Decision)?;
        match &decision {
            IntentDecision::NoIntent { .. } => Ok(SupervisorDecision::NoIntent { decision }),
            IntentDecision::Advance { intent, .. } => {
                let intent = intent.clone();
                let candidate = inputs
                    .candidates
                    .iter()
                    .find(|candidate| candidate.id.as_str() == intent.as_str())
                    .ok_or_else(|| SupervisorError::UnsupportedAdvance {
                        intent: intent.clone(),
                    })?;
                match candidate.status {
                    IntentStatus::Ready => {
                        let task = tasks
                            .iter()
                            .find(|task| {
                                task.intent()
                                    .is_some_and(|value| value.id.as_str() == intent.as_str())
                            })
                            .ok_or_else(|| SupervisorError::UnauthorizedAdvance {
                                intent: intent.clone(),
                            })?;
                        Ok(SupervisorDecision::Task {
                            task: task.clone(),
                            decision,
                        })
                    }
                    IntentStatus::Waiting | IntentStatus::Suspended => {
                        Ok(SupervisorDecision::ContinueSession {
                            intent,
                            disposition: ContinueDisposition::Resume,
                            decision,
                        })
                    }
                    IntentStatus::Running
                    | IntentStatus::Verifying
                    | IntentStatus::Evolving => {
                        let has_session = existing_sessions
                            .iter()
                            .any(|session| session.as_str() == intent.as_str());
                        if !has_session {
                            return Err(SupervisorError::NoActiveSession { intent });
                        }
                        Ok(SupervisorDecision::ContinueSession {
                            intent,
                            disposition: ContinueDisposition::Continue,
                            decision,
                        })
                    }
                    _ => Err(SupervisorError::UnsupportedAdvance { intent }),
                }
            }
        }
    }
}

/// `ContinueSession` 的控制值。不是 P7C 的 `ResourceAction`，也不是相位。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContinueDisposition {
    /// `Running`／`Verifying`／`Evolving`：会话已在运行，交给执行器继续。
    Continue,
    /// `Waiting`／`Suspended`：P7B 的恢复分支；不要求既有会话。
    Resume,
}

/// `decide` 的结果：只三臂。
#[derive(Debug, Clone)]
pub enum SupervisorDecision {
    /// P7B 选中了一枚 READY Intent，且它有对应的 `AuthorizedTask`。
    Task {
        task: AuthorizedTask,
        decision: IntentDecision,
    },
    /// P7B 选中了一枚活跃 Intent；不迁移状态。
    ContinueSession {
        intent: IntentId,
        disposition: ContinueDisposition,
        decision: IntentDecision,
    },
    /// P7B 明确 `NoIntent`；既有会话与后台任务必须保留。
    NoIntent { decision: IntentDecision },
}

impl SupervisorDecision {
    pub fn intent_decision(&self) -> &IntentDecision {
        match self {
            Self::Task { decision, .. }
            | Self::ContinueSession { decision, .. }
            | Self::NoIntent { decision } => decision,
        }
    }

    pub fn selected_intent(&self) -> Option<&IntentId> {
        match self {
            Self::Task { task, .. } => task.intent().map(|intent| &intent.id),
            Self::ContinueSession { intent, .. } => Some(intent),
            Self::NoIntent { .. } => None,
        }
    }

    pub fn basis(&self) -> &DecisionBasis {
        self.intent_decision().basis()
    }

    pub fn continue_disposition(&self) -> Option<ContinueDisposition> {
        match self {
            Self::ContinueSession { disposition, .. } => Some(*disposition),
            Self::Task { .. } | Self::NoIntent { .. } => None,
        }
    }
}
