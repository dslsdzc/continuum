//! Verifier 的选择、隔离与两个输入面（设计 §7.2–§7.5；计划 Task 6 的 Step 1）。
//!
//! 三条口径（照计划的测试纪律）：
//! - 失败路径的用例断言**是哪一种** `Err`（`NoAvailableVerifier` / `NoIndependentVerifier` /
//!   `EvidenceFromVerifiedNode`），不只「返回了 `Err`」；
//! - 「守卫」两侧成对：选择规则的两条 `Err` 与它们的对照正例成对、
//!   `an_unknown_identity_is_not_established` 是 fail-closed 侧的另一侧、
//!   `evidence_from_another_node_is_accepted` 是 (m) 的另一侧、(o) 的「有物可读」一侧；
//! - 标「编译期照片」的地方**不是运行期红**——判据是「编译不过」或「从编译失败变成
//!   编译通过」，不假称它们会跑红（纪律 1(c)：「编译失败不是变红」）。
//!
//! **跨 task 的同一处变异**：`an_unknown_identity_is_not_established` 与 Task 7 的 (s)
//! 第一条钉的是同一处判据（`NotEstablished` 不得写成 `NotIndependent`）。

use continuum_artifact::{Artifact, ArtifactId, ArtifactType, ContentHash, PrivacyClass};
use continuum_graph::NodeId;
use continuum_operator::BackendId;
use continuum_verify::{
    AvailableVerifier, BlindVerdict, Claim, ContractView, Evidence, EvidenceId, EvidenceProducer,
    EvidenceScope, EvidenceStrength, EvidenceSubject, EvidenceType, IndependenceVerdict,
    InitialVerifierInput, VerdictAggregate, VerificationPolicy, VerificationProfile,
    VerifierLevel, VerifierVerdict, VerifyError, WorkerIdentity, clamp_by_readability,
    independence_of, select_verifier,
};
use serde_json::json;

/// 一个候选：`level` 与 `backend` 给定，身份两项默认 `None`
/// （P1 设计 §14：`model` / `provider` 在 P3 之前恒为 `None`）。
fn candidate(level: VerifierLevel, backend: &str) -> AvailableVerifier {
    AvailableVerifier {
        level,
        backend: BackendId::new(backend),
        model: None,
        provider: None,
    }
}

/// 一个候选，三项身份齐备。
fn named_candidate(
    level: VerifierLevel,
    backend: &str,
    model: &str,
    provider: &str,
) -> AvailableVerifier {
    AvailableVerifier {
        level,
        backend: BackendId::new(backend),
        model: Some(model.to_owned()),
        provider: Some(provider.to_owned()),
    }
}

/// Worker 身份，三项齐备（§7.2 的 `WorkerIdentity`）。
fn named_worker(backend: &str, model: &str, provider: &str) -> WorkerIdentity {
    WorkerIdentity {
        model: Some(model.to_owned()),
        provider: Some(provider.to_owned()),
        backend: Some(BackendId::new(backend)),
    }
}

/// Worker 身份，三项全未知（今天 `ExecutionProfile` 的常态）。
fn unknown_worker() -> WorkerIdentity {
    WorkerIdentity {
        model: None,
        provider: None,
        backend: None,
    }
}

/// 无下限的节点级策略。
fn no_floor_policy() -> VerificationPolicy {
    VerificationPolicy::default()
}

/// 任务级档案，只有 `independent_verifier_required` 可变（其余字段只存不判，给空值）。
fn profile_requiring_independence(required: bool) -> VerificationProfile {
    VerificationProfile {
        required_evidence_types: Vec::new(),
        optional_evidence_types: Vec::new(),
        fuzz_budget: None,
        mutation_threshold: None,
        property_requirements: Vec::new(),
        independent_verifier_required: required,
    }
}

/// 理由文本（本 crate 不解析 `Claim`）。
fn claim(s: &str) -> Claim {
    Claim::from_str(s)
}

