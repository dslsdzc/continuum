//! §232 的 `ProductReadiness`：九维、每维 `value/evidence/last_verified` 三元组。
//!
//! **没有聚合**：`score()`／`average()`／`sum()`／`aggregate()` 在类型上不存在。
//! **不派生 `PartialEq`**：`Assessed(Opaque)` 里的 `Opaque` 没有 `PartialEq`。

use continuum_canonical::Opaque;
use continuum_verify::EvidenceId;

use crate::error::ProductError;

/// §232 的九个维度。字段名逐字；顺序照规范。
#[derive(Debug, Clone)]
pub struct ProductReadiness {
    functional: ReadinessDimension,
    correctness: ReadinessDimension,
    reliability: ReadinessDimension,
    performance: ReadinessDimension,
    security: ReadinessDimension,
    usability: ReadinessDimension,
    maintainability: ReadinessDimension,
    deployability: ReadinessDimension,
    documentation: ReadinessDimension,
}

/// 一维的记录。字段私有；`assessed` 是唯一能造出「有值」的构造点。
///
/// 只读三元组必须经 `value()`／`evidence()`／`last_verified()` 取——这是
/// `MilestoneGenerator`（gap analysis）与 `ProgressPolicy`（delta）的唯一输入面。
#[derive(Debug, Clone)]
pub struct ReadinessDimension {
    value: ReadinessValue,
    evidence: Vec<EvidenceId>,
    last_verified: Option<i64>,
}

#[derive(Debug, Clone)]
pub enum ReadinessValue {
    /// 尚未评估。`evidence` 必空、`last_verified` 必 `None`。
    Unassessed,
    /// 已评估。值是不透明串（量纲由调用方的策略解释）；`evidence` 非空、
    /// `last_verified` 为 `Some`。
    Assessed(Opaque),
}

impl ReadinessValue {
    /// 只读判别：`Unassessed` ⇒ true；`Assessed` ⇒ false。
    pub fn is_unassessed(&self) -> bool {
        matches!(self, Self::Unassessed)
    }

    /// 只读判别：`Assessed(v)` ⇒ `Some(v)`；`Unassessed` ⇒ `None`。
    ///
    /// **不提供 `as_f64`／`score`**：量纲由调用方解释，P7A 不比较。
    pub fn as_assessed(&self) -> Option<&Opaque> {
        match self {
            Self::Assessed(v) => Some(v),
            Self::Unassessed => None,
        }
    }
}

impl ReadinessDimension {
    pub fn unassessed() -> Self {
        Self {
            value: ReadinessValue::Unassessed,
            evidence: Vec::new(),
            last_verified: None,
        }
    }

    /// 唯一的「有值」构造点。校验顺序：空 value ⇒ `EmptyReadinessValue`；
    /// 空 evidence ⇒ `ReadinessValueWithoutEvidence`；重复 evidence ⇒
    /// `DuplicateEvidence`。
    ///
    /// `assessed` 的签名不含维度名，故错误里的 `dimension` 写 `"<unbound>"`；
    /// 绑定维度后的真实名字由 `record_milestone_progress`／`load_product` 重验。
    pub fn assessed(
        value: Opaque,
        evidence: Vec<EvidenceId>,
        last_verified: i64,
    ) -> Result<Self, ProductError> {
        if value.as_str().is_empty() {
            return Err(ProductError::EmptyReadinessValue {
                dimension: "<unbound>",
            });
        }
        if evidence.is_empty() {
            return Err(ProductError::ReadinessValueWithoutEvidence {
                dimension: "<unbound>",
            });
        }
        for (i, e) in evidence.iter().enumerate() {
            if evidence[..i].iter().any(|prev| prev == e) {
                return Err(ProductError::DuplicateEvidence {
                    id: e.as_str().to_owned(),
                });
            }
        }
        Ok(Self {
            value: ReadinessValue::Assessed(value),
            evidence,
            last_verified: Some(last_verified),
        })
    }

    pub fn value(&self) -> &ReadinessValue {
        &self.value
    }

    pub fn evidence(&self) -> &[EvidenceId] {
        &self.evidence
    }

    pub fn last_verified(&self) -> Option<i64> {
        self.last_verified
    }
}

