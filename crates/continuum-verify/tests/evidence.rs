//! `Evidence` 的唯一产生点（§4.4/§89）：拒绝表的两侧、两臂 `subject` 的可区分性、
//! 八个访问器与两枚不透明值的往返（计划 Task 2 的 Step 1；设计 §4.3、§4.4）。
//!
//! 本文件里标「编译期照片」的地方**不是运行期红**——判据是「编译不过」或「从编译失败
//! 变成编译通过」，不假称它们会跑红（纪律 1(c)：「编译失败不是变红」）。

use continuum_artifact::ArtifactId;
use continuum_graph::NodeId;
use continuum_operator::BackendId;
use continuum_verify::{
    Claim, Evidence, EvidenceId, EvidenceProducer, EvidenceScope, EvidenceStrength,
    EvidenceSubject, EvidenceType, RequirementId, VerifyError,
};
use serde_json::json;

/// 一条节点证据的生产者：`Node { node, backend }`（设计 §4.3.3）。
fn node_producer() -> EvidenceProducer {
    EvidenceProducer::Node {
        node: NodeId::new("worker-n1"),
        backend: BackendId::new("backend-a"),
    }
}

/// 除四个入参外全取固定值的构造。拒绝表的两侧都从它出发。
fn evidence(
    evidence_type: EvidenceType,
    subject: EvidenceSubject,
    producer: EvidenceProducer,
    artifact_refs: Vec<ArtifactId>,
) -> Result<Evidence, VerifyError> {
    Evidence::from_tool_result(
        EvidenceId::from_str("ev-1"),
        evidence_type,
        subject,
        Claim::from_str("断言文本（本 crate 不解析）"),
        producer,
        artifact_refs,
        EvidenceStrength::from_value(json!({ "level": 3 })),
        EvidenceScope::from_value(json!({ "covers": "全部" })),
    )
}

/// [`evidence`] 的常用形：`subject` 固定 `Unattached`（拒绝表两侧的主题都不是归属）。
fn unattached(
    evidence_type: EvidenceType,
    producer: EvidenceProducer,
    artifact_refs: Vec<ArtifactId>,
) -> Result<Evidence, VerifyError> {
    evidence(
        evidence_type,
        EvidenceSubject::Unattached,
        producer,
        artifact_refs,
    )
}

