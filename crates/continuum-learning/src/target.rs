//! 六类可学习目标（设计 §5.1）。

/// §320 的五类加 §216 的 Scheduling Heuristic。闭枚举，`ALL` 六枚。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LearningTarget {
    RoutingPolicy,
    PlanningHeuristics,
    ExecutionMethod,
    VerificationStrategy,
    ContextSelection,
    SchedulingHeuristic,
}

impl LearningTarget {
    /// 全部六枚。
    pub const ALL: [LearningTarget; 6] = [
        LearningTarget::RoutingPolicy,
        LearningTarget::PlanningHeuristics,
        LearningTarget::ExecutionMethod,
        LearningTarget::VerificationStrategy,
        LearningTarget::ContextSelection,
        LearningTarget::SchedulingHeuristic,
    ];

    /// 小写原词（设计 §2.2）。`match` 穷尽且无通配臂。
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RoutingPolicy => "routing policy",
            Self::PlanningHeuristics => "planning heuristics",
            Self::ExecutionMethod => "execution method",
            Self::VerificationStrategy => "verification strategy",
            Self::ContextSelection => "context selection",
            Self::SchedulingHeuristic => "scheduling heuristic",
        }
    }

    /// [`LearningTarget::as_str`] 的严格逆：表外串一律 `None`。
    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|x| x.as_str() == s)
    }
}
