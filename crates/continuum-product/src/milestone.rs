//! §234 的 `Milestone` 七字段、七状态与有界批次的惰性生成。
//!
//! **唯一的公开产生路径**是 [`propose_next_batch`]（生成）与 `load_milestone(s)`
//! （读回，C2 后）。外部写不出 `Milestone { .. }`，否则「只生成一个有界批次」
//! 可以被绕过。

use continuum_canonical::Opaque;
use continuum_semantics::ProductIdRef;

use crate::baseline::EngineeringBaseline;
use crate::error::{PortError, ProductError};
use crate::product::Product;
use crate::readiness::ProductReadiness;

/// §234 的七个字段。字段名逐字；字段私有。
///
/// 不派生 `PartialEq`：`Opaque` 没有 `PartialEq`。
#[derive(Debug, Clone)]
pub struct Milestone {
    id: MilestoneId,
    product_id: ProductIdRef,
    objective: Opaque,
    dependencies: Vec<MilestoneId>,
    entry_conditions: Vec<Opaque>,
    exit_conditions: Vec<Opaque>,
    state: MilestoneState,
}

impl Milestone {
    pub fn id(&self) -> &MilestoneId {
        &self.id
    }

    pub fn product_id(&self) -> &ProductIdRef {
        &self.product_id
    }

    pub fn objective(&self) -> &Opaque {
        &self.objective
    }

    pub fn dependencies(&self) -> &[MilestoneId] {
        &self.dependencies
    }

    pub fn entry_conditions(&self) -> &[Opaque] {
        &self.entry_conditions
    }

    pub fn exit_conditions(&self) -> &[Opaque] {
        &self.exit_conditions
    }

    pub fn state(&self) -> MilestoneState {
        self.state
    }

    /// crate 内唯一构造点：`propose_next_batch` 与 `persist::load_milestone(s)` 调用。
    pub(crate) fn from_verified_parts(
        id: MilestoneId,
        product_id: ProductIdRef,
        objective: Opaque,
        dependencies: Vec<MilestoneId>,
        entry_conditions: Vec<Opaque>,
        exit_conditions: Vec<Opaque>,
        state: MilestoneState,
    ) -> Result<Self, ProductError> {
        if id.as_str().is_empty() {
            return Err(ProductError::UnknownMilestone {
                id: id.as_str().to_owned(),
            });
        }
        if product_id.as_str().is_empty() {
            return Err(ProductError::MilestoneProductMismatch {
                milestone: id.as_str().to_owned(),
                product: String::new(),
            });
        }
        if objective.as_str().is_empty() {
            return Err(ProductError::EmptyMilestoneObjective);
        }
        for (field, conditions) in [
            ("entry_conditions", &entry_conditions),
            ("exit_conditions", &exit_conditions),
        ] {
            for (index, condition) in conditions.iter().enumerate() {
                if condition.as_str().is_empty() {
                    return Err(ProductError::EmptyMilestoneCondition { field, index });
                }
            }
        }
        Ok(Self {
            id,
            product_id,
            objective,
            dependencies,
            entry_conditions,
            exit_conditions,
            state,
        })
    }
}

/// §234 的七个状态；编码用小写、多词下划线（与 P4 的枚举列纪律同形）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MilestoneState {
    Proposed,
    Approved,
    Active,
    Verifying,
    Completed,
    Failed,
    Superseded,
}

impl MilestoneState {
    /// 落库／错误消息用的规范名字。`pub(crate)`：编码与类型同址。
    #[allow(dead_code)]
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Proposed => "proposed",
            Self::Approved => "approved",
            Self::Active => "active",
            Self::Verifying => "verifying",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Superseded => "superseded",
        }
    }

    /// [`Self::as_str`] 的逆；认不出就是 `None`。
    #[allow(dead_code)]
    pub(crate) fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "proposed" => Self::Proposed,
            "approved" => Self::Approved,
            "active" => Self::Active,
            "verifying" => Self::Verifying,
            "completed" => Self::Completed,
            "failed" => Self::Failed,
            "superseded" => Self::Superseded,
            _ => return None,
        })
    }
}

