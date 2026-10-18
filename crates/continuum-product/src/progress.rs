//! §4.5 的 Progress Gate：三值判定、调用方申报的 delta 与阈值。
//!
//! **不提供默认策略、不提供默认阈值**；`evaluate_progress` 不产生 `Decision`、
//! 不读写库。

use continuum_canonical::Opaque;
use continuum_semantics::{DecisionOption, ProductIdRef};

use crate::milestone::MilestoneId;
use crate::readiness::ProductReadiness;

/// §337 的三值判定。`Triggered` 只在有申报策略、阈值有效、且连续无进展达到
/// 阈值时给出。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgressGateVerdict {
    Triggered {
        consecutive_no_progress: u32,
        threshold: u32,
    },
    NotTriggered {
        consecutive_no_progress: u32,
        threshold: u32,
    },
    Undetermined {
        reason: ProgressUndetermined,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgressUndetermined {
    NoPolicy,
    InvalidThreshold,
    ReadinessUnassessed,
    DeltaUndetermined,
    HistoryUndetermined,
}

/// §154 的 `progress_delta`：四值。计算由 [`ProgressPolicy`] 给；P7A 只计数。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressDelta {
    Improved,
    NoProgress,
    Regressed,
    Undetermined,
}

/// §154 的记录：previous／current 产品状态与 `progress_delta`。
///
/// 不派生 `PartialEq`：两个 readiness 字段里的 `Opaque` 没有 `PartialEq`。
#[derive(Debug, Clone)]
pub struct ProgressRecord {
    pub product_id: ProductIdRef,
    pub milestone_id: MilestoneId,
    pub previous_product_state: ProductReadiness,
    pub current_product_state: ProductReadiness,
    pub progress_delta: ProgressDelta,
    pub recorded_at: i64,
}

/// 调用方申报的 delta 与阈值。**P7A 不提供默认实现**。
pub trait ProgressPolicy {
    fn delta(
        &self,
        previous: &ProductReadiness,
        current: &ProductReadiness,
    ) -> ProgressDelta;

    fn stall_threshold(&self) -> u32;
}

/// 调用方给的 replan `Decision` 内容；P7A 不生成路线，也不改 Contract／Plan。
#[derive(Debug, Clone)]
pub struct ReplanDraft {
    pub issue: Opaque,
    pub options: Vec<DecisionOption>,
    pub blocking: bool,
}

/// §4.5 的八步判定：
///
/// 1. `policy == None` ⇒ `Undetermined(NoPolicy)`；
/// 2. `threshold == 0` ⇒ `Undetermined(InvalidThreshold)`；
/// 3. previous 与 current 九维全 `Unassessed` ⇒ `Undetermined(ReadinessUnassessed)`；
/// 4. `delta = policy.delta(..)`；`Undetermined` ⇒ `Undetermined(DeltaUndetermined)`；
/// 5. 当前 `Improved`／`Regressed` ⇒ `NotTriggered { 0, threshold }`（不看历史）；
/// 6. 当前 `NoProgress` ⇒ `count = 1`；从 `history` 最近一条往回加 `NoProgress`，
///    遇 `Improved`／`Regressed` 停止；在达到 threshold 之前遇 `Undetermined`
///    ⇒ `Undetermined(HistoryUndetermined)`；
/// 7. 任一时点 `count >= threshold` ⇒ `Triggered`；
/// 8. 否则 `NotTriggered`；`Regressed` 不触发 replan。
pub fn evaluate_progress(
    previous: &ProductReadiness,
    current: &ProductReadiness,
    history: &[ProgressRecord],
    policy: Option<&dyn ProgressPolicy>,
) -> ProgressGateVerdict {
    let Some(policy) = policy else {
        return ProgressGateVerdict::Undetermined {
            reason: ProgressUndetermined::NoPolicy,
        };
    };

    let threshold = policy.stall_threshold();
    if threshold == 0 {
        return ProgressGateVerdict::Undetermined {
            reason: ProgressUndetermined::InvalidThreshold,
        };
    }

    if previous.iter().into_iter().all(|(_, d)| d.value().is_unassessed())
        && current.iter().into_iter().all(|(_, d)| d.value().is_unassessed())
    {
        return ProgressGateVerdict::Undetermined {
            reason: ProgressUndetermined::ReadinessUnassessed,
        };
    }

    match policy.delta(previous, current) {
        ProgressDelta::Undetermined => ProgressGateVerdict::Undetermined {
            reason: ProgressUndetermined::DeltaUndetermined,
        },
        ProgressDelta::Improved | ProgressDelta::Regressed => {
            ProgressGateVerdict::NotTriggered {
                consecutive_no_progress: 0,
                threshold,
            }
        }
        ProgressDelta::NoProgress => {
            let mut count = 1;
            for record in history.iter().rev() {
                match record.progress_delta {
                    ProgressDelta::NoProgress => count += 1,
                    ProgressDelta::Improved | ProgressDelta::Regressed => break,
                    ProgressDelta::Undetermined if count < threshold => {
                        return ProgressGateVerdict::Undetermined {
                            reason: ProgressUndetermined::HistoryUndetermined,
                        }
                    }
                    ProgressDelta::Undetermined => break,
                }
                if count >= threshold {
                    return ProgressGateVerdict::Triggered {
                        consecutive_no_progress: count,
                        threshold,
                    };
                }
            }
            if count >= threshold {
                ProgressGateVerdict::Triggered {
                    consecutive_no_progress: count,
                    threshold,
                }
            } else {
                ProgressGateVerdict::NotTriggered {
                    consecutive_no_progress: count,
                    threshold,
                }
            }
        }
    }
}
