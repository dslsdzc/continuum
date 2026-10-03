//! Artifact 数据模型（§240）与双身份（ENG-003）。

use crate::content::ContentHash;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// `§239` 的类型集合。枚举外的类型不可表达。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactType {
    SourceTree,
    Patch,
    TestResult,
    Text,
    Json,
    Blob,
}

/// `§243`。
///
/// 字符串名在 [`PrivacyClass::as_str`] / [`PrivacyClass::parse`] 上，是该编码唯一
/// 的产生点：库列（`crate::persist`）与策略条件的取值（`continuum-policy`）都取用它。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrivacyClass {
    Public,
    Personal,
    Private,
    Secret,
    LocalOnly,
}

impl PrivacyClass {
    /// 全部五档。供全量遍历的用例使用（与 `continuum_effect::EffectType::ALL` 同形）。
    ///
    /// 完整性与数目由 `tests/privacy_class.rs` 的
    /// `every_privacy_class_round_trips_through_its_encoding` 把关。
    pub const ALL: [PrivacyClass; 5] = [
        PrivacyClass::Public,
        PrivacyClass::Personal,
        PrivacyClass::Private,
        PrivacyClass::Secret,
        PrivacyClass::LocalOnly,
    ];

    /// `§243` 各档的字符串名：小写、多词以 `_` 连接。
    ///
    /// **本函数是该编码唯一的产生点**。match 穷尽且无通配臂：加档时本函数编译不过，
    /// 编码不会漏分支。
    ///
    /// 与派生出来的 serde 表示的关系：`#[serde(rename_all = "snake_case")]` 给出的
    /// 字符串与本函数**逐档相同**，故问题不是「两套表示」而是「两条读写路径」——
    /// 派生若被用上，同一条编码就有了第二个产生点，改一处不会让另一处失败。
    /// 至今本 crate 没有任何路径序列化 `PrivacyClass`（`Artifact` 的 serde 派生
    /// 只用于其 `metadata` / `provenance` 两个 `Value` 字段），故这条隐患尚未落地；
    /// 派生本身不在本次改动范围内，据实记录。
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Personal => "personal",
            Self::Private => "private",
            Self::Secret => "secret",
            Self::LocalOnly => "local_only",
        }
    }

    /// [`PrivacyClass::as_str`] 的**严格逆**：对 [`PrivacyClass::ALL`] 里的每档都有
    /// `PrivacyClass::parse(p.as_str()) == Some(p)`，表外字符串一律 `None`。
    ///
    /// `None` 而不是默认档次：隐私等级是策略裁决的依据，取默认会让策略表漏判。
    ///
    /// 解码侧没有穷尽 match 的保护（来源是 `&str` 而非枚举），故由
    /// `tests/privacy_class.rs` 的 `every_privacy_class_round_trips_through_its_encoding`
    /// 遍历 [`PrivacyClass::ALL`] 兜住。
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "public" => Self::Public,
            "personal" => Self::Personal,
            "private" => Self::Private,
            "secret" => Self::Secret,
            "local_only" => Self::LocalOnly,
            _ => return None,
        })
    }
}

/// 图引用与版本身份。稳定，不随内容变化。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ArtifactId(String);

impl ArtifactId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for ArtifactId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Artifact {
    pub id: ArtifactId,
    pub artifact_type: ArtifactType,
    pub content_hash: ContentHash,
    pub size: u64,
    pub producer_node: Option<String>,
    pub input_artifacts: Vec<ArtifactId>,
    pub metadata: Value,
    pub provenance: Value,
    pub privacy_class: PrivacyClass,
    pub version: u32,
}