/// 九维的名字；用于 required_dimensions、迭代与错误消息。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadinessDimensionName {
    Functional,
    Correctness,
    Reliability,
    Performance,
    Security,
    Usability,
    Maintainability,
    Deployability,
    Documentation,
}

impl ReadinessDimensionName {
    /// 落库／错误消息用的 §232 名字。`pub(crate)`：`as_str`／`parse` 与类型同址。
    /// Task 2 起用于谓词校验；Task 10 起用于库行编解码（C2 后）。
    #[allow(dead_code)]
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Functional => "functional",
            Self::Correctness => "correctness",
            Self::Reliability => "reliability",
            Self::Performance => "performance",
            Self::Security => "security",
            Self::Usability => "usability",
            Self::Maintainability => "maintainability",
            Self::Deployability => "deployability",
            Self::Documentation => "documentation",
        }
    }

    /// [`Self::as_str`] 的逆；认不出就是 `None`。
    #[allow(dead_code)]
    pub(crate) fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "functional" => Self::Functional,
            "correctness" => Self::Correctness,
            "reliability" => Self::Reliability,
            "performance" => Self::Performance,
            "security" => Self::Security,
            "usability" => Self::Usability,
            "maintainability" => Self::Maintainability,
            "deployability" => Self::Deployability,
            "documentation" => Self::Documentation,
            _ => return None,
        })
    }
}

impl ProductReadiness {
    pub fn unassessed() -> Self {
        Self {
            functional: ReadinessDimension::unassessed(),
            correctness: ReadinessDimension::unassessed(),
            reliability: ReadinessDimension::unassessed(),
            performance: ReadinessDimension::unassessed(),
            security: ReadinessDimension::unassessed(),
            usability: ReadinessDimension::unassessed(),
            maintainability: ReadinessDimension::unassessed(),
            deployability: ReadinessDimension::unassessed(),
            documentation: ReadinessDimension::unassessed(),
        }
    }

    pub fn dimension(&self, name: ReadinessDimensionName) -> &ReadinessDimension {
        match name {
            ReadinessDimensionName::Functional => &self.functional,
            ReadinessDimensionName::Correctness => &self.correctness,
            ReadinessDimensionName::Reliability => &self.reliability,
            ReadinessDimensionName::Performance => &self.performance,
            ReadinessDimensionName::Security => &self.security,
            ReadinessDimensionName::Usability => &self.usability,
            ReadinessDimensionName::Maintainability => &self.maintainability,
            ReadinessDimensionName::Deployability => &self.deployability,
            ReadinessDimensionName::Documentation => &self.documentation,
        }
    }

    pub fn with_dimension(self, name: ReadinessDimensionName, d: ReadinessDimension) -> Self {
        let mut next = self;
        match name {
            ReadinessDimensionName::Functional => next.functional = d,
            ReadinessDimensionName::Correctness => next.correctness = d,
            ReadinessDimensionName::Reliability => next.reliability = d,
            ReadinessDimensionName::Performance => next.performance = d,
            ReadinessDimensionName::Security => next.security = d,
            ReadinessDimensionName::Usability => next.usability = d,
            ReadinessDimensionName::Maintainability => next.maintainability = d,
            ReadinessDimensionName::Deployability => next.deployability = d,
            ReadinessDimensionName::Documentation => next.documentation = d,
        }
        next
    }

    pub fn iter(&self) -> [(ReadinessDimensionName, &ReadinessDimension); 9] {
        [
            (
                ReadinessDimensionName::Functional,
                &self.functional,
            ),
            (
                ReadinessDimensionName::Correctness,
                &self.correctness,
            ),
            (
                ReadinessDimensionName::Reliability,
                &self.reliability,
            ),
            (
                ReadinessDimensionName::Performance,
                &self.performance,
            ),
            (ReadinessDimensionName::Security, &self.security),
            (ReadinessDimensionName::Usability, &self.usability),
            (
                ReadinessDimensionName::Maintainability,
                &self.maintainability,
            ),
            (
                ReadinessDimensionName::Deployability,
                &self.deployability,
            ),
            (
                ReadinessDimensionName::Documentation,
                &self.documentation,
            ),
        ]
    }
}
