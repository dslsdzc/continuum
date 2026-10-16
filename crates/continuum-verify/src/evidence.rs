//! §258 的 `Evidence`：八个字段与它唯一的产生点 [`Evidence::from_tool_result`]
//! （设计 §4/§89）。本文件装设计 §4.3 的三处形状（`EvidenceSubject` / `Claim` 与
//! `WorkerCompletionDeclaration` 的分界 / `EvidenceProducer`）与 §4.3.4 的两枚不透明值。

use continuum_artifact::ArtifactId;
use continuum_graph::NodeId;
use continuum_operator::BackendId;
use serde_json::Value;

use crate::contract_view::RequirementId;
use crate::error::VerifyError;
use crate::evidence_type::EvidenceType;

/// §258 的 `id`（设计 §4.1：照录，`String` 承载）。
///
/// 不透明串：只有 [`EvidenceId::as_str`] / [`EvidenceId::from_str`]，无校验、
/// 不改写不裁剪（照 P4 对 id 类类型的处置）。
/// 派生 `PartialEq`：它是 [`Evidence`] 的字段，而 `Evidence` 派生 `PartialEq`
/// （本 task 定的派生清单，设计 §4.4）。
#[derive(Debug, Clone, PartialEq)]
pub struct EvidenceId(String);

impl EvidenceId {
    /// 取里子串。**不改写、不裁剪**。
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 从任意串构造。**无校验**——§258 只给了这个字段名，没给取值域。
    ///
    /// 名字用 `from_str` 而没有实现 `std::str::FromStr`：后者要求一个 `Err` 关联类型，
    /// 而本类型**不失败**（同 P4 的 id 类类型）。
    pub fn from_str(s: &str) -> Self {
        EvidenceId(s.to_owned())
    }
}

/// §258 的 `claim`。**不解析**：规范未给语法（设计 §4.3.2）。
///
/// **据实记录的残余**：`Claim` 是自由文本，故它**是**一个能把自然语言塞进初判输入的
/// 口子——结构性保证只能做到「`Evidence` 没有 reasoning 字段」，做不到「`claim` 里没人
/// 塞 reasoning」。这是缺口，不是已解决的问题（设计 §15 第 2 条）。
#[derive(Debug, Clone, PartialEq)]
pub struct Claim(String);

impl Claim {
    /// 取里子串。**不改写、不裁剪**。
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 从任意串构造。**无校验、不解析**（理由同 [`EvidenceId::from_str`]）。
    pub fn from_str(s: &str) -> Self {
        Claim(s.to_owned())
    }

    /// 空串 `Claim`（编译期常量）。**只在本 crate 内可见**。
    ///
    /// 用途只有一个：本 crate 的枚举名单（`VerifierVerdict::ALL` / `BlindVerdict::ALL`，
    /// 落在 `src/verdict.rs`）里载荷臂需要一个编译期可得的载荷，而 [`Claim::from_str`]
    /// 不是 `const fn`（`String::to_owned` 不是 `const`）。**它不是一条构造路径**——
    /// `Claim` 本来无校验；载荷在名单里不参与「有哪些臂」这件事，逐臂点名由
    /// `tests/verifier.rs` 把关。
    pub(crate) const EMPTY: Claim = Claim(String::new());
}

/// §29 的「Worker reasoning summary / Worker 自己的完成声明」。
///
/// **它不是一个 `Evidence`，且没有任何到 `Evidence` 的转换**——
/// §263 禁止它进入初判输入，若它能变成 Evidence，那条禁止就没有落点。
/// **可以构造**（§29 的「之后才比较」需要它）；唯一消费者是 `PostBlindVerifierInput`（Task 6）。
///
/// 不派生 `PartialEq`：本 task 没有按它比较的判定，而它不进 [`Evidence`]，
/// 故不因 `Evidence` 的派生清单而承压（要加时随用它的判定一起加）。
#[derive(Debug, Clone)]
pub struct WorkerCompletionDeclaration(String);

impl WorkerCompletionDeclaration {
    /// 取里子串。**不改写、不裁剪**。
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 从任意串构造。**无校验**——规范只给了这个概念的出处（§29），没给它的语法。
    pub fn from_str(s: &str) -> Self {
        WorkerCompletionDeclaration(s.to_owned())
    }
}

