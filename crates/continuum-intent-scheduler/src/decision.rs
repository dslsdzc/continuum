use continuum_semantics::{Intent, IntentId, IntentStatus};

use crate::inputs::{Blocking, SchedulerInputs, UserFocus};

/// §302 的「现在推进哪个 Intent」。
///
/// **两臂封闭**：`Advance` 带一个候选的 `IntentId`；`NoIntent` **不带任何 `IntentId`**。
/// `NoIntent` 不是哨兵 `IntentId`：空串在语义层是合法值，若用它当「没有 Intent」，
/// 调用方会把空串当成一枚真 Intent。本 crate 不提供这条路径。
///
/// `decide` 返回 `Result<IntentDecision, IntentDecisionError>`；策略违约与策略失败走错误臂，
/// **不**折叠成 `NoIntent`。
///
/// `Advance` 只表示「这一枚现在值得推进」；它不承诺「启动一个新的 READY」还是「继续一个活跃会话」。
/// P7D 按候选的 `IntentStatus` 分派。
#[derive(Debug, Clone)]
pub enum IntentDecision {
    Advance { intent: IntentId, basis: DecisionBasis },
    NoIntent { basis: DecisionBasis },
}

impl IntentDecision {
    pub fn advanced(&self) -> Option<&IntentId> {
        match self {
            Self::Advance { intent, .. } => Some(intent),
            Self::NoIntent { .. } => None,
        }
    }

    pub fn basis(&self) -> &DecisionBasis {
        match self {
            Self::Advance { basis, .. } | Self::NoIntent { basis } => basis,
        }
    }

    pub fn is_no_intent(&self) -> bool {
        matches!(self, Self::NoIntent { .. })
    }
}

/// 决策的不透明依据。P7B 不解释内容；P7D 可以展示，但不得按内容分支。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionBasis(String);

impl DecisionBasis {
    pub fn new(detail: impl Into<String>) -> Self { Self(detail.into()) }
    pub fn as_str(&self) -> &str { &self.0 }
}

/// `decide` 的失败面。`IntentDecision` 保持两臂；策略失败与越界不折叠成 `NoIntent`。
#[derive(Debug, Clone)]
pub enum IntentDecisionError {
    /// 策略自身失败（模型不可用／无法判断）：`choose` 返回的 `Err`。
    PolicyFailed { basis: DecisionBasis },
    /// 策略返回的 `Advance` 不在候选集，或候选状态不可推进。
    InvalidAdvance { intent: IntentId },
}

impl std::fmt::Display for IntentDecisionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PolicyFailed { basis } => write!(f, "policy failed: {}", basis.as_str()),
            Self::InvalidAdvance { intent } => {
                write!(f, "policy chose ineligible intent {}", intent.as_str())
            }
        }
    }
}

impl std::error::Error for IntentDecisionError {}

/// §302 五输入的组合面。**评分算法、权重、模型提示不冻**。
///
/// 这是排序策略的端口，**不是通用模型端口**：参数是 `SchedulerInputs`，
/// 返回 `Result<IntentDecision, DecisionBasis>`；P7B 不在这里调 `ModelProvider`。
///
/// `Ok(NoIntent)` = 策略明确选择本轮不推进；`Err(basis)` = 策略自身失败
/// （模型不可用／无法判断），**不是**「没有工作」。`decide` 把后者包成
/// `IntentDecisionError::PolicyFailed`，不折叠成 `NoIntent`。
/// **失败只能走 `Err`**：策略实现不得用 panic 表达可恢复失败；P7B 不捕获 panic。
pub trait IntentSelectionPolicy {
    fn choose(
        &self,
        inputs: &SchedulerInputs<'_>,
    ) -> Result<IntentDecision, DecisionBasis>;
}

/// 候选状态是否可推进：`Ready`（启动）与五个活跃状态（继续／恢复）。
/// `Running`／`Verifying` 已在运行，继续它们不需要状态迁移；`Evolving` 仅在 P7D
/// 确有对应活跃会话时才继续，且同样不需要迁移。`is_advanceable` 只判状态合法性，
/// 不检查会话是否存在。
pub(crate) fn is_advanceable(status: IntentStatus) -> bool {
    matches!(
        status,
        IntentStatus::Ready
            | IntentStatus::Running
            | IntentStatus::Waiting
            | IntentStatus::Suspended
            | IntentStatus::Verifying
            | IntentStatus::Evolving
    )
}

/// 确定性基线。**它不是规范排名**；它是 P7D 可以直接用的默认策略。
///
/// 次序（本设计自定）：可推进候选为空 ⇒ `NoIntent`；`UserFocus::Intent` 命中；
/// 最早 deadline；最大 blocking 去重度；输入序第一枚。`priority` 与
/// `background_suitability` **不参与基线**。
#[derive(Debug, Default, Clone, Copy)]
pub struct BaselineIntentPolicy;

