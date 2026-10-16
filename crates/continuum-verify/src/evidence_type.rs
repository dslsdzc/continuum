//! §258 的 `EvidenceType`：十四臂与它的落库编码（设计 §4.2）。

use serde::{Deserialize, Serialize};

/// §258 的 `type` 字段（字段名不叫 `type`：Rust 保留字；与 `Artifact.artifact_type` 同形）。
///
/// **封闭枚举**：`VerificationProfile.required_evidence_types[]` 与产出的证据之间要做集合包含判定，
/// 开放字符串会把「必需类型齐了吗」退化成字符串比较，而漏判的方向是「以为齐了」（设计 §4.2）。
///
/// 字符串名在 [`EvidenceType::as_str`] / [`EvidenceType::parse`] 上，是该编码唯一的
/// 产生点（同 `continuum-artifact` 的 `ArtifactType`）：本 crate 的落库列（`evidence.evidence_type`，
/// Task 9）取用它，不取 serde 表示。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceType {
    Test,
    Fuzz,
    Property,
    FormalProof,
    StaticAnalysis,
    Benchmark,
    VisualCheck,
    MetadataCheck,
    HumanConfirmation,
    Differential,
    Metamorphic,
    ModelReview,
    ContradictionCheck,
    CitationVerification,
}

impl EvidenceType {
    /// 已实测的出处有十四行（设计 §4.2 的表）：§258 的九臂 ＋ §193 里 §258 未列的两臂
    /// ＋ §262/§330 报出的三臂。**本常量带数目字面量**，与 `ArtifactType::ALL`
    /// （`crates/continuum-artifact/src/artifact.rs:28`）同形。
    ///
    /// **不写「只有这十四臂」**：§258 的九臂前缀是「例如」，§193 的六种方式之外规范
    /// 没有穷尽声明。完整性与数目由 `tests/evidence_type.rs` 的
    /// `the_type_list_names_the_fourteen_arms_in_one_place` 把关。
    pub const ALL: [EvidenceType; 14] = [
        EvidenceType::Test,
        EvidenceType::Fuzz,
        EvidenceType::Property,
        EvidenceType::FormalProof,
        EvidenceType::StaticAnalysis,
        EvidenceType::Benchmark,
        EvidenceType::VisualCheck,
        EvidenceType::MetadataCheck,
        EvidenceType::HumanConfirmation,
        EvidenceType::Differential,
        EvidenceType::Metamorphic,
        EvidenceType::ModelReview,
        EvidenceType::ContradictionCheck,
        EvidenceType::CitationVerification,
    ];

    /// §258 各型的字符串名：小写、多词以 `_` 连接。
    ///
    /// **本函数是该编码唯一的产生点**。match 穷尽且无通配臂：加臂时本函数编译不过，
    /// 编码不会漏分支。
    ///
    /// 与派生出来的 serde 表示的关系同 `ArtifactType::as_str`：`rename_all =
    /// "snake_case"` 给出的字符串与本函数逐型相同，故隐患不是「两套表示」而是「两条
    /// 读写路径」。本 crate 的落库列一律经本函数与 [`EvidenceType::parse`] 读写，
    /// 不依赖 serde、不用 `Debug`（`Debug` 表示含大写）。
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Test => "test",
            Self::Fuzz => "fuzz",
            Self::Property => "property",
            Self::FormalProof => "formal_proof",
            Self::StaticAnalysis => "static_analysis",
            Self::Benchmark => "benchmark",
            Self::VisualCheck => "visual_check",
            Self::MetadataCheck => "metadata_check",
            Self::HumanConfirmation => "human_confirmation",
            Self::Differential => "differential",
            Self::Metamorphic => "metamorphic",
            Self::ModelReview => "model_review",
            Self::ContradictionCheck => "contradiction_check",
            Self::CitationVerification => "citation_verification",
        }
    }

    /// [`EvidenceType::as_str`] 的**严格逆**：对 [`EvidenceType::ALL`] 里的每型都有
    /// `EvidenceType::parse(t.as_str()) == Some(t)`，表外字符串一律 `None`。
    ///
    /// `None` 而不是默认型：落库列读出不认识的串说明库里有本版本不认得的证据类型，
    /// 取默认会把一份别的东西当成本型处理（理由同
    /// `crates/continuum-artifact/src/artifact.rs:61-63` 的注释）。
    ///
    /// 解码侧没有穷尽 match 的保护（来源是 `&str` 而非枚举），故由
    /// `tests/evidence_type.rs` 的 `every_evidence_type_round_trips_through_its_encoding`
    /// 遍历 [`EvidenceType::ALL`] 兜住。
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "test" => Self::Test,
            "fuzz" => Self::Fuzz,
            "property" => Self::Property,
            "formal_proof" => Self::FormalProof,
            "static_analysis" => Self::StaticAnalysis,
            "benchmark" => Self::Benchmark,
            "visual_check" => Self::VisualCheck,
            "metadata_check" => Self::MetadataCheck,
            "human_confirmation" => Self::HumanConfirmation,
            "differential" => Self::Differential,
            "metamorphic" => Self::Metamorphic,
            "model_review" => Self::ModelReview,
            "contradiction_check" => Self::ContradictionCheck,
            "citation_verification" => Self::CitationVerification,
            _ => return None,
        })
    }
}
