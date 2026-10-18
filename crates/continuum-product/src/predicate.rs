//! §4.6 的 `ShippablePredicate`：至少一维、至少一条子句。
//!
//! 谓词是**发布条件**；判定是结果（`ShippableVerdict`）；相位是生命周期
//! （`ProductPhase`）。三个词不混用。P7A 不评估子句内容。

use continuum_canonical::Opaque;

use crate::error::ProductError;
use crate::readiness::ReadinessDimensionName;

#[derive(Debug, Clone)]
pub struct ShippablePredicate {
    required_dimensions: Vec<ReadinessDimensionName>,
    clauses: Vec<ShippableClause>,
}

#[derive(Debug, Clone)]
pub struct ShippableClause {
    id: ShippableClauseId,
    statement: Opaque,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShippableClauseId(String);

impl ShippableClauseId {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 空串也返回 `Self`：空校验在 `ShippableClause::new`。
    pub fn from_str(s: &str) -> Self {
        Self(s.to_owned())
    }
}

impl ShippableClause {
    pub fn new(id: ShippableClauseId, statement: Opaque) -> Result<Self, ProductError> {
        if id.as_str().is_empty() {
            return Err(ProductError::EmptyClauseId);
        }
        if statement.as_str().is_empty() {
            return Err(ProductError::EmptyClauseStatement {
                id: id.as_str().to_owned(),
            });
        }
        Ok(Self { id, statement })
    }

    pub fn id(&self) -> &ShippableClauseId {
        &self.id
    }

    pub fn statement(&self) -> &Opaque {
        &self.statement
    }
}

impl ShippablePredicate {
    /// 校验顺序：空维度／空子句 ⇒ `EmptyShippablePredicate`；重复维度 ⇒
    /// `DuplicateDimension`；重复 clause id ⇒ `DuplicateClause`。
    pub fn new(
        required_dimensions: Vec<ReadinessDimensionName>,
        clauses: Vec<ShippableClause>,
    ) -> Result<Self, ProductError> {
        if required_dimensions.is_empty() || clauses.is_empty() {
            return Err(ProductError::EmptyShippablePredicate);
        }
        for (i, dimension) in required_dimensions.iter().enumerate() {
            if required_dimensions[..i].contains(dimension) {
                return Err(ProductError::DuplicateDimension {
                    dimension: dimension.as_str(),
                });
            }
        }
        for (i, clause) in clauses.iter().enumerate() {
            if clauses[..i].iter().any(|prev| prev.id == clause.id) {
                return Err(ProductError::DuplicateClause {
                    id: clause.id.as_str().to_owned(),
                });
            }
        }
        Ok(Self {
            required_dimensions,
            clauses,
        })
    }

    pub fn required_dimensions(&self) -> &[ReadinessDimensionName] {
        &self.required_dimensions
    }

    pub fn clauses(&self) -> &[ShippableClause] {
        &self.clauses
    }
}