/// 一条节点产的 `Test` 证据（`Test` 在「要读制品」的十三型内，故带一件制品引用）。
fn node_evidence(id: &str, node: &str, backend: &str) -> Evidence {
    Evidence::from_tool_result(
        EvidenceId::from_str(id),
        EvidenceType::Test,
        EvidenceSubject::Unattached,
        claim("断言文本（本 crate 不解析）"),
        EvidenceProducer::Node {
            node: NodeId::new(node),
            backend: BackendId::new(backend),
        },
        vec![ArtifactId::new("artifact-1")],
        EvidenceStrength::from_value(json!({ "level": 1 })),
        EvidenceScope::from_value(json!({ "covers": "全部" })),
    )
    .expect("夹具证据应能构造（Test 型带一件制品引用）")
}

/// 一件制品（字段值不重要；`has_readable_artifact` 只判有没有）。
fn an_artifact(id: &str) -> Artifact {
    Artifact {
        id: ArtifactId::new(id),
        artifact_type: ArtifactType::Text,
        content_hash: ContentHash::of(b"artifact-content"),
        size: 16,
        producer_node: None,
        input_artifacts: Vec::new(),
        metadata: json!({}),
        provenance: json!({}),
        privacy_class: PrivacyClass::Public,
        version: 1,
    }
}

/// 空视图（输入面三类的第一类）。
fn an_contract_view() -> ContractView {
    ContractView {
        requirements: Vec::new(),
    }
}

/// §7.2 第 1 条：取 `level` 最高（`Ord` 最小）的可用候选。
///
/// 红条件（**取反档**）：实现取最低（例如把比较方向反过来 ⇒ 取 `VerifierLevel::HumanReview`）
/// ⇒ 本条全红。
///
/// **另一侧**：只有 3、5 级两个候选 ⇒ 选 3 级（没有这一半，本条可由「恒取第 1 级」满足）。
#[test]
fn the_highest_available_level_wins() {
    let worker = named_worker("worker-backend", "worker-model", "worker-provider");

    let available = vec![
        candidate(VerifierLevel::DeterministicChecker, "backend-1"),
        candidate(VerifierLevel::IndependentModelVerifier, "backend-3"),
        candidate(VerifierLevel::HumanReview, "backend-5"),
    ];
    let selection = select_verifier(
        &available,
        &no_floor_policy(),
        &profile_requiring_independence(false),
        &worker,
    )
    .expect("三个候选都可用：选择应成功（§7.2 第 1 条）");
    assert_eq!(
        selection.level,
        VerifierLevel::DeterministicChecker,
        "§262 第 1 级在场时应选它——可确定性验证优先（§7.2 第 1 条）"
    );
    assert_eq!(
        selection.backend,
        BackendId::new("backend-1"),
        "选中的 backend 应是第 1 级那个候选的 backend"
    );

    // 另一侧：没有第 1 级时不得仍「恒取第 1 级」，而应取在场者里最高的一级。
    let available = vec![
        candidate(VerifierLevel::IndependentModelVerifier, "backend-3"),
        candidate(VerifierLevel::HumanReview, "backend-5"),
    ];
    let selection = select_verifier(
        &available,
        &no_floor_policy(),
        &profile_requiring_independence(false),
        &worker,
    )
    .expect("两个候选都可用：选择应成功（§7.2 第 1 条）");
    assert_eq!(
        selection.level,
        VerifierLevel::IndependentModelVerifier,
        "第 1 级不在场时应取在地最高的一级（第 3 级）"
    );
}

