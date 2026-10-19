use crate::declaration::WorkloadDeclaration;

/// §300 的四档优先级。**定义在私有模块 `priority`**；默认构建的公开面没有它。
/// 只有 `#[cfg(feature = "assembly")]` 的 `assembly` 模块把它 re-export 给装配方。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriorityTier {
    UserWorkload,
    ForegroundRuntime,
    BackgroundRuntime,
    Maintenance,
}

impl PriorityTier {
    /// 逐字 §300 的名字（含空格）。
    #[cfg(feature = "assembly")]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::UserWorkload => "User workload",
            Self::ForegroundRuntime => "Foreground Runtime",
            Self::BackgroundRuntime => "Background Runtime",
            Self::Maintenance => "Maintenance",
        }
    }

    /// 展示用的序号；内部动作阶梯不依赖它（设计 §4.6）。
    #[cfg(feature = "assembly")]
    pub fn rank(&self) -> u8 {
        match self {
            Self::UserWorkload => 3,
            Self::ForegroundRuntime => 2,
            Self::BackgroundRuntime => 1,
            Self::Maintenance => 0,
        }
    }
}

/// 唯一的分类函数；`assembly_view` 与将来的 `direct` 共用这一份（设计 §5.3）。
pub(crate) fn classify(declaration: WorkloadDeclaration) -> PriorityTier {
    if declaration.maintenance {
        PriorityTier::Maintenance
    } else if declaration.user_process {
        PriorityTier::UserWorkload
    } else if declaration.foreground {
        PriorityTier::ForegroundRuntime
    } else {
        PriorityTier::BackgroundRuntime
    }
}
