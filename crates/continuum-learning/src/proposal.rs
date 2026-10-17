//! 一条学习产物：绑定核心指纹的提案（设计 §5.3、§5.4）。

use continuum_protected::{CoreFingerprint, ProtectedCore};

use crate::target::LearningTarget;

/// 一条学习产物。字段私有：`core_fingerprint` 与 `target` 的搭配是构造点维持的
/// 不变量。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LearningProposal {
    target: LearningTarget,
    subject: String,
    rationale: String,
    core_fingerprint: CoreFingerprint,
}

impl LearningProposal {
    /// **唯一构造点**。`core` 只读：本函数不改变它，也不保存它；指纹由它读出。
    pub fn new(
        core: &ProtectedCore,
        target: LearningTarget,
        subject: impl Into<String>,
        rationale: impl Into<String>,
    ) -> Self {
        Self {
            target,
            subject: subject.into(),
            rationale: rationale.into(),
            core_fingerprint: core.fingerprint(),
        }
    }

    pub fn target(&self) -> LearningTarget {
        self.target
    }

    pub fn subject(&self) -> &str {
        &self.subject
    }

    pub fn rationale(&self) -> &str {
        &self.rationale
    }

    pub fn core_fingerprint(&self) -> &CoreFingerprint {
        &self.core_fingerprint
    }

    /// 这条提案是不是在**当前**这枚核心上学的。核心一变，全部旧提案过期。
    pub fn is_current_for(&self, core: &ProtectedCore) -> bool {
        self.core_fingerprint == core.fingerprint()
    }
}
