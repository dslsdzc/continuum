//! Requirement Coverage 的迭代域、两臂判定与五条计数规则（设计 §6.1–§6.3；
//! 计划 Task 5 的 Step 1）。
//!
//! 本文件里凡提到「编译失败」的地方都照纪律 1(c) 记账：**编译失败不是「变红」**，
//! 不假称它们会跑红。

use continuum_artifact::ArtifactId;
use continuum_graph::NodeId;
use continuum_operator::BackendId;
use continuum_verify::{
    CheckVerdict, Claim, ContractRequirement, ContractView, CoverageVerdict, Evidence, EvidenceId,
    EvidenceProducer, EvidenceScope, EvidenceStrength, EvidenceSubject, EvidenceType,
    IncompleteReason, RequirementClass, RequirementId, TestValidity, TestValidityCheck,
    VerificationProfile, coverage,
};
use serde_json::json;

// ---------- 夹具 ----------

/// 一条节点证据的生产者（同 `tests/evidence.rs` 的 `node_producer`）。
fn node_producer() -> EvidenceProducer {
    EvidenceProducer::Node {
        node: NodeId::new("worker-n1"),
        backend: BackendId::new("backend-a"),
    }
}

/// 一条证据：除 id / 类型 / 归属外全取固定值（`Test` 要读制品，故给一条制品引用）。
fn evidence_of(id: &str, evidence_type: EvidenceType, subject: EvidenceSubject) -> Evidence {
    Evidence::from_tool_result(
        EvidenceId::from_str(id),
        evidence_type,
        subject,
        Claim::from_str("断言文本（本 crate 不解析）"),
        node_producer(),
        vec![ArtifactId::new("artifact-a")],
        EvidenceStrength::from_value(json!({ "level": 3 })),
        EvidenceScope::from_value(json!({ "covers": "全部" })),
    )
    .expect("制品引用非空、类型与生产者匹配，构造应成功")
}

/// 一条绑定 `r-1` 的证据（id 与类型由入参给）。
fn evidence_for_r1(id: &str, evidence_type: EvidenceType) -> Evidence {
    evidence_of(
        id,
        evidence_type,
        EvidenceSubject::Requirement(RequirementId::from_str("r-1")),
    )
}

/// 一条 `Test` 证据（id 固定 `ev-test`、绑定 `r-1`）。
fn test_evidence_for_r1() -> Evidence {
    evidence_for_r1("ev-test", EvidenceType::Test)
}

/// 一条五臂齐的有效性记录（判定按 [`TestValidityCheck::ALL`] 的顺序给）。
fn record(evidence_id: &str, verdicts: [CheckVerdict; 5]) -> TestValidity {
    TestValidity::new(
        EvidenceId::from_str(evidence_id),
        TestValidityCheck::ALL.into_iter().zip(verdicts).collect(),
    )
    .expect("五臂齐的记录应构造成功")
}

/// 五条检查全 `Pass` ⇒ `Valid` 的记录。
fn valid_record(evidence_id: &str) -> TestValidity {
    record(evidence_id, [CheckVerdict::Pass; 5])
}

/// 视图里的一条 requirement（`verification_requirement` 固定 `None`：本块只读不判）。
fn requirement(id: &str, class: RequirementClass) -> ContractRequirement {
    ContractRequirement {
        id: RequirementId::from_str(id),
        class,
        verification_requirement: None,
    }
}

/// 一条 `REQUIRED` 的 requirement。
fn required(id: &str) -> ContractRequirement {
    requirement(id, RequirementClass::Required)
}

/// 由若干条 requirement 组一个视图。
fn view(requirements: Vec<ContractRequirement>) -> ContractView {
    ContractView { requirements }
}

/// 任务档案：`required_evidence_types` 由入参给，其余五项取空 / `false`
/// （它们只存不判，本块不读；§5.2）。
fn profile(required_evidence_types: Vec<EvidenceType>) -> VerificationProfile {
    VerificationProfile {
        required_evidence_types,
        optional_evidence_types: Vec::new(),
        fuzz_budget: None,
        mutation_threshold: None,
        property_requirements: Vec::new(),
        independent_verifier_required: false,
    }
}

