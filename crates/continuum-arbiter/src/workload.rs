/// §13 的四档进程类（设计 §4.3）。**闭集四枚；没有 INTERACTIVE**。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkloadClass {
    InteractiveGame,
    ComputeWorkload,
    Compiler,
    VideoRender,
}

impl WorkloadClass {
    pub const ALL: [WorkloadClass; 4] = [
        WorkloadClass::InteractiveGame,
        WorkloadClass::ComputeWorkload,
        WorkloadClass::Compiler,
        WorkloadClass::VideoRender,
    ];

    /// 逐字的大写文本；解析按同一张表精确匹配。
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InteractiveGame => "INTERACTIVE_GAME",
            Self::ComputeWorkload => "COMPUTE_WORKLOAD",
            Self::Compiler => "COMPILER",
            Self::VideoRender => "VIDEO_RENDER",
        }
    }

    /// 精确大写解析；`INTERACTIVE` 是规范的笔误，不映射到任何一档（设计 §4.3）。
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "INTERACTIVE_GAME" => Some(Self::InteractiveGame),
            "COMPUTE_WORKLOAD" => Some(Self::ComputeWorkload),
            "COMPILER" => Some(Self::Compiler),
            "VIDEO_RENDER" => Some(Self::VideoRender),
            _ => None,
        }
    }
}