/// §7.2 第 2 条：`min_verifier_level` 有值时下低于它的候选；**一个不剩即 `Err`**
/// （(l) 的第一子例）。
///
/// 红条件（**移除档**）：不应用下限 ⇒ 第一条红。
///
/// **另一侧**：2 级与 5 级各一 ⇒ 选中 2 级（下限不误伤在场的可接受级）。
#[test]
fn a_candidate_below_the_policy_floor_is_dropped() {
    let policy = VerificationPolicy {
        min_verifier_level: Some(VerifierLevel::FormalStaticChecker),
        required_evidence_types: Vec::new(),
    };

    // 只有 5 级候选：它低于下限（第 2 级）⇒ 一个不剩 ⇒ Err(NoAvailableVerifier)。
    let available = vec![candidate(VerifierLevel::HumanReview, "backend-5")];
    assert_eq!(
        select_verifier(
            &available,
            &policy,
            &profile_requiring_independence(false),
            &unknown_worker()
        ),
        Err(VerifyError::NoAvailableVerifier),
        "唯一候选低于下限 ⇒ Err(NoAvailableVerifier)——不可用即 Err，不静默降级（§7.2 第 4 条）"
    );

    // 另一侧：2 级（恰在下限上）保留、5 级（低于下限）丢弃 ⇒ 选中 2 级。
    let available = vec![
        candidate(VerifierLevel::FormalStaticChecker, "backend-2"),
        candidate(VerifierLevel::HumanReview, "backend-5"),
    ];
    let selection = select_verifier(
        &available,
        &policy,
        &profile_requiring_independence(false),
        &unknown_worker(),
    )
    .expect("下限上的候选应保留（§7.2 第 2 条）");
    assert_eq!(
        selection.level,
        VerifierLevel::FormalStaticChecker,
        "下限不下低于下限的候选（恰在下限上的保留）"
    );
}

/// §7.2 第 3 条 + §7.3：`independent_verifier_required == true` 时下不独立的候选；
/// **一个不剩即 `Err(NoIndependentVerifier)`**（(l) 的第二子例）。
///
/// 红条件（**放宽档**）：降级到那个不独立的候选（返回 `Ok`）⇒ 第一条红。
///
/// **另一侧**：候选与 Worker 的 `backend` 不同、`model` 与 `provider` 均已知且不同 ⇒ `Ok`。
///
/// **这两条合起来钉的是「不可用时返回 `Err`，不静默降级」**（§7.2 第 4 条：
/// `A5`/`OPEN-004` 未定义降级路径）。
#[test]
fn no_independent_candidate_is_an_error_when_independence_is_required() {
    // 唯一候选与 Worker 同 backend ⇒ 不独立 ⇒ 一个不剩。
    let available = vec![candidate(
        VerifierLevel::IndependentModelVerifier,
        "shared-backend",
    )];
    let worker = named_worker("shared-backend", "worker-model", "worker-provider");
    assert_eq!(
        select_verifier(
            &available,
            &no_floor_policy(),
            &profile_requiring_independence(true),
            &worker
        ),
        Err(VerifyError::NoIndependentVerifier),
        "要求独立而唯一候选与 Worker 同 backend ⇒ Err(NoIndependentVerifier)，不降级（§7.2 第 4 条）"
    );

    // 另一侧：backend 不同、model 与 provider 均已知且都不同 ⇒ 独立 ⇒ Ok。
    let available = vec![named_candidate(
        VerifierLevel::IndependentModelVerifier,
        "verifier-backend",
        "verifier-model",
        "verifier-provider",
    )];
    let selection = select_verifier(
        &available,
        &no_floor_policy(),
        &profile_requiring_independence(true),
        &worker,
    )
    .expect("三项都不同 ⇒ 独立 ⇒ 候选保留（§7.3）");
    assert_eq!(
        selection.independence,
        IndependenceVerdict::Independent,
        "三项都不同的候选应标注独立"
    );
    assert_eq!(
        selection.backend,
        BackendId::new("verifier-backend"),
        "选中的应是那个独立候选"
    );
}

/// §7.3 判据一：同 `backend` ⇒ `NotIndependent`（同一条执行路径不可能独立于自己）。
///
/// 红条件（**取反档**）：删掉同 backend 那一支（或改比较对象）⇒ 本条红。
#[test]
fn a_same_backend_candidate_is_not_independent() {
    let candidate = candidate(VerifierLevel::DeterministicChecker, "same-backend");
    let worker = WorkerIdentity {
        model: None,
        provider: None,
        backend: Some(BackendId::new("same-backend")),
    };
    assert_eq!(
        independence_of(&candidate, &worker),
        IndependenceVerdict::NotIndependent,
        "同 backend 即同一条执行路径 ⇒ 不独立（§7.3 判据一）"
    );
}