/// P7A 的 Milestone 身份。不透明串。
///
/// `Milestone` 的 id 由 [`propose_next_batch`] 赋；`from_str` 只供读回与测试构造，
/// 不是生产入口。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MilestoneId(String);

impl MilestoneId {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 只供存储解码与测试构造。生产路径由 `propose_next_batch` 派生
    /// （`milestone/{product_id}/{ordinal}`，`ordinal = history.len() + i`）。
    pub fn from_str(s: &str) -> Self {
        Self(s.to_owned())
    }
}

/// 一个批次的上界。显式参数、无默认；`new` 拒绝 `max_milestones == 0`。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatchBoundary {
    max_milestones: usize,
    max_dependency_depth: u32,
}

impl BatchBoundary {
    /// `max_milestones == 0` ⇒ `InvalidBatchBoundary`；`max_dependency_depth` 可为 0
    /// （只允许批内无依赖的并行项）。没有默认边界。
    pub fn new(
        max_milestones: usize,
        max_dependency_depth: u32,
    ) -> Result<Self, ProductError> {
        if max_milestones == 0 {
            return Err(ProductError::InvalidBatchBoundary);
        }
        Ok(Self {
            max_milestones,
            max_dependency_depth,
        })
    }

    pub fn max_milestones(&self) -> usize {
        self.max_milestones
    }

    pub fn max_dependency_depth(&self) -> u32 {
        self.max_dependency_depth
    }
}

/// 生成端口的输入。`history` 只含**已生成**的 Milestone，不含未来。
///
/// gap analysis 必须经 `readiness.iter()`＋`ReadinessDimension::value()`／
/// `evidence()`／`last_verified()` 读取；生成器拿到的不是只有字段名的空壳。
pub struct MilestoneProposalInput<'a> {
    pub product_id: &'a ProductIdRef,
    pub baseline: &'a EngineeringBaseline,
    pub readiness: &'a ProductReadiness,
    pub history: &'a [Milestone],
    pub boundary: BatchBoundary,
}

/// 单枚提案：没有 id、没有 product_id、没有 state——这三样由 P7A 赋。
#[derive(Debug, Clone)]
pub struct MilestoneDraft {
    pub objective: Opaque,
    pub dependencies: Vec<MilestoneDependency>,
    pub entry_conditions: Vec<Opaque>,
    pub exit_conditions: Vec<Opaque>,
}

/// 依赖只能是「已生成的历史」或「本批次里更早的一枚」。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MilestoneDependency {
    Existing(MilestoneId),
    InBatch(usize),
}

#[derive(Debug, Clone)]
pub struct MilestoneBatchDraft {
    pub milestones: Vec<MilestoneDraft>,
}

#[async_trait::async_trait]
pub trait MilestoneGenerator {
    async fn propose(
        &self,
        input: &MilestoneProposalInput<'_>,
    ) -> Result<MilestoneBatchDraft, PortError>;
}

