//! 会话启动面：Manual／Auto 共用的 SessionPlan（设计 §4.5／§4.6）。

use continuum_intent_scheduler::DecisionBasis;
use continuum_semantics::{Intent, IntentId};

use crate::error::SupervisorError;
use crate::supervisor::{BackgroundSupervisor, SupervisorDecision};
use crate::task::AuthorizedTask;

/// 会话模式。两模式共用同一 `SessionPlan` 形状。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionMode {
    Manual,
    Auto,
}

/// Manual 的候选展示；Auto 从同一组候选里选。
#[derive(Debug, Clone)]
pub enum SessionCandidate {
    /// 既有会话。
    ContinueSession { intent: IntentId },
    /// P7D 的授权任务。
    Task(AuthorizedTask),
    /// 后台新结果：只展示，不参与排名。
    BackgroundFinding(BackgroundNote),
    /// 固定 UI 选项「开始新对话」；两模式都含。
    NewConversation,
}

/// 一条后台结果摘要。不透明文本；不是独立的 Evidence 类型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackgroundNote {
    label: String,
    at_millis: i64,
}

impl BackgroundNote {
    pub fn new(label: impl Into<String>, at_millis: i64) -> Result<Self, SupervisorError> {
        let label = label.into();
        if label.is_empty() {
            return Err(SupervisorError::InvalidBackgroundNote);
        }
        Ok(Self { label, at_millis })
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn at_millis(&self) -> i64 {
        self.at_millis
    }
}

/// 后台新结果的摘要面：completed 加 findings。两数组保持输入序，不排序。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BackgroundDigest {
    completed: Vec<BackgroundNote>,
    findings: Vec<BackgroundNote>,
}

impl BackgroundDigest {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn new(completed: Vec<BackgroundNote>, findings: Vec<BackgroundNote>) -> Self {
        Self {
            completed,
            findings,
        }
    }

    pub fn completed(&self) -> &[BackgroundNote] {
        &self.completed
    }

    pub fn findings(&self) -> &[BackgroundNote] {
        &self.findings
    }
}

/// 两模式共用的会话计划。字段私有；唯一构造点是 `plan_manual`／`plan_auto`。
/// `decision` 是选择的唯一表示；`selection()` 只是只读投影。
#[derive(Debug, Clone)]
pub struct SessionPlan {
    mode: SessionMode,
    candidates: Vec<SessionCandidate>,
    decision: Option<SupervisorDecision>,
    background: BackgroundDigest,
}

impl SessionPlan {
    pub fn mode(&self) -> SessionMode {
        self.mode
    }

    pub fn candidates(&self) -> &[SessionCandidate] {
        &self.candidates
    }

    pub fn decision(&self) -> Option<&SupervisorDecision> {
        self.decision.as_ref()
    }

    pub fn selection(&self) -> Option<&SupervisorDecision> {
        match self.decision.as_ref()? {
            SupervisorDecision::NoIntent { .. } => None,
            SupervisorDecision::Task { .. } | SupervisorDecision::ContinueSession { .. } => {
                self.decision.as_ref()
            }
        }
    }

    pub fn background(&self) -> &BackgroundDigest {
        &self.background
    }

    pub fn basis(&self) -> Option<&DecisionBasis> {
        self.decision
            .as_ref()
            .map(SupervisorDecision::basis)
    }
}

/// 会话编排器。持一个 `BackgroundSupervisor`，装配方经它共用同一张状态表。
#[derive(Debug, Default)]
pub struct SessionOrchestrator {
    supervisor: BackgroundSupervisor,
}

impl SessionOrchestrator {
    pub fn new(supervisor: BackgroundSupervisor) -> Self {
        Self { supervisor }
    }

    pub fn supervisor(&self) -> &BackgroundSupervisor {
        &self.supervisor
    }

    pub fn supervisor_mut(&mut self) -> &mut BackgroundSupervisor {
        &mut self.supervisor
    }

    /// Manual：只生成候选；`decision` 恒 `None`；不调 P7B。
    pub fn plan_manual(
        &self,
        candidates: &[Intent],
        tasks: &[AuthorizedTask],
        existing_sessions: &[IntentId],
        background: BackgroundDigest,
    ) -> SessionPlan {
        let visible = self.supervisor.next_tasks(candidates, tasks);
        let mut planned = Vec::new();
        for intent in existing_sessions {
            planned.push(SessionCandidate::ContinueSession {
                intent: intent.clone(),
            });
        }
        for task in visible {
            planned.push(SessionCandidate::Task(task.clone()));
        }
        for note in background.findings() {
            planned.push(SessionCandidate::BackgroundFinding(note.clone()));
        }
        planned.push(SessionCandidate::NewConversation);
        SessionPlan {
            mode: SessionMode::Manual,
            candidates: planned,
            decision: None,
            background,
        }
    }
}