/// §7.3 判据二：`model` 与 `provider` 均已知且两项都相同 ⇒ `NotIndependent`。
///
/// 红条件（**取反档**）：把「两项都相同」写成「任一项相同」⇒ 本条仍红（本条两项都同）；
/// 把该支写成恒不命中 ⇒ 本条红。
#[test]
fn same_model_and_provider_is_not_independent() {
    let candidate = named_candidate(
        VerifierLevel::IndependentModelVerifier,
        "verifier-backend",
        "same-model",
        "same-provider",
    );
    let worker = named_worker("worker-backend", "same-model", "same-provider");
    assert_eq!(
        independence_of(&candidate, &worker),
        IndependenceVerdict::NotIndependent,
        "backend 不同但 model 与 provider 均已知且都相同 ⇒ 不独立（§7.3 判据二）"
    );
}

/// §7.3 判据三（**另一侧**）：任一项为 `None`、且 `independent_verifier_required == false`
/// ⇒ 候选**保留**且 `selection.independence == NotEstablished`（**不是 `NotIndependent`**）。
///
/// 红条件（**取反档**）：写成 `NotIndependent`（把「没判」写成「判为假」）⇒ 本条红，
/// **且 Task 7 的 (s) 第一条随之红**（这是跨 task 的同一处变异：
/// `NotEstablished` 被压成 `NotIndependent` 会让第 10 项从「不可判」变成「为假」）。
#[test]
fn an_unknown_identity_is_not_established() {
    let mut candidate = candidate(VerifierLevel::IndependentModelVerifier, "verifier-backend");
    candidate.model = Some("verifier-model".to_owned()); // provider 留 None：任一项为 None。
    let worker = named_worker("worker-backend", "worker-model", "worker-provider");

    let selection = select_verifier(
        &[candidate],
        &no_floor_policy(),
        &profile_requiring_independence(false),
        &worker,
    )
    .expect("不要求独立时，身份未知的候选保留（§7.3 判据三）");

    assert_eq!(
        selection.independence,
        IndependenceVerdict::NotEstablished,
        "身份任一项为 None ⇒ 判不出——标注「未确立」使它在审计里可见（§7.3）"
    );
    assert_ne!(
        selection.independence,
        IndependenceVerdict::NotIndependent,
        "「判不出」不是「判为不独立」——两者分开正是 §8.4 警告的那条"
    );
}

/// §7.3 的 fail-closed 侧：`required == true` ＋ 身份含 `None` ＋ 只剩这一个候选
/// ⇒ `Err(NoIndependentVerifier)`。
///
/// 红条件（**放宽档**）：把 `NotEstablished` 留在候选里（或降级给它）⇒ 本条红。
#[test]
fn an_unknown_identity_is_dropped_when_independence_is_required() {
    let mut candidate = candidate(VerifierLevel::IndependentModelVerifier, "verifier-backend");
    candidate.provider = Some("verifier-provider".to_owned()); // model 留 None：任一项为 None。
    let worker = named_worker("worker-backend", "worker-model", "worker-provider");

    assert_eq!(
        select_verifier(
            &[candidate],
            &no_floor_policy(),
            &profile_requiring_independence(true),
            &worker
        ),
        Err(VerifyError::NoIndependentVerifier),
        "要求独立时，身份未确立的候选被丢弃（fail-closed，§7.3 判据三）"
    );
}