/// 生成一个有界批次。校验顺序：
///
/// 1. 批次为空 ⇒ `EmptyBatch`；
/// 2. `len > max_milestones` ⇒ `BatchTooLarge`；
/// 3. `history` 里每枚的 product_id 必须等于 `product.id`；
/// 4. objective 非空；
/// 5. `entry_conditions`／`exit_conditions` 元素非空；
/// 6. `Existing(id)` 必须在 history；`InBatch(i)` 必须 `i < 当前 index`；
/// 7. 批内依赖深度 `<= max_dependency_depth`（`Existing` 不计入）；
/// 8. 赋 `milestone/{product_id}/{ordinal}`（`ordinal = history.len() + i`）与
///    `state = Proposed`。
///
/// **不截断**：任何错误都在构造 `Vec<Milestone>` 之前返回。
///
/// `history` 必须由调用方按 `seq` 升序传入（`load_milestones` 的返回序）；本函数
/// 不校验顺序，也不为错序新增错误臂。
pub async fn propose_next_batch(
    product: &Product,
    history: &[Milestone],
    boundary: BatchBoundary,
    generator: &dyn MilestoneGenerator,
) -> Result<Vec<Milestone>, ProductError> {
    let input = MilestoneProposalInput {
        product_id: product.id(),
        baseline: product.baseline(),
        readiness: product.readiness(),
        history,
        boundary,
    };
    let draft = generator.propose(&input).await?;
    let drafts = draft.milestones;

    if drafts.is_empty() {
        return Err(ProductError::EmptyBatch);
    }
    if drafts.len() > boundary.max_milestones() {
        return Err(ProductError::BatchTooLarge {
            limit: boundary.max_milestones(),
            actual: drafts.len(),
        });
    }

    for milestone in history {
        if milestone.product_id().as_str() != product.id().as_str() {
            return Err(ProductError::MilestoneProductMismatch {
                milestone: milestone.id().as_str().to_owned(),
                product: product.id().as_str().to_owned(),
            });
        }
    }

    for draft in &drafts {
        if draft.objective.as_str().is_empty() {
            return Err(ProductError::EmptyMilestoneObjective);
        }
        for (field, conditions) in [
            ("entry_conditions", &draft.entry_conditions),
            ("exit_conditions", &draft.exit_conditions),
        ] {
            for (index, condition) in conditions.iter().enumerate() {
                if condition.as_str().is_empty() {
                    return Err(ProductError::EmptyMilestoneCondition { field, index });
                }
            }
        }
    }

    let ordinals = history.len();
    let mut ids = Vec::with_capacity(drafts.len());
    for i in 0..drafts.len() {
        ids.push(MilestoneId::from_str(&format!(
            "milestone/{}/{ordinals_i}",
            product.id().as_str(),
            ordinals_i = ordinals + i
        )));
    }

    let mut depths = vec![0_u32; drafts.len()];
    for (i, draft) in drafts.iter().enumerate() {
        for dependency in &draft.dependencies {
            match dependency {
                MilestoneDependency::Existing(id) => {
                    if !history.iter().any(|m| m.id() == id) {
                        return Err(ProductError::UnknownMilestone {
                            id: id.as_str().to_owned(),
                        });
                    }
                }
                MilestoneDependency::InBatch(j) => {
                    if *j >= i {
                        return Err(ProductError::DependencyOutOfRange { index: *j });
                    }
                    depths[i] = depths[i].max(depths[*j] + 1);
                }
            }
        }
    }

    let actual_depth = depths.iter().copied().max().unwrap_or(0);
    if actual_depth > boundary.max_dependency_depth() {
        return Err(ProductError::DependencyDepthExceeded {
            limit: boundary.max_dependency_depth(),
            actual: actual_depth,
        });
    }

    drafts
        .into_iter()
        .enumerate()
        .map(|(i, draft)| {
            let dependencies = draft
                .dependencies
                .iter()
                .map(|dependency| match dependency {
                    MilestoneDependency::Existing(id) => id.clone(),
                    MilestoneDependency::InBatch(j) => ids[*j].clone(),
                })
                .collect();
            Milestone::from_verified_parts(
                ids[i].clone(),
                product.id().clone(),
                draft.objective,
                dependencies,
                draft.entry_conditions,
                draft.exit_conditions,
                MilestoneState::Proposed,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const STATES: [MilestoneState; 7] = [
        MilestoneState::Proposed,
        MilestoneState::Approved,
        MilestoneState::Active,
        MilestoneState::Verifying,
        MilestoneState::Completed,
        MilestoneState::Failed,
        MilestoneState::Superseded,
    ];

    #[test]
    fn the_seven_states_round_trip_through_as_str_and_parse() {
        for state in STATES {
            assert_eq!(MilestoneState::parse(state.as_str()), Some(state));
        }
        assert_eq!(MilestoneState::parse("unknown"), None);
    }
}