/// §4.4 拒绝表的第一行：十三型「要读制品」的证据在 `artifact_refs` 为空时被拒。
///
/// **十三型逐型各一条，不抽代表**（设计 §4.4 的表逐型列出）。`HumanConfirmation` 是
/// 十四臂里唯一不需要制品的一型，它不在这里——它的对照在
/// `human_confirmation_needs_no_artifact`。
///
/// 红条件（取反档）：把实现里那条穷尽 `match`（`src/evidence.rs` 的
/// `reads_an_artifact`，对 `evidence_type` 穷尽、**无通配臂**）的某一型挪到
/// 「不需要制品」支 ⇒ 该型那一条红。**这条拒绝规则实现成对 `evidence_type` 的穷尽
/// `match`**——`EvidenceType` 加臂时那条 `match` **编译不过**，**那是编译期照片、不是
/// 运行期红**（纪律 1(c)：编译失败不算变红）。
#[test]
fn a_type_that_reads_an_artifact_is_rejected_when_artifact_refs_is_empty() {
    assert_eq!(
        unattached(EvidenceType::Test, node_producer(), Vec::new()),
        Err(VerifyError::EvidenceWithoutArtifact {
            evidence_type: EvidenceType::Test
        }),
        "Test 要读制品：artifact_refs 为空应在构造点被拒（§4.4 第 1 条）"
    );
    assert_eq!(
        unattached(EvidenceType::Fuzz, node_producer(), Vec::new()),
        Err(VerifyError::EvidenceWithoutArtifact {
            evidence_type: EvidenceType::Fuzz
        }),
        "Fuzz 要读制品：artifact_refs 为空应在构造点被拒（§4.4 第 1 条）"
    );
    assert_eq!(
        unattached(EvidenceType::Property, node_producer(), Vec::new()),
        Err(VerifyError::EvidenceWithoutArtifact {
            evidence_type: EvidenceType::Property
        }),
        "Property 要读制品：artifact_refs 为空应在构造点被拒（§4.4 第 1 条）"
    );
    assert_eq!(
        unattached(EvidenceType::FormalProof, node_producer(), Vec::new()),
        Err(VerifyError::EvidenceWithoutArtifact {
            evidence_type: EvidenceType::FormalProof
        }),
        "FormalProof 要读制品：artifact_refs 为空应在构造点被拒（§4.4 第 1 条）"
    );
    assert_eq!(
        unattached(EvidenceType::StaticAnalysis, node_producer(), Vec::new()),
        Err(VerifyError::EvidenceWithoutArtifact {
            evidence_type: EvidenceType::StaticAnalysis
        }),
        "StaticAnalysis 要读制品：artifact_refs 为空应在构造点被拒（§4.4 第 1 条）"
    );
    assert_eq!(
        unattached(EvidenceType::Benchmark, node_producer(), Vec::new()),
        Err(VerifyError::EvidenceWithoutArtifact {
            evidence_type: EvidenceType::Benchmark
        }),
        "Benchmark 要读制品：artifact_refs 为空应在构造点被拒（§4.4 第 1 条）"
    );
    assert_eq!(
        unattached(EvidenceType::VisualCheck, node_producer(), Vec::new()),
        Err(VerifyError::EvidenceWithoutArtifact {
            evidence_type: EvidenceType::VisualCheck
        }),
        "VisualCheck 要读制品：artifact_refs 为空应在构造点被拒（§4.4 第 1 条）"
    );
    assert_eq!(
        unattached(EvidenceType::MetadataCheck, node_producer(), Vec::new()),
        Err(VerifyError::EvidenceWithoutArtifact {
            evidence_type: EvidenceType::MetadataCheck
        }),
        "MetadataCheck 要读制品：artifact_refs 为空应在构造点被拒（§4.4 第 1 条）"
    );
    assert_eq!(
        unattached(EvidenceType::Differential, node_producer(), Vec::new()),
        Err(VerifyError::EvidenceWithoutArtifact {
            evidence_type: EvidenceType::Differential
        }),
        "Differential 要读制品：artifact_refs 为空应在构造点被拒（§4.4 第 1 条）"
    );
    assert_eq!(
        unattached(EvidenceType::Metamorphic, node_producer(), Vec::new()),
        Err(VerifyError::EvidenceWithoutArtifact {
            evidence_type: EvidenceType::Metamorphic
        }),
        "Metamorphic 要读制品：artifact_refs 为空应在构造点被拒（§4.4 第 1 条）"
    );
    assert_eq!(
        unattached(EvidenceType::ModelReview, node_producer(), Vec::new()),
        Err(VerifyError::EvidenceWithoutArtifact {
            evidence_type: EvidenceType::ModelReview
        }),
        "ModelReview 要读制品：artifact_refs 为空应在构造点被拒（§4.4 第 1 条）"
    );
    assert_eq!(
        unattached(
            EvidenceType::ContradictionCheck,
            node_producer(),
            Vec::new()
        ),
        Err(VerifyError::EvidenceWithoutArtifact {
            evidence_type: EvidenceType::ContradictionCheck
        }),
        "ContradictionCheck 要读制品：artifact_refs 为空应在构造点被拒（§4.4 第 1 条）"
    );
    assert_eq!(
        unattached(
            EvidenceType::CitationVerification,
            node_producer(),
            Vec::new()
        ),
        Err(VerifyError::EvidenceWithoutArtifact {
            evidence_type: EvidenceType::CitationVerification
        }),
        "CitationVerification 要读制品：artifact_refs 为空应在构造点被拒（§4.4 第 1 条）"
    );
}

/// 上一条的**对照侧**：`HumanConfirmation` 不需要制品（§4.4 表末行的例外）。
///
/// 只钉拒绝那侧会让「一律拒绝空 `artifact_refs`」漂过去，故这一侧必须存在。
/// 红条件（收紧档）：把 `reads_an_artifact` 里的 `HumanConfirmation` 支挪到
/// 「需要制品」支 ⇒ 本条红。
#[test]
fn human_confirmation_needs_no_artifact() {
    let constructed = unattached(
        EvidenceType::HumanConfirmation,
        EvidenceProducer::Human,
        Vec::new(),
    )
    .expect("HumanConfirmation ＋ Human 生产者：空 artifact_refs 应可构造（§4.4 的例外）");
    assert_eq!(
        constructed.evidence_type(),
        EvidenceType::HumanConfirmation,
        "读回的类型应是 HumanConfirmation"
    );
}