/// (m)：`assemble` 的 `evidence` 里有一条 `producer` 指向受验节点自己
/// ⇒ `Err(VerifyError::EvidenceFromVerifiedNode { .. })`（§7.4 第 3 条）。
///
/// 红条件（**移除档**）：删掉该检查（自产证据放进来）⇒ 本条红。
#[test]
fn the_verified_nodes_own_evidence_is_rejected() {
    let verified = NodeId::new("verified-node");
    let own = node_evidence("ev-own", "verified-node", "backend-x");

    assert_eq!(
        InitialVerifierInput::assemble(
            an_contract_view(),
            vec![an_artifact("artifact-1")],
            vec![own],
            &verified,
            1_000,
        ),
        Err(VerifyError::EvidenceFromVerifiedNode {
            evidence_id: EvidenceId::from_str("ev-own"),
            node: verified,
        }),
        "受验节点自产的证据进不了初判输入——否则同一节点内 §263 的隔离失效（§7.4 第 3 条）"
    );
}

/// (n)（**另一侧**）：别的节点产的同类证据 ⇒ `Ok`，且 `input.evidence` 的长度
/// 等于传入条数。
///
/// 红条件（**取反档**）：把检查写成「一律 `Err`」（不看生产者）⇒ 本条红。
///
/// **前提写清**：本条不退化为「一律 `Err`」靠的是「图上有别的节点产同类证据」这个前提；
/// 无别的节点时按 `Err` 走（与本块其余 fail-closed 侧同向，不静默放行）。
#[test]
fn evidence_from_another_node_is_accepted() {
    let verified = NodeId::new("verified-node");
    let other = node_evidence("ev-other", "other-node", "backend-y");
    let human = Evidence::from_tool_result(
        EvidenceId::from_str("ev-human"),
        EvidenceType::HumanConfirmation,
        EvidenceSubject::Unattached,
        claim("人确认（§258 的 HUMAN_CONFIRMATION）"),
        EvidenceProducer::Human,
        Vec::new(),
        EvidenceStrength::from_value(json!({ "level": 1 })),
        EvidenceScope::from_value(json!({ "covers": "全部" })),
    )
    .expect("HumanConfirmation 不需要制品引用（§4.4 第 1 条）。");

    let input = InitialVerifierInput::assemble(
        an_contract_view(),
        vec![an_artifact("artifact-1")],
        vec![other, human],
        &verified,
        1_000,
    )
    .expect("别的节点与人的证据都应通过（§7.4 第 3 条的正例）");
    assert_eq!(
        input.evidence.len(),
        2,
        "输入面应原样收下传入的每条证据（不吞、不增）"
    );
}

/// §263 的初判输入面**按名穷尽解构**，**不带 `..`**。
///
/// **这是编译期照片，不是运行期红**：红条件（**移除档**）是给 `InitialVerifierInput`
/// 加第四类字段 ⇒ 本函数的解构**编译不过**。观测方式是「本段从编译通过变成编译不过」——
/// 本用例不会跑红（纪律 1(c)）。
///
/// **这条是 §263 与 §29 在本块的结构性兑现**：字段里没有任何一个装得下 Worker 的
/// 完成声明或完整推理（把 `TestValidity` 喂进来也没有位置——它是 Test Validity
/// Verifier 的产出，见 `InitialVerifierInput` 的文档）。
///
/// 各绑定都要被用掉：未用变量会让本块的门（0 warning）红了，故顺带让三类输入
/// 「存在且类型对」在运行期也各有一处读点。
#[test]
fn the_initial_input_names_exactly_the_three_inputs_of_263() {
    let verified = NodeId::new("verified-node");
    let input = InitialVerifierInput::assemble(
        an_contract_view(),
        vec![an_artifact("artifact-1")],
        vec![node_evidence("ev-other", "other-node", "backend-y")],
        &verified,
        1_000,
    )
    .expect("别的节点的证据应通过（§7.4 第 3 条的正例）");

    let InitialVerifierInput {
        contract,
        artifacts,
        evidence,
    } = input;

    assert_eq!(contract.requirements.len(), 0, "字段一：ContractView");
    assert_eq!(artifacts.len(), 1, "字段二：Artifact 列表");
    assert_eq!(evidence.len(), 1, "字段三：Evidence 列表");
}

