//! 五类观察（设计 §5.2）。

/// §216 的五个观察类。`as_str` 返回规范里的中文原句。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObservationKind {
    PlanRework,
    ModelSkillFailure,
    VerificationMiss,
    ExecutionMethodReliability,
    ContextSelectionNoise,
}

impl ObservationKind {
    /// 全部五枚。
    pub const ALL: [ObservationKind; 5] = [
        ObservationKind::PlanRework,
        ObservationKind::ModelSkillFailure,
        ObservationKind::VerificationMiss,
        ObservationKind::ExecutionMethodReliability,
        ObservationKind::ContextSelectionNoise,
    ];

    /// §216 中文原句，逐字（含空格与大小写）。`match` 穷尽且无通配臂。
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PlanRework => "哪些计划经常返工",
            Self::ModelSkillFailure => "哪些模型在哪些 Skill 上失败",
            Self::VerificationMiss => "哪些验证方式经常漏错",
            Self::ExecutionMethodReliability => "哪些 Execution Method 更可靠",
            Self::ContextSelectionNoise => "哪些 Context 选择导致噪声",
        }
    }

    /// [`ObservationKind::as_str`] 的严格逆：表外串一律 `None`。
    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|x| x.as_str() == s)
    }
}