/// §4.4 拒绝表的第二行：`HumanConfirmation` 的生产者不是 `Human` 时被拒。
///
/// 给一件制品，使「空 `artifact_refs`」那条检查不参与（`HumanConfirmation` 本就不需要
/// 制品），本条只钉生产者不匹配这一条。红条件（移除档）：删掉该检查 ⇒ 本条红。
#[test]
fn human_confirmation_from_a_node_is_rejected() {
    let result = unattached(
        EvidenceType::HumanConfirmation,
        node_producer(),
        vec![ArtifactId::new("a-1")],
    );
    assert_eq!(
        result,
        Err(VerifyError::HumanConfirmationFromNonHuman),
        "HumanConfirmation ＋ Node 生产者应在构造点被拒（§4.4 第 2 条）"
    );
}

/// 上一条的**另一侧**：`producer: Human` 与别的类型组合是合法的。
///
/// 上一条不许被实现成「`producer` 必须是 `Node`」——§4.4 只对
/// `HumanConfirmation` 约束生产者。红条件（收紧档）：把生产者检查写成
/// 「`producer == Human` 即拒」或「非 `Node` 即拒」⇒ 本条红。
#[test]
fn a_human_producer_on_another_type_is_accepted() {
    let constructed = unattached(
        EvidenceType::Test,
        EvidenceProducer::Human,
        vec![ArtifactId::new("a-1")],
    )
    .expect("Test ＋ Human 生产者 ＋ 一件制品应可构造（§4.4 只约束 HumanConfirmation 的生产者）");
    assert_eq!(
        constructed.evidence_type(),
        EvidenceType::Test,
        "读回的类型应是 Test"
    );
}

/// `Requirement(id)` 与 `Unattached` 两臂的可区分性（设计 §4.3.1）。
///
/// 断言读回的 `subject` 是哪一臂——**穷尽 `match`，不抽代表**。
/// 红条件（取反档）：把 `from_tool_result` 里构 `Evidence` 的**那一处 `subject` 字段
/// 初始化**改成恒写 `EvidenceSubject::Unattached` ⇒ 本条红。**锚点是唯一的**：
/// `Evidence` 只有一个构造点，八个私有字段各只在那处初始化一次。**改枚举定义那一处不是
/// 本条锚点**——用例文件里的 `EvidenceSubject::Unattached` 引用会编译不过，那是编译失败、
/// 不是变红（纪律 1(c)）。**理由**：失配的方向是静默漏配——一条证据谁都不要，覆盖判定
/// 看到空集合，把「有证据但没挂上」误判成「没证据」（设计 §4.3.1 读法 3）。
#[test]
fn the_two_subjects_are_distinguishable() {
    let attached = evidence(
        EvidenceType::Test,
        EvidenceSubject::Requirement(RequirementId::from_str("req-42")),
        node_producer(),
        vec![ArtifactId::new("a-1")],
    )
    .expect("Test ＋ 一件制品应可构造");
    let unattached_subject = unattached(
        EvidenceType::Test,
        node_producer(),
        vec![ArtifactId::new("a-1")],
    )
    .expect("同上");

    match attached.subject() {
        EvidenceSubject::Requirement(id) => assert_eq!(
            id.as_str(),
            "req-42",
            "Requirement(..) 构造的证据应读回同一条 Requirement"
        ),
        EvidenceSubject::Unattached => panic!(
            "Requirement(..) 构造的证据读回 Unattached：构造点漏配 subject（静默漏配的方向）"
        ),
    }
    match unattached_subject.subject() {
        EvidenceSubject::Unattached => {}
        EvidenceSubject::Requirement(id) => panic!(
            "Unattached 构造的证据读回 Requirement({})：构造点把 subject 硬塞了归属",
            id.as_str()
        ),
    }
}