/// (o)：level 1 的确定性检查需要一个可读的制品；**没有制品时不许给 `Pass`**
/// （§7.5 末段）。
///
/// 红条件（**移除档**）：`clamp_by_readability` 写成恒等 ⇒ 第一条红；
/// （**收紧档**）：写成恒 `Unknown` ⇒ 第二条红。**两侧都钉**。
///
/// **理由**：这是 §4.4 的构造点规则在判定侧的对应——「无物可读」必须走到 `Unknown`，
/// 不能走到 `Pass`。
#[test]
fn a_level_one_check_without_a_readable_artifact_is_unknown_not_pass() {
    assert!(
        matches!(
            clamp_by_readability(false, VerifierVerdict::Pass),
            VerifierVerdict::Unknown { .. }
        ),
        "无物可读 ⇒ Unknown，不得 Pass（§7.5 末段）"
    );

    // 另一侧：有可读制品时 Pass 保持——钳制不得把有物可读的通过也吞掉。
    assert_eq!(
        clamp_by_readability(true, VerifierVerdict::Pass),
        VerifierVerdict::Pass,
        "有可读制品时 Pass 保持（钳制只动「无物可读的 Pass」这一支）"
    );
}

/// (p)：同时存在 `Pass` 与 `Fail` ⇒ `AdjudicationRequired`
/// （**不是** `Failed`、**不是**多数票的 `Passed`）。
///
/// 红条件（**取反档**）：把 `AdjudicationRequired` 与 `Failed` 的次序互换、
/// 或把 `Failed` 写成「至少一条 `Fail`」⇒ 红。**这一档若不红，说明 p 只拍了单项 `Fail`**
/// （设计 §12.2 的预告）。
#[test]
fn pass_and_fail_together_require_adjudication() {
    let fail = || VerifierVerdict::Fail {
        reason: claim("反例或失败理由"),
    };
    assert_eq!(
        VerdictAggregate::from_verdicts(&[VerifierVerdict::Pass, fail()]),
        VerdictAggregate::AdjudicationRequired,
        "Pass 与 Fail 同时存在 ⇒ 需要裁决（§121：不得多数投票）"
    );

    // 多数票形态：Pass 占多数也不得判 Passed（§121 的「不得多数投票」在另一侧的直接照片）。
    assert_eq!(
        VerdictAggregate::from_verdicts(&[VerifierVerdict::Pass, VerifierVerdict::Pass, fail()]),
        VerdictAggregate::AdjudicationRequired,
        "Pass 占多数也不得判 Passed——「不得多数投票」"
    );
}

/// §7.5 的五臂，**逐臂一条**。
///
/// `[]` ⇒ `Unknown`：空集落在「无 `Fail`、无 `Pass`」那一臂；
/// **本计划取这条读法并在此写明**——空与「全是 `Unknown`」都落在 `Unknown` 臂，
/// 而**不许**落在 `Passed`（fail-closed 的一侧）。
///
/// 红条件（**取反档**）：改任一臂的判据 ⇒ 该条红。
#[test]
fn each_of_the_five_aggregate_arms_has_one_case() {
    let unknown = || VerifierVerdict::Unknown {
        reason: claim("判不出"),
    };

    assert_eq!(
        VerdictAggregate::from_verdicts(&[]),
        VerdictAggregate::Unknown,
        "空集落在「无 Fail、无 Pass」那一臂（本计划的读法），不许落 Passed"
    );
    assert_eq!(
        VerdictAggregate::from_verdicts(&[unknown()]),
        VerdictAggregate::Unknown,
        "全是 Unknown ⇒ Unknown"
    );
    assert_eq!(
        VerdictAggregate::from_verdicts(&[VerifierVerdict::Pass]),
        VerdictAggregate::Passed,
        "全部 Pass ⇒ Passed"
    );
    assert_eq!(
        VerdictAggregate::from_verdicts(&[VerifierVerdict::Pass, unknown()]),
        VerdictAggregate::PassedWithUnknown,
        "无 Fail、有 Pass 且有 Unknown ⇒ PassedWithUnknown"
    );
    assert_eq!(
        VerdictAggregate::from_verdicts(&[VerifierVerdict::Fail {
            reason: claim("失败"),
        }]),
        VerdictAggregate::Failed,
        "至少一条 Fail 且无 Pass ⇒ Failed"
    );
}