/// 一条 REQUIRED（`r-1`）＋给定证据与有效性 ⇒ 该唯一的判定。
fn only_verdict(
    required_types: Vec<EvidenceType>,
    evidence: &[Evidence],
    validity: &[TestValidity],
) -> CoverageVerdict {
    let rows = coverage(
        &view(vec![required("r-1")]),
        evidence,
        validity,
        &profile(required_types),
    )
    .expect("coverage 今天没有失败路径（签名照设计 §6.1 照录）");
    assert_eq!(rows.len(), 1, "一条 REQUIRED 应恰好一行");
    rows[0].verdict.clone()
}

// ---------- 用例 ----------

/// §6.1 的迭代域：四种 `class` 取值各一条 requirement，**只有 `REQUIRED` 那条进判定**
/// （§6.1 末尾：四种取值各要一条用例）。
///
/// 红条件：把迭代域写成「全部 requirement」（**取反档**）⇒ 非 REQUIRED 那三条进结果表 ⇒ 红。
#[test]
fn only_required_requirements_enter_the_result() {
    let contract = view(vec![
        required("r-required"),
        requirement("r-preferred", RequirementClass::Preferred),
        requirement("r-flexible", RequirementClass::Flexible),
        requirement("r-unspecified", RequirementClass::Unspecified),
    ]);
    let rows =
        coverage(&contract, &[], &[], &profile(Vec::new())).expect("coverage 今天没有失败路径");
    assert_eq!(
        rows.len(),
        1,
        "四条 requirement 里只有 REQUIRED 那一条应进结果表"
    );
    assert_eq!(
        rows[0].requirement.as_str(),
        "r-required",
        "进结果表的应是 REQUIRED 那一条"
    );
}

/// §342 第 1 条（用户 REQUIRED 不得被静默降低）：**三条** REQUIRED ⇒ **三**行，
/// 且逐条按传入序对应（§12.1 (k)；本块唯一的可执行守卫）。
///
/// 期望值 3 是**手写**的——若写成 `contract.requirements.len()`，它测的是 Rust 的
/// `len()`、恒真（设计 §12.2 的「一条预防」）。
///
/// 红条件：迭代域去重（`HashSet`）或漏掉一条（**移除档**）⇒ 行数或逐条点名红。
#[test]
fn coverage_returns_one_row_per_required_requirement() {
    let contract = view(vec![required("r-1"), required("r-2"), required("r-3")]);
    let rows =
        coverage(&contract, &[], &[], &profile(Vec::new())).expect("coverage 今天没有失败路径");
    assert_eq!(rows.len(), 3, "三条 REQUIRED 应逐项一行（手写期望值 3）");
    assert_eq!(
        rows[0].requirement.as_str(),
        "r-1",
        "第 1 行是传入序第 1 条"
    );
    assert_eq!(
        rows[1].requirement.as_str(),
        "r-2",
        "第 2 行是传入序第 2 条"
    );
    assert_eq!(
        rows[2].requirement.as_str(),
        "r-3",
        "第 3 行是传入序第 3 条"
    );
}

/// (a) 一条 REQUIRED、零证据 ⇒ `Incomplete { NoValidEvidence }`（§190）。
#[test]
fn a_required_requirement_with_no_evidence_is_incomplete() {
    assert_eq!(
        only_verdict(Vec::new(), &[], &[]),
        CoverageVerdict::Incomplete {
            reason: IncompleteReason::NoValidEvidence
        },
        "零证据 ⇒ 确定的欠缺（§190：每一项 REQUIRED 都必须有对应证据）"
    );
}

