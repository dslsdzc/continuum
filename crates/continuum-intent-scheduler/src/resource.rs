use continuum_graph::NodeId;
use continuum_semantics::{Intent, IntentId, Priority};

use crate::decision::{IntentDecision, IntentScheduler, is_advanceable};
use crate::inputs::{BackgroundSuitability, Deadline, SchedulerInputs};

/// §304 的具名资源端口。**P7B 定义 trait，装配方实现**。
///
/// 实现方持有 P7C 的结论（资源压力、负载类别与四档优先级），本 crate 不引入
/// P7C 的任何类型；`direct` 只收 [`ExecutionRequest`]。
pub trait ResourceDirectivePort {
    fn direct(
        &self,
        request: &ExecutionRequest<'_>,
    ) -> Result<ResourceDirective, ResourceDirectiveError>;
}

/// 一次执行请求：Intent ＋ 可选 Graph 节点 ＋ 显式信号。
#[derive(Debug)]
pub struct ExecutionRequest<'a> {
    pub intent: &'a Intent,
    pub node: Option<&'a NodeId>,
    pub signals: ExecutionSignals<'a>,
}

/// 进入资源请求的显式信号。`user focus` / `blocking` 只影响 Intent 选择，
/// 不进入资源请求；装配方需要时它已持有 [`SchedulerInputs`]。
#[derive(Debug)]
pub struct ExecutionSignals<'a> {
    pub priority: Option<&'a Priority>,
    pub deadline: Option<&'a Deadline>,
    /// 整轮单值；不是每候选一条。
    pub background_suitability: &'a BackgroundSuitability,
}

/// §304 四问的不透明载体。**槽存在是结构保证；槽内容由装配方定义，P7B 不解析**。
/// `None` 表示「该问未答／不可判」，**不是**默认值。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceDirective {
    node_placement: Option<String>,
    model: Option<String>,
    tool: Option<String>,
    parallelism: Option<String>,
}

impl ResourceDirective {
    pub fn new(
        node_placement: Option<String>,
        model: Option<String>,
        tool: Option<String>,
        parallelism: Option<String>,
    ) -> Self {
        Self { node_placement, model, tool, parallelism }
    }

    pub fn node_placement(&self) -> Option<&str> { self.node_placement.as_deref() }
    pub fn model(&self) -> Option<&str> { self.model.as_deref() }
    pub fn tool(&self) -> Option<&str> { self.tool.as_deref() }
    pub fn parallelism(&self) -> Option<&str> { self.parallelism.as_deref() }
}

/// 端口错误。**不透明**：P7B 不解释错误类别；装配方把端口内部的错误映射进来。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceDirectiveError(String);

impl ResourceDirectiveError {
    pub fn new(message: impl Into<String>) -> Self { Self(message.into()) }
    pub fn message(&self) -> &str { &self.0 }
}

impl std::fmt::Display for ResourceDirectiveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ResourceDirectiveError {}

/// `directive_for` 的错误面：契约违反与端口失败分开，不折叠成 `Ok(None)`。
#[derive(Debug, Clone)]
pub enum DirectiveForError {
    /// `Advance` 的 id 不在候选集，或候选状态不可推进。
    InvalidDecision { intent: IntentId },
    /// 端口实现失败，原样透传。
    Port(ResourceDirectiveError),
}

impl std::fmt::Display for DirectiveForError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidDecision { intent } => {
                write!(f, "advance id {} is not an advanceable candidate", intent.as_str())
            }
            Self::Port(error) => write!(f, "resource directive port failed: {error}"),
        }
    }
}

impl std::error::Error for DirectiveForError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Port(error) => Some(error),
            Self::InvalidDecision { .. } => None,
        }
    }
}

impl IntentScheduler {
    /// 把一次已作出的 `IntentDecision` 交给资源端口，并原样返回不透明指令值。
    ///
    /// - `NoIntent` ⇒ `Ok(None)`，**不调用端口**；
    /// - `Advance` 的 id 不在候选集或状态不可推进 ⇒ `Err(InvalidDecision)`（契约违反）；
    /// - 端口错误 ⇒ `Err(Port(e))`，原样透传，不包装。
    ///
    /// 同一 Intent 多条 `priority` 取输入序第一条；多条 `deadline` 取最早；
    /// `background_suitability` 是整轮单值。
    pub fn directive_for(
        inputs: &SchedulerInputs<'_>,
        decision: &IntentDecision,
        node: Option<&NodeId>,
        port: &dyn ResourceDirectivePort,
    ) -> Result<Option<ResourceDirective>, DirectiveForError> {
        let IntentDecision::Advance { intent, .. } = decision else {
            return Ok(None);
        };

        let candidate = inputs
            .candidates
            .iter()
            .find(|candidate| candidate.id.as_str() == intent.as_str())
            .filter(|candidate| is_advanceable(candidate.status))
            .ok_or_else(|| DirectiveForError::InvalidDecision {
                intent: intent.clone(),
            })?;

        let priority = inputs
            .priority
            .iter()
            .find(|signal| signal.intent.as_str() == intent.as_str())
            .map(|signal| &signal.priority);
        let deadline = inputs
            .deadline
            .iter()
            .filter(|deadline| deadline.intent.as_str() == intent.as_str())
            .min_by_key(|deadline| deadline.at_millis);

        let request = ExecutionRequest {
            intent: candidate,
            node,
            signals: ExecutionSignals {
                priority,
                deadline,
                background_suitability: &inputs.background_suitability,
            },
        };

        port.direct(&request).map(Some).map_err(DirectiveForError::Port)
    }
}