/// `BlindVerdict` 的三臂**逐条**由 `From<VerifierVerdict>` 给出。
///
/// **来历写明**：规范**未给** `BlindVerdict` 的臂（§29 只规定顺序，未规定结论的形态）；
/// 本设计取「与 `VerifierVerdict` 同形的三臂」，转换入口是 `BlindVerdict::from`——
/// 盲判即 Verifier 在看不到 Worker 完成声明与推理时的判定，本设计不为它新增判定语义。
///
/// 红条件（**取反档**）：`from` 里把某一臂映射错（例如 `Pass` 映成 `Unknown`）
/// ⇒ 该条红。
#[test]
fn a_blind_verdict_comes_from_a_verifier_verdict() {
    let reason = claim("理由文本（逐臂原样搬运）");

    assert_eq!(
        BlindVerdict::from(VerifierVerdict::Pass),
        BlindVerdict::Pass,
        "Pass 臂逐条同形"
    );
    assert_eq!(
        BlindVerdict::from(VerifierVerdict::Fail {
            reason: reason.clone()
        }),
        BlindVerdict::Fail {
            reason: reason.clone()
        },
        "Fail 臂逐条同形（载荷原样搬运）"
    );
    assert_eq!(
        BlindVerdict::from(VerifierVerdict::Unknown {
            reason: reason.clone()
        }),
        BlindVerdict::Unknown {
            reason: reason.clone()
        },
        "Unknown 臂逐条同形（载荷原样搬运）"
    );
}