/// (b) **本设计最重要的一条**（设计 §12.1）：证据齐备且有效 ⇒ **恰好是
/// `NotEstablished`**（不是 `Incomplete`，也不是任何正臂）。
///
/// **这是 §342 第 5 条的照片**：测试通过换不来「满足」。
/// 红条件：把返回的那一臂换成 `Incomplete`，或给 `CoverageVerdict` 加一枚正臂
/// （**取反档**）⇒ 红。
#[test]
fn a_requirement_with_complete_valid_evidence_is_not_established() {
    let test = test_evidence_for_r1();
    let validity = vec![valid_record("ev-test")];
    assert_eq!(
        only_verdict(vec![EvidenceType::Test], &[test], &validity),
        CoverageVerdict::NotEstablished,
        "证据齐备且有效仍不放行（OPEN-001：「充分」无判据；§342 第 5 条）"
    );
}

/// (c) `required_evidence_types == [Test, Fuzz]` 而证据只到 `Test` ⇒
/// `Incomplete { MissingRequiredTypes }`，**且 `missing` 恰是 `[Fuzz]`**（§261）。
///
/// 红条件：`missing` 里多列一型、或只断「非空」（**取反档**／等价变异体——「非空」断不住
/// 内容）⇒ 红。
#[test]
fn a_missing_required_type_is_incomplete_with_exactly_that_type() {
    let test = test_evidence_for_r1();
    let validity = vec![valid_record("ev-test")];
    assert_eq!(
        only_verdict(
            vec![EvidenceType::Test, EvidenceType::Fuzz],
            &[test],
            &validity
        ),
        CoverageVerdict::Incomplete {
            reason: IncompleteReason::MissingRequiredTypes {
                missing: vec![EvidenceType::Fuzz]
            },
        },
        "Fuzz 缺席 ⇒ 缺项恰是 [Fuzz]（§261；只断「非空」是等价变异体）"
    );
}

/// (d) 同上但 `Fuzz` 已在（非测试证据不要求有效性记录，§6.3 第 5 条）⇒
/// **`NotEstablished`**（缺项判据的**另一侧**）。
///
/// 只钉 (c) 会让「只要 profile 非空就恒缺项」漂过去。
#[test]
fn a_present_required_type_is_not_missing() {
    let test = test_evidence_for_r1();
    let fuzz = evidence_for_r1("ev-fuzz", EvidenceType::Fuzz);
    let validity = vec![valid_record("ev-test")];
    assert_eq!(
        only_verdict(
            vec![EvidenceType::Test, EvidenceType::Fuzz],
            &[test, fuzz],
            &validity
        ),
        CoverageVerdict::NotEstablished,
        "必需类型都在 ⇒ 不再缺项（缺项判据不得恒真）"
    );
}

/// (e) `Test` 证据、无 `TestValidity` 记录 ⇒ 不计数 ⇒ `Incomplete { NoValidEvidence }`。
///
/// 判据是 §191 的原文「测试不能因为存在于 `test/` 目录里就自动被认为有效」。
/// 红条件：把「无记录」当计数（**放宽档**）⇒ 红。
#[test]
fn a_test_evidence_without_a_validity_record_does_not_count() {
    let test = test_evidence_for_r1();
    assert_eq!(
        only_verdict(Vec::new(), &[test], &[]),
        CoverageVerdict::Incomplete {
            reason: IncompleteReason::NoValidEvidence
        },
        "无有效性记录的测试证据不计数（§191）"
    );
}

/// (f) `Test` 证据有 `Valid` 记录 ⇒ 计数 ⇒ `NotEstablished`（上一条的**另一侧**）。
///
/// 红条件：(e) 的反向（**收紧档**——有记录仍不计数）⇒ 红。
#[test]
fn a_test_evidence_with_a_valid_record_counts() {
    let test = test_evidence_for_r1();
    let validity = vec![valid_record("ev-test")];
    assert_eq!(
        only_verdict(Vec::new(), &[test], &validity),
        CoverageVerdict::NotEstablished,
        "Valid 的有效性记录 ⇒ 计数（§6.3 第 4 条）"
    );
}

