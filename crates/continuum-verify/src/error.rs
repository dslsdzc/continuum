//! 本 crate 的错误类型：`VerifyError`（设计 §2.2 的冻结清单）。变体逐 task 增量加。

use continuum_graph::{GraphId, NodeId};
use continuum_persist::PersistError;

use crate::evidence::EvidenceId;
use crate::evidence_type::EvidenceType;
use crate::test_validity::TestValidityCheck;
use crate::verdict::VerdictAggregate;

/// 本 crate 的失败面。变体逐 task 增量加（Task 2 的两枚是 §4.4 的两条；Task 3 追加
/// `IncompleteTestValidityChecks`；Task 4 追加 `UnknownVerificationPolicy` 与
/// `ProfileMissing`；后续 task 的变体随各自的产生方落地；Task 9 追加 `Persist` /
/// `MissingEvidenceEdge` / `CounterexampleWithPassingAggregate`——持久层失败、
/// §10.2 第 3 条的边核对与 §9.1 第 3 条的消费规则）。
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

    /// `VerificationPolicy::from_value` 拿到的值非空但不认识（§5.1 第 2 条）：缺字段、
    /// 多出未定义字段、`min_verifier_level` 不是五级序里的值、`required_evidence_types`
    /// 含未知类型名。取 `Err` 而不是取默认：一个写着内容的策略被当成空策略，正是
    /// 「静默放宽」。
    #[error(
        "verification_policy 非空但不认识（§5.1 第 2 条）：缺字段、多出未定义字段、min_verifier_level 不在五级序内或 required_evidence_types 含未知类型名"
    )]
    UnknownVerificationPolicy,

    /// 无验证档案（§5.3 第 3 条）：**不是「无要求」**——把它当默认会让每项 REQUIRED
    /// 的 required 集合为空、覆盖判定无缺项，即「无档案 ⇒ 无要求 ⇒ 不阻断」（fail-open）。
    #[error("缺 VerificationProfile：无档案是错，不是「无要求」（§5.3 第 3 条）")]
    ProfileMissing,

    /// §7.2 第 2 条：`min_verifier_level` 有值时下低于它的候选；**一个不剩即本错**。
    ///
    /// **不静默降级**（§7.2 第 4 条）：`A5`/`OPEN-004` 未定义降级路径，替它选一条
    /// 就是替规范做决定。降级只有两个合法来路（`independent_verifier_required` 显式给了假，
    /// 或用户经 `Decision` 授权），两条都不在本块。
    #[error(
        "没有可用的 verifier 候选：按 §7.2 第 2 条过滤后一个不剩（候选集为空，或全在 min_verifier_level 之下）"
    )]
    NoAvailableVerifier,

    /// §7.2 第 3 条 + §7.3：`independent_verifier_required == true` 时下不独立的候选；
    /// **身份含 `None` 的候选（`NotEstablished`）同样被下**（fail-closed）；**一个不剩即本错**。
    ///
    /// 同 [`VerifyError::NoAvailableVerifier`]：不静默降级（§7.2 第 4 条）。
    #[error(
        "没有独立的 verifier 候选：按 §7.2 第 3 条过滤（不独立或独立性未确立）后一个不剩"
    )]
    NoIndependentVerifier,

    /// §7.4 第 3 条：受验节点自己产的 `Evidence` 进不了初判输入。
    ///
    /// **比 §263 的文本更严**（§263 只点名「完整 Worker reasoning」与「Worker 自己的
    /// 完成声明」，没说受验节点自产的证据要排除）——设计把这一条作为 fail-closed 的
    /// 收紧记了代价：允许自产证据进来等于让受验者自证，§263 的隔离在同一节点内失效。
    /// 载荷带证据 id 与受验节点：只报「自产」而不给是哪条、谁产的，调用方无法定位。
    #[error(
        "证据 {} 由受验节点 {} 自己产出：受验者不得自证（§7.4 第 3 条）",
        evidence_id.as_str(),
        node
    )]
    EvidenceFromVerifiedNode {
        evidence_id: EvidenceId,
        node: NodeId,
    },

    /// §10.2 第 3 条的边核对：写 `evidence` 行时图上没有
    /// `producer_node → verified_node` 的 `EdgeKind::Evidence` 边。
    ///
    /// **只在 `Node` 一支产生**：`Human` 的证据没有产生者节点（§4.3.3），其
    /// `producer_node` 写 `NULL`、不核对——不变量与那一支不能同真（本 task 定的形状，
    /// 申报见 `persist.rs` 的文件头与计划 `## 遗留` 第一节第 7 条）。
    /// 载荷四枚（证据、图、两端）：只报「缺边」而不给这些，调用方无法定位。
    #[error(
        "图 {graph_id} 上没有 {producer_node} → {verified_node} 的 EdgeKind::Evidence 边，证据 {} 不得落库（§10.2 第 3 条）",
        evidence_id.as_str()
    )]
    MissingEvidenceEdge {
        evidence_id: EvidenceId,
        graph_id: GraphId,
        producer_node: NodeId,
        verified_node: NodeId,
    },

    /// §9.1 第 3 条的消费规则：`CounterexampleFound` 时该轮聚合**不得**取
    /// `Passed` / `PassedWithUnknown`（§215 的「必须增强验证体系」落在这一条上）。
    ///
    /// 由 `record_verification_round` 在写行前施加；**只拒绝那一对组合、不改写聚合值**
    /// （§9.1 第 4 条：本轮它不是判定反转，唯一的可见形态是 finding 记录 ＋
    /// `strategy_verdict` 的标注）。载荷两枚（轮次与聚合值）供调用方定位与决策。
    #[error(
        "第 {round_id} 轮找到反例（strategy_verdict = counterexample_found），而聚合为 {aggregate}，不得落库（§9.1 第 3 条）",
        aggregate = aggregate.as_str()
    )]
    CounterexampleWithPassingAggregate {
        round_id: String,
        aggregate: VerdictAggregate,
    },

    /// `Tx` 的往返失败（`#[from]` 转出）：建表/写行/写事件/查询的库层错误。
    ///
    /// **不吞成上面几枚**：上面各枚是「判定为错」，这一枚是「库层失败」，调用方的
    /// 处置不同（改动作／改数据），同 P4 把判定错误与 `Persist` 分在两处的理由。
    /// `evidence_id` 撞主键（§10.2 第 1 条的二次写）也走这一枚——`PersistError`
    /// 没有独立的冲突变体（同 P4 的 `ContractError::Persist` 的一段说明）。
    #[error("持久化失败：{0}")]
    Persist(#[from] PersistError),
}
