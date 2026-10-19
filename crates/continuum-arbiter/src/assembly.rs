use crate::arbiter::ResourceArbiter;
use crate::declaration::WorkloadDeclaration;
use crate::summary::ResourceSummary;

pub use crate::priority::PriorityTier;

/// 只对装配方（`ResourceDirectivePort` 的实现）可见的结论面：
/// summary 两项＋§300 四档。**不进 AI／后台的 `ResourceSummary`／`ResourceDecision`。**
/// 默认构建不可命名；装配构建下不承诺跨 crate 强制（设计 §6.5、J7）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssemblyView {
    summary: ResourceSummary,
    priority: PriorityTier,
}

impl AssemblyView {
    pub fn summary(&self) -> ResourceSummary {
        self.summary
    }

    pub fn priority(&self) -> PriorityTier {
        self.priority
    }
}

impl ResourceArbiter {
    pub fn assembly_view(&self, declaration: WorkloadDeclaration) -> AssemblyView {
        AssemblyView {
            summary: self.summary(),
            priority: crate::priority::classify(declaration),
        }
    }
}