impl IntentSelectionPolicy for BaselineIntentPolicy {
    fn choose(&self, inputs: &SchedulerInputs<'_>) -> Result<IntentDecision, DecisionBasis> {
        let eligible: Vec<&Intent> = inputs
            .candidates
            .iter()
            .filter(|candidate| is_advanceable(candidate.status))
            .collect();
        if eligible.is_empty() {
            return Ok(IntentDecision::NoIntent {
                basis: DecisionBasis::new("no_advanceable_candidate"),
            });
        }

        if let UserFocus::Intent(focus) = &inputs.user_focus {
            if eligible
                .iter()
                .any(|candidate| candidate.id.as_str() == focus.as_str())
            {
                return Ok(IntentDecision::Advance {
                    intent: focus.clone(),
                    basis: DecisionBasis::new("user_focus"),
                });
            }
        }

        // 每个候选先取自己最早的 deadline；min_by_key 并列取第一个。
        let deadline_pick = eligible
            .iter()
            .filter_map(|candidate| {
                inputs
                    .deadline
                    .iter()
                    .filter(|deadline| deadline.intent.as_str() == candidate.id.as_str())
                    .map(|deadline| deadline.at_millis)
                    .min()
                    .map(|at| (*candidate, at))
            })
            .min_by_key(|(_, at)| *at);
        if let Some((candidate, _)) = deadline_pick {
            return Ok(IntentDecision::Advance {
                intent: candidate.id.clone(),
                basis: DecisionBasis::new("earliest_deadline"),
            });
        }

        // blocking：合并 + 按 as_str 去重 + 严格 > 保留输入序第一个最大值。
        let mut best: Option<(&Intent, usize)> = None;
        for candidate in &eligible {
            let records: Vec<&Blocking> = inputs
                .blocking
                .iter()
                .filter(|blocking| blocking.intent.as_str() == candidate.id.as_str())
                .collect();
            if records.is_empty() {
                continue;
            }
            let mut blocked: Vec<&str> = records
                .iter()
                .flat_map(|blocking| blocking.blocks.iter().map(|id| id.as_str()))
                .collect();
            blocked.sort_unstable();
            blocked.dedup();
            let count = blocked.len();
            match best {
                Some((_, best_count)) if count <= best_count => {}
                _ => best = Some((*candidate, count)),
            }
        }
        if let Some((candidate, _)) = best {
            return Ok(IntentDecision::Advance {
                intent: candidate.id.clone(),
                basis: DecisionBasis::new("blocking_degree"),
            });
        }

        let candidate = eligible[0];
        Ok(IntentDecision::Advance {
            intent: candidate.id.clone(),
            basis: DecisionBasis::new("candidate_order"),
        })
    }
}

/// Intent Scheduler 的唯一决策入口（§302）。
#[derive(Debug, Default, Clone, Copy)]
pub struct IntentScheduler;

impl IntentScheduler {
    /// 调策略，并校验策略的输出：
    /// - 策略返回 `Err(basis)` ⇒ `Err(IntentDecisionError::PolicyFailed { basis })`；
    /// - `Advance` 的 id 必须是 `candidates` 里的一枚且状态可推进，否则
    ///   `Err(IntentDecisionError::InvalidAdvance { intent })`；
    /// - `NoIntent` 原样返回；`basis` 为空时补 `"no_intent"`（`decide` 的不变量）。
    ///
    /// **不迁移状态**：`Advance` 只给 id；P7D 按候选状态走 §223 的既有迁移或不迁移。
    pub fn decide(
        inputs: &SchedulerInputs<'_>,
        policy: &dyn IntentSelectionPolicy,
    ) -> Result<IntentDecision, IntentDecisionError> {
        let decision = policy
            .choose(inputs)
            .map_err(|basis| IntentDecisionError::PolicyFailed { basis })?;
        match decision {
            IntentDecision::Advance { intent, basis } => {
                let eligible = inputs.candidates.iter().any(|candidate| {
                    candidate.id.as_str() == intent.as_str()
                        && is_advanceable(candidate.status)
                });
                if eligible {
                    Ok(IntentDecision::Advance { intent, basis })
                } else {
                    Err(IntentDecisionError::InvalidAdvance { intent })
                }
            }
            IntentDecision::NoIntent { basis } => {
                let basis = if basis.as_str().trim().is_empty() {
                    DecisionBasis::new("no_intent")
                } else {
                    basis
                };
                Ok(IntentDecision::NoIntent { basis })
            }
        }
    }
}