/// §258 的 `subject`（设计 §4.3.1：两臂，**不是** artifact）。
///
/// 读法的理由（三条）：不与 `artifact_refs[]` 重复；§260 的 `Requirement → Evidence Set`
/// 需要它，而 §258 的八个字段里只有 `subject` 的语义是「关于什么」；必须是强类型
/// （`String` 会让归属判定退化成字符串比较，且失配方向是静默漏配）。
#[derive(Debug, Clone, PartialEq)]
pub enum EvidenceSubject {
    /// 该证据**为**某项 Requirement 提出。§260 的 `Requirement → Evidence Set` 靠它成立。
    ///
    /// 载荷是本块定义的 [`RequirementId`]：与 P4 的同名类型是两个类型，
    /// 对应关系由装配方维持（设计 §4.3.1 的订正段）。
    Requirement(RequirementId),
    /// 该证据不针对任何 Requirement。
    ///
    /// §259 明写测试通过只产生 `Evidence<Test>`、不产生 `requirement_satisfied`；
    /// §89 说工具结果必须先转成 Evidence。两者合起来意味着**证据先于归属存在**：
    /// 工具节点产出一条证据时，它可能还不知道（或不该由它决定）面向哪条 Requirement。
    /// 给这个状态一个臂，是为了让它**看得见**，而不是被硬塞进某条 Requirement。
    Unattached,
}

impl EvidenceSubject {
    /// 两臂的判别串（小写、多词以 `_` 连接）：落 `evidence.subject_kind` 列（Task 9）。
    ///
    /// **本函数是该编码唯一的产生点**。match 穷尽且无通配臂：加臂时本函数编译不过，
    /// 编码不会漏分支。
    ///
    /// **不设 `parse`**：本 task 的证据表只写不读（没有 `load_evidence`，消费者未建；
    /// 计划 `## 遗留` 第三节记「不建读取路径」），读回路径不建则逆函数没有落点——
    /// 它是随第一个读回消费者落地的东西，不是遗漏。承载 Requirement 载荷的那一列
    /// （`subject_requirement`）不进本编码：它是 [`RequirementId`] 的串或 `NULL`，
    /// 由写行处直接取 `as_str`。
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Requirement(_) => "requirement",
            Self::Unattached => "unattached",
        }
    }
}

/// §258 的 `producer`（设计 §4.3.3：两臂）。
///
/// §263 的隔离判定要能命名「这条证据是谁产的」，否则那条 MUST 没有判据。
/// **这里没有 `Worker` 这一臂，是刻意的**：Worker 的完成声明**不是证据**（§263），
/// 把它做成 `EvidenceProducer` 的一个臂，等于承认它可以是一条 `Evidence`。
/// 它以独立的类型存在（[`WorkerCompletionDeclaration`]），不在本枚举内。
#[derive(Debug, Clone, PartialEq)]
pub enum EvidenceProducer {
    /// 由某个算子节点产出（§89：工具结果转成 Evidence）。
    Node {
        /// 生产者的节点身份（P1 的 `NodeId`）。
        node: NodeId,
        /// 该节点这一轮的 backend（§245 的 `BackendId`）：§262 第 3、4 级的可用性、
        /// §120 的「不同 family / 不同证据路径」都要能命名「哪一条路径产出了这条证据」。
        backend: BackendId,
    },
    /// 由人在环中给出（§258 的 `HUMAN_CONFIRMATION`）。
    Human,
}

/// §258 的 `strength`。**规范只给字段名**：取值域与比较规则都没有（设计 §4.3.4）。
///
/// 只有 [`EvidenceStrength::as_value`] / [`EvidenceStrength::from_value`]，
/// **没有** `Ord`、没有阈值、没有命名档。§265 的「大量 mutation 无法被测试捕获 ⇒
/// test evidence strength ↓」是一条**方向**陈述，而等级集（能降到哪一档、档与档之间
/// 怎么比）规范未给——下降在类型上需要 `Ord`，给一个规范没给的 `Ord` 就是发明，
/// 故方向照录、判定不落。
///
/// 本轮**没有消费者**：「谁读它」取决于 OPEN-001 怎么解（设计 §4.3.4）。
#[derive(Debug, Clone, PartialEq)]
pub struct EvidenceStrength(Value);