/// 一次构造，八个访问器逐字段读回（设计 §4.4 的「读取面」）。
///
/// **本条是编译期照片**：八个访问器按名逐个调用（`id` / `evidence_type` / `subject` /
/// `claim` / `producer` / `artifact_refs` / `strength` / `scope`），删掉或改名任何一个
/// 即编译不过（`E0599`）。它**不是运行期红**——照实写（纪律 1(c)）。
#[test]
fn the_eight_accessors_read_back_what_was_constructed() {
    let constructed = Evidence::from_tool_result(
        EvidenceId::from_str("ev-7"),
        EvidenceType::MetadataCheck,
        EvidenceSubject::Requirement(RequirementId::from_str("req-42")),
        Claim::from_str("resolution metadata 读出 1920×1080"),
        node_producer(),
        vec![ArtifactId::new("a-1"), ArtifactId::new("a-2")],
        EvidenceStrength::from_value(json!({ "level": 3 })),
        EvidenceScope::from_value(json!({ "covers": "resolution" })),
    )
    .expect("八项齐备应可构造");

    assert_eq!(constructed.id().as_str(), "ev-7", "id 访问器");
    assert_eq!(
        constructed.evidence_type(),
        EvidenceType::MetadataCheck,
        "evidence_type 访问器"
    );
    assert_eq!(
        constructed.subject(),
        &EvidenceSubject::Requirement(RequirementId::from_str("req-42")),
        "subject 访问器"
    );
    assert_eq!(
        constructed.claim().as_str(),
        "resolution metadata 读出 1920×1080",
        "claim 访问器"
    );
    match constructed.producer() {
        EvidenceProducer::Node { node, backend } => {
            assert_eq!(node.as_str(), "worker-n1", "producer 访问器的节点");
            assert_eq!(backend.as_str(), "backend-a", "producer 访问器的后端");
        }
        EvidenceProducer::Human => panic!("Node 生产者读回 Human：producer 访问器串了字段"),
    }
    let refs: Vec<&str> = constructed
        .artifact_refs()
        .iter()
        .map(ArtifactId::as_str)
        .collect();
    assert_eq!(refs, ["a-1", "a-2"], "artifact_refs 访问器（含顺序）");
    assert_eq!(
        constructed.strength().as_value(),
        &json!({ "level": 3 }),
        "strength 访问器"
    );
    assert_eq!(
        constructed.scope().as_value(),
        &json!({ "covers": "resolution" }),
        "scope 访问器"
    );
}

/// `EvidenceStrength` / `EvidenceScope` 是**不透明值**（设计 §4.3.4）：只有
/// `as_value` / `from_value`，逐枚往返——标量与**非标量**各一条，断言 `as_value()`
/// 读回构造时给的那个 `serde_json::Value`。
///
/// **限度（照实写）**：把某枚改成命名档枚举（例如加了 `Ord` 或阈值）会让本用例
/// **编译不过**（`from_value` / `as_value` 不再是那两个签名）——**编译不过不算变红**
/// （纪律 1(c)）。故本条的守卫是**运行期往返**，它钉的是「取值域不由本块封闭」。
#[test]
fn strength_and_scope_are_opaque_values() {
    let scalar = json!(3);
    assert_eq!(
        EvidenceStrength::from_value(scalar.clone()).as_value(),
        &scalar,
        "标量强度应原样读回（无命名档、无阈值）"
    );
    let non_scalar = json!({ "levels": [1, 2, 3], "note": "取值域不由本块封闭" });
    assert_eq!(
        EvidenceStrength::from_value(non_scalar.clone()).as_value(),
        &non_scalar,
        "非标量强度应原样读回（本块不封闭它的取值域）"
    );

    let scalar_scope = json!("目录级");
    assert_eq!(
        EvidenceScope::from_value(scalar_scope.clone()).as_value(),
        &scalar_scope,
        "标量范围应原样读回"
    );
    let non_scalar_scope = json!(["文件", { "目录": "src" }]);
    assert_eq!(
        EvidenceScope::from_value(non_scalar_scope.clone()).as_value(),
        &non_scalar_scope,
        "非标量范围应原样读回（比较规则规范未给）"
    );
}