/// §6.3 第 3 条：`Test` 证据的记录判 `Invalid` ⇒ 不计数 ⇒ `NoValidEvidence`
/// （§190 的「有效证据」里的「有效」由 §191 给）。
#[test]
fn a_test_evidence_marked_invalid_does_not_count() {
    let test = test_evidence_for_r1();
    let validity = vec![record(
        "ev-test",
        [
            CheckVerdict::Fail,
            CheckVerdict::Pass,
            CheckVerdict::Pass,
            CheckVerdict::Pass,
            CheckVerdict::Pass,
        ],
    )];
    assert_eq!(
        only_verdict(Vec::new(), &[test], &validity),
        CoverageVerdict::Incomplete {
            reason: IncompleteReason::NoValidEvidence
        },
        "判为无效的测试证据不计数（§6.3 第 3 条）"
    );
}

/// §6.3 第 4 条：`Unknown` 的记录**仍计数** ⇒ `NotEstablished`（上一条的**另一侧**）。
///
/// `Unknown` 是 §123 的正式未知（「有判据但判不出」），不计数会把它变成**确定的欠缺**，
/// 那是把判不出说成判为假。**这一条就是设计 §12.2 预告的那一档的语料**
/// （「`Unknown` 的检查结果算作不计数 ⇒ 只在『证据里有 `Unknown` 有效性』的语料上红」）。
/// 红条件：把 `Unknown` 归到不计数（**收紧档**）⇒ 红。
#[test]
fn a_test_evidence_with_unknown_validity_still_counts() {
    let test = test_evidence_for_r1();
    let validity = vec![record(
        "ev-test",
        [
            CheckVerdict::Pass,
            CheckVerdict::Pass,
            CheckVerdict::Pass,
            CheckVerdict::Pass,
            CheckVerdict::Unknown,
        ],
    )];
    assert_eq!(
        only_verdict(Vec::new(), &[test], &validity),
        CoverageVerdict::NotEstablished,
        "正式未知仍计数（§6.3 第 4 条；不得把「判不出」降格成「判为假」）"
    );
}

/// §6.3 第 5 条：非测试证据（`Fuzz`）**不要求**有效性记录 ⇒ 计数 ⇒ `NotEstablished`。
///
/// 判据：§191 的判据只覆盖测试证据，规范未给非测试证据的「不计数」判据，本设计不发明。
/// 红条件：把非测试证据也要求有效性记录（**收紧档**）⇒ 红。
#[test]
fn a_non_test_evidence_counts_without_a_validity_record() {
    let fuzz = evidence_for_r1("ev-fuzz", EvidenceType::Fuzz);
    assert_eq!(
        only_verdict(Vec::new(), &[fuzz], &[]),
        CoverageVerdict::NotEstablished,
        "非测试证据无需有效性记录即计数（§6.3 第 5 条）"
    );
}

/// §6.3 第 1 条（前提）：证据挂到**别的** requirement 上 ⇒ 本条 requirement 的计数集为空
/// ⇒ `NoValidEvidence`。
///
/// 红条件：`counts` 忽略 `subject`、把每条证据都算给每条 REQUIRED（**放宽档**）⇒ 红
/// （语料里给该证据配 `Valid` 记录，否则「忽略 subject」会被第 2 条挡掉、变异不变红）。
#[test]
fn evidence_of_another_requirement_does_not_count() {
    let other = evidence_of(
        "ev-1",
        EvidenceType::Test,
        EvidenceSubject::Requirement(RequirementId::from_str("r-other")),
    );
    let validity = vec![valid_record("ev-1")];
    assert_eq!(
        only_verdict(Vec::new(), &[other], &validity),
        CoverageVerdict::Incomplete {
            reason: IncompleteReason::NoValidEvidence
        },
        "挂到别的 requirement 的证据不计数（§6.3 第 1 条）"
    );
}

/// **另一侧**：`subject == Unattached` 的证据同样不计数 ⇒ `NoValidEvidence`。
///
/// **两条都要**：`Unattached` 是 §4.3.1 的显式臂，漏了它会让「挂错 requirement」静默。
#[test]
fn unattached_evidence_does_not_count() {
    let unattached = evidence_of("ev-1", EvidenceType::Test, EvidenceSubject::Unattached);
    let validity = vec![valid_record("ev-1")];
    assert_eq!(
        only_verdict(Vec::new(), &[unattached], &validity),
        CoverageVerdict::Incomplete {
            reason: IncompleteReason::NoValidEvidence
        },
        "Unattached 的证据不属于任何 requirement（§6.3 第 1 条）"
    );
}