impl EvidenceStrength {
    /// 取载荷（判据留给调用方）。
    pub fn as_value(&self) -> &Value {
        &self.0
    }

    /// 从任意 `serde_json::Value` 构造。**无校验**——取值域不由本块封闭。
    pub fn from_value(value: Value) -> Self {
        EvidenceStrength(value)
    }
}

/// §258 的 `scope`。**规范只给字段名**（设计 §4.3.4）：取值域与比较规则都没有。
///
/// 只有 [`EvidenceScope::as_value`] / [`EvidenceScope::from_value`]。**为什么不给判据**：
/// `scope` 的语义若与覆盖有关（「这条证据覆盖了 Requirement 的哪一部分」），那么
/// 「各部分合起来是否盖满」正是 OPEN-001 缺的那件东西（设计 §3）。故它同样只存。
///
/// 本轮**没有消费者**（同 [`EvidenceStrength`]）。
#[derive(Debug, Clone, PartialEq)]
pub struct EvidenceScope(Value);

impl EvidenceScope {
    /// 取载荷（判据留给调用方）。
    pub fn as_value(&self) -> &Value {
        &self.0
    }

    /// 从任意 `serde_json::Value` 构造。**无校验**——取值域不由本块封闭。
    pub fn from_value(value: Value) -> Self {
        EvidenceScope(value)
    }
}

/// §258 的 `Evidence`：八个字段**全部私有**（设计 §4.4 的「唯一产生点」是结构事实）。
///
/// # 读取面（本 task 定的形状）
///
/// 每个字段配一个只读访问器（[`Evidence::id`] / [`Evidence::evidence_type`] /
/// [`Evidence::subject`] / [`Evidence::claim`] / [`Evidence::producer`] /
/// [`Evidence::artifact_refs`] / [`Evidence::strength`] / [`Evidence::scope`]）。
/// **代价**：多八个公开读者；**收益**：不必把字段开成 `pub`——那会与 §4.4 相抵。
///
/// # 派生清单（本 task 定的形状）
///
/// 允许 `Debug` / `Clone` / `PartialEq`——它们不产生构造路径。
/// **禁止** `Default`、`From`、`FromStr`，且**禁止** `Serialize` / `Deserialize`：
/// `Deserialize` 就是第二个构造点，加它等于把「唯一产生点」这句话变假（设计 §4.4 只
/// 点名了前三项，`Deserialize` 由本计划补上，计划的 `## 遗留` 第二节）。
///
/// **「唯一产生点」这句话的照片只有编译期两条**（`Evidence { .. }` 与
/// `Evidence::default()` 都编不过）：〔时点〕本 task 落地时它们尚未落地，
/// 在 Task 10 的 `trybuild` 样例（计划 §12.3 第 4 条）。
///
/// # 证据记录只追加
///
/// `Evidence` 一经写入不改（设计 §4.1）。§241 的不可变原则正文说的是 Artifact，
/// 同一条纪律用在这里的理由是具体的——**有效性判定**（§6.4）与**证据**必须是两条记录，
/// 否则「验证者改了被验之物」。故本类型没有更新路径（`evidence` 表只有 INSERT，Task 9）。
#[derive(Debug, Clone, PartialEq)]
pub struct Evidence {
    id: EvidenceId,
    evidence_type: EvidenceType,
    subject: EvidenceSubject,
    claim: Claim,
    producer: EvidenceProducer,
    artifact_refs: Vec<ArtifactId>,
    strength: EvidenceStrength,
    scope: EvidenceScope,
}

