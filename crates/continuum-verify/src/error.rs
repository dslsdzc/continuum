//! 本 crate 的错误类型：`VerifyError`（设计 §2.2 的冻结清单）。变体逐 task 增量加。

use crate::evidence_type::EvidenceType;
use crate::test_validity::TestValidityCheck;

/// 本 crate 的失败面。变体逐 task 增量加（Task 2 的两枚是 §4.4 的两条；Task 3 追加
/// `IncompleteTestValidityChecks`；后续 task 的变体随各自的产生方落地）。
///
/// **派生 `PartialEq`**：失败路径的用例要断言「是哪一种 `Err`」（本计划的纪律第 4 条），
/// 而 `EvidenceWithoutArtifact` 的十三型逐型断言还要求载荷里的 `EvidenceType` 可比较
/// （它已派生 `PartialEq`）。
#[derive(Debug, PartialEq, thiserror::Error)]
pub enum VerifyError {
    /// `evidence_type` 在读制品的十三型内而 `artifact_refs` 为空（§4.4 第 1 条）。
    ///
    /// `evidence_type` 随错误带回：只报「缺制品」而不给类型，调用方无法判断该补一件
    /// 制品还是该改类型。
    #[error("证据类型 {} 要读制品，而 artifact_refs 为空（§4.4 第 1 条）", evidence_type.as_str())]
    EvidenceWithoutArtifact { evidence_type: EvidenceType },

    /// §258 的 `HUMAN_CONFIRMATION` 与生产者不匹配（§4.4 第 2 条）：
    /// 人确认的证据必须由 [`EvidenceProducer::Human`](crate::evidence::EvidenceProducer::Human)
    /// 给出。放过去会让「谁确认的」不可判。
    #[error("HumanConfirmation 的生产者不是 Human（§4.4 第 2 条）")]
    HumanConfirmationFromNonHuman,

    /// `TestValidity::new` 的 `checks` 未做到「五臂齐且不重复」（§6.4）。
    ///
    /// 缺臂与重复臂都随错误带回：只报「不齐」而不给是哪一臂，调用方要么重算要么去猜；
    /// 而「重复补齐五条」正是要拦的那种输入（重复臂把缺席的臂静默填平），故两个名单分开给。
    /// 两个名单都按 [`TestValidityCheck::ALL`] 的顺序排列。
    #[error(
        "TestValidity 的 checks 未做到五臂齐且不重复（§6.4）：缺臂 {missing:?}、重复臂 {repeated:?}"
    )]
    IncompleteTestValidityChecks {
        missing: Vec<TestValidityCheck>,
        repeated: Vec<TestValidityCheck>,
    },
}