/// 四值各一次编码往返＋`ALL` 的数目与逐字点名（§224 的四个规范名，编码取小写）。
///
/// 红条件：改某一臂的串（**取反档**）⇒ 该臂的往返红；删 [`RequirementClass::ALL`] 里的一臂
/// （**移除档**）⇒ 改了长度标注则 `len()` 断言与逐字点名红（运行期），不改标注则数组长度
/// 不符直接编译失败。**「删一臂」的锚点只取 `ALL`**——删枚举定义里那一臂是用例侧的
/// 编译失败，不是变红（纪律 1(a)、(c)）。
#[test]
fn the_four_class_values_each_round_trip() {
    assert_eq!(
        RequirementClass::ALL.len(),
        4,
        "RequirementClass 的名单与臂数不符"
    );
    assert_eq!(
        RequirementClass::ALL.as_slice(),
        [
            RequirementClass::Required,
            RequirementClass::Preferred,
            RequirementClass::Flexible,
            RequirementClass::Unspecified,
        ]
        .as_slice(),
        "ALL 的四项与 §224 的四个规范名（REQUIRED / PREFERRED / FLEXIBLE / UNSPECIFIED）不符"
    );
    assert_eq!(
        RequirementClass::Required.as_str(),
        "required",
        "§224 REQUIRED 的编码"
    );
    assert_eq!(
        RequirementClass::Preferred.as_str(),
        "preferred",
        "§224 PREFERRED 的编码"
    );
    assert_eq!(
        RequirementClass::Flexible.as_str(),
        "flexible",
        "§224 FLEXIBLE 的编码"
    );
    assert_eq!(
        RequirementClass::Unspecified.as_str(),
        "unspecified",
        "§224 UNSPECIFIED 的编码"
    );
    for class in RequirementClass::ALL {
        let encoded = class.as_str();
        assert_eq!(
            RequirementClass::parse(encoded),
            Some(class),
            "{encoded} 应解回 {class:?}"
        );
        assert!(
            !encoded.is_empty() && encoded.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
            "编码应为小写、多词以 _ 连接（规范名是大写；Debug 表示也含大写），实际 {encoded}"
        );
    }
    assert_eq!(
        RequirementClass::parse("REQUIRED"),
        None,
        "表外取值一律 None（大写规范名不是编码）"
    );
    assert_eq!(RequirementClass::parse(""), None, "表外取值一律 None");
}

/// 两枚判定枚举各把它的臂点名列齐（设计 §12.1 首段：「每一枚枚举的每一臂」都有数目断言；
/// 计划把本 task 的三处分为 `CoverageVerdict` 2 / `IncompleteReason` 2 / `RequirementClass` 4，
/// 后者在上一条用例）。
///
/// `ALL` 只用于数臂与逐臂点名，载荷取最小样例。红条件：删 `ALL` 里的一项（**移除档**）⇒
/// 数目断言与逐字点名红。
#[test]
fn the_two_verdict_enums_each_name_their_arms() {
    assert_eq!(
        CoverageVerdict::ALL.len(),
        2,
        "CoverageVerdict 的名单与臂数不符"
    );
    assert_eq!(
        CoverageVerdict::ALL.as_slice(),
        [
            CoverageVerdict::Incomplete {
                reason: IncompleteReason::NoValidEvidence
            },
            CoverageVerdict::NotEstablished,
        ]
        .as_slice(),
        "ALL 的两项与 §6.2 的两臂不符（没有第三臂）"
    );
    assert_eq!(
        IncompleteReason::ALL.len(),
        2,
        "IncompleteReason 的名单与臂数不符"
    );
    assert_eq!(
        IncompleteReason::ALL.as_slice(),
        [
            IncompleteReason::NoValidEvidence,
            IncompleteReason::MissingRequiredTypes {
                missing: Vec::new()
            },
        ]
        .as_slice(),
        "ALL 的两项与 §6.2 的两条理由不符"
    );
}