impl Evidence {
    /// §89：工具结果转成 `Evidence` 的**唯一**构造点。
    ///
    /// 拒绝两条：
    /// （1）十三型需要制品的证据而 `artifact_refs` 为空
    /// （[`VerifyError::EvidenceWithoutArtifact`]，判据在同文件的 `reads_an_artifact`）；
    /// （2）`HumanConfirmation` 而 `producer != Human`
    /// （[`VerifyError::HumanConfirmationFromNonHuman`]）。
    ///
    /// **不检查**：`claim` 的内容、`strength` / `scope` 的值、`artifact_refs` 里制品是否
    /// 存在（那要查库，属于 `artifact_exists`，Task 9）、生产者的节点是否真属该图
    /// （在 Task 6 的装配处查）——设计 §4.4 的「它不检查什么」逐条照办。
    ///
    /// 跨块口径（协调者 2026-10-10 裁定）：四个领域算子块一律不写域包装，
    /// 一律直接调本构造点。
    pub fn from_tool_result(
        id: EvidenceId,
        evidence_type: EvidenceType,
        subject: EvidenceSubject,
        claim: Claim,
        producer: EvidenceProducer,
        artifact_refs: Vec<ArtifactId>,
        strength: EvidenceStrength,
        scope: EvidenceScope,
    ) -> Result<Evidence, VerifyError> {
        if artifact_refs.is_empty() && reads_an_artifact(evidence_type) {
            return Err(VerifyError::EvidenceWithoutArtifact { evidence_type });
        }
        if evidence_type == EvidenceType::HumanConfirmation
            && !matches!(producer, EvidenceProducer::Human)
        {
            return Err(VerifyError::HumanConfirmationFromNonHuman);
        }
        // 八个私有字段**只在构造点的这一处**初始化——`Evidence` 没有第二个构造点。
        // 本条的可拍形态是编译期的（Task 10 的两份 `compile_fail` 样例，见 `Evidence`
        // 的文档）；`tests/evidence.rs` 的 `the_two_subjects_are_distinguishable` 把
        // `subject` 那一行的取反档写成红条件。
        Ok(Evidence {
            id,
            evidence_type,
            subject,
            claim,
            producer,
            artifact_refs,
            strength,
            scope,
        })
    }

    /// §258 的 `id`。
    pub fn id(&self) -> &EvidenceId {
        &self.id
    }

    /// §258 的 `type`（字段名不叫 `type`——Rust 保留字；与 `Artifact.artifact_type` 同形）。
    pub fn evidence_type(&self) -> EvidenceType {
        self.evidence_type
    }

    /// §258 的 `subject`。
    pub fn subject(&self) -> &EvidenceSubject {
        &self.subject
    }

    /// §258 的 `claim`。
    pub fn claim(&self) -> &Claim {
        &self.claim
    }

    /// §258 的 `producer`。
    pub fn producer(&self) -> &EvidenceProducer {
        &self.producer
    }

    /// §258 的 `artifact_refs`。
    pub fn artifact_refs(&self) -> &[ArtifactId] {
        &self.artifact_refs
    }

    /// §258 的 `strength`。
    pub fn strength(&self) -> &EvidenceStrength {
        &self.strength
    }

    /// §258 的 `scope`。
    pub fn scope(&self) -> &EvidenceScope {
        &self.scope
    }
}

/// §4.4 拒绝表第 1 行的判据：该型证据的判定是否需要读某个制品。
///
/// 十四臂**逐臂写死**、穷尽 `match`、**无通配臂**：`EvidenceType` 加一臂时本函数
/// **编译不过**，新臂不会悄悄落进「不需要制品」那一支（设计 §4.4）。
///
/// 需要制品的十三型：这十三型的判定都要读某个制品——§29 的 4K 例子（读 resolution
/// metadata）是 `MetadataCheck` 的实例；`ModelReview` / `ContradictionCheck` /
/// `CitationVerification` 三臂读被审查或被核验的制品（设计 §4.4 末段）。
///
/// 唯一不需要制品的是 `HumanConfirmation`（§258 的 `HUMAN_CONFIRMATION`）：人的确认
/// 无物可读。空引用的判据是 `artifact_refs.is_empty()`——「无物可读」必须走到 `Unknown`、
/// 不能走到 `Pass`（§7.5），故这一条放在构造点，不让它悄悄变成一次空判。
fn reads_an_artifact(evidence_type: EvidenceType) -> bool {
    match evidence_type {
        EvidenceType::HumanConfirmation => false,
        EvidenceType::Test => true,
        EvidenceType::Fuzz => true,
        EvidenceType::Property => true,
        EvidenceType::FormalProof => true,
        EvidenceType::StaticAnalysis => true,
        EvidenceType::Benchmark => true,
        EvidenceType::VisualCheck => true,
        EvidenceType::MetadataCheck => true,
        EvidenceType::Differential => true,
        EvidenceType::Metamorphic => true,
        EvidenceType::ModelReview => true,
        EvidenceType::ContradictionCheck => true,
        EvidenceType::CitationVerification => true,
    }
}