/// 四枚枚举的臂数与逐臂点名；`VerdictAggregate` 与 `IndependenceVerdict`
/// 各做一次**编码往返**（它们要落列，见 Task 9）。
///
/// 红条件（**取反档**）：改某一枚的串（`as_str` 或 `parse` 里那一行）⇒ 该枚那一条红；
/// 删**该枚举的 `ALL` 常量数组**里的一臂（**移除档**）⇒ 数目断言红。
/// **「删一臂」的锚点只取 `ALL`**（纪律 1(a)）——删枚举定义里那一臂是用例与 `as_str`
/// 的编译失败，不是变红（纪律 1(c)）。
#[test]
fn the_four_enums_each_name_their_arms() {
    // 数目断言（每枚一条）。
    assert_eq!(
        VerifierVerdict::ALL.len(),
        3,
        "VerifierVerdict 的名单与臂数不符"
    );
    assert_eq!(
        VerdictAggregate::ALL.len(),
        5,
        "VerdictAggregate 的名单与臂数不符"
    );
    assert_eq!(
        IndependenceVerdict::ALL.len(),
        3,
        "IndependenceVerdict 的名单与臂数不符"
    );
    assert_eq!(
        BlindVerdict::ALL.len(),
        3,
        "BlindVerdict 的名单与臂数不符"
    );

    // 逐臂点名（四枚各一条，不抽代表）。
    assert_eq!(
        VerifierVerdict::ALL.as_slice(),
        [
            VerifierVerdict::Pass,
            VerifierVerdict::Fail {
                reason: claim("")
            },
            VerifierVerdict::Unknown {
                reason: claim("")
            },
        ]
        .as_slice(),
        "VerifierVerdict::ALL 的三臂与设计 §7.5 不符"
    );
    assert_eq!(
        VerdictAggregate::ALL.as_slice(),
        [
            VerdictAggregate::AdjudicationRequired,
            VerdictAggregate::Failed,
            VerdictAggregate::Unknown,
            VerdictAggregate::PassedWithUnknown,
            VerdictAggregate::Passed,
        ]
        .as_slice(),
        "VerdictAggregate::ALL 的五臂与设计 §7.5 不符（声明序即优先序）"
    );
    assert_eq!(
        IndependenceVerdict::ALL.as_slice(),
        [
            IndependenceVerdict::Independent,
            IndependenceVerdict::NotIndependent,
            IndependenceVerdict::NotEstablished,
        ]
        .as_slice(),
        "IndependenceVerdict::ALL 的三臂与设计 §7.3/§7.5 不符"
    );
    assert_eq!(
        BlindVerdict::ALL.as_slice(),
        [
            BlindVerdict::Pass,
            BlindVerdict::Fail {
                reason: claim("")
            },
            BlindVerdict::Unknown {
                reason: claim("")
            },
        ]
        .as_slice(),
        "BlindVerdict::ALL 的三臂应与其转换源同形"
    );

    // VerdictAggregate 的编码：逐臂串名（不抽代表），再逐臂往返。
    assert_eq!(
        VerdictAggregate::AdjudicationRequired.as_str(),
        "adjudication_required",
        "VerdictAggregate 第一臂的编码"
    );
    assert_eq!(
        VerdictAggregate::Failed.as_str(),
        "failed",
        "VerdictAggregate 第二臂的编码"
    );
    assert_eq!(
        VerdictAggregate::Unknown.as_str(),
        "unknown",
        "VerdictAggregate 第三臂的编码"
    );
    assert_eq!(
        VerdictAggregate::PassedWithUnknown.as_str(),
        "passed_with_unknown",
        "VerdictAggregate 第四臂的编码"
    );
    assert_eq!(
        VerdictAggregate::Passed.as_str(),
        "passed",
        "VerdictAggregate 第五臂的编码"
    );
    for arm in VerdictAggregate::ALL {
        let encoded = arm.as_str();
        assert_eq!(
            VerdictAggregate::parse(encoded),
            Some(arm),
            "{encoded} 应解回 {arm:?}"
        );
        assert!(
            !encoded.is_empty() && encoded.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
            "编码应为小写、多词以 _ 连接（Debug 表示含大写），实际 {encoded}"
        );
    }
    // 表外取值一律 None（不取默认臂——取默认会把一份别的东西当成本臂处理）。
    assert_eq!(
        VerdictAggregate::parse("Passed"),
        None,
        "编码一律小写：大写形是表外串"
    );
    assert_eq!(VerdictAggregate::parse(""), None, "空串是表外串");
    assert_eq!(
        VerdictAggregate::parse("adjudication"),
        None,
        "截断的串是表外串"
    );

    // IndependenceVerdict 的编码：同上，逐臂串名 ＋ 往返 ＋ 表外 None。
    assert_eq!(
        IndependenceVerdict::Independent.as_str(),
        "independent",
        "IndependenceVerdict 第一臂的编码"
    );
    assert_eq!(
        IndependenceVerdict::NotIndependent.as_str(),
        "not_independent",
        "IndependenceVerdict 第二臂的编码"
    );
    assert_eq!(
        IndependenceVerdict::NotEstablished.as_str(),
        "not_established",
        "IndependenceVerdict 第三臂的编码"
    );
    for arm in IndependenceVerdict::ALL {
        let encoded = arm.as_str();
        assert_eq!(
            IndependenceVerdict::parse(encoded),
            Some(arm),
            "{encoded} 应解回 {arm:?}"
        );
        assert!(
            !encoded.is_empty() && encoded.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
            "编码应为小写、多词以 _ 连接（Debug 表示含大写），实际 {encoded}"
        );
    }
    assert_eq!(
        IndependenceVerdict::parse("Independent"),
        None,
        "编码一律小写：大写形是表外串"
    );
    assert_eq!(IndependenceVerdict::parse(""), None, "空串是表外串");
    assert_eq!(
        IndependenceVerdict::parse("not_established "),
        None,
        "带空格的串是表外串"
    );
}
