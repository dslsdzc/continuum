//! Operator 注册表。

use crate::definition::{Operator, OperatorId, OperatorVersion};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OperatorError {
    #[error("Operator {id} 版本 {version:?} 未注册")]
    NotFound { id: OperatorId, version: OperatorVersion },
    #[error("Operator {id} 版本 {version:?} 已注册")]
    Duplicate { id: OperatorId, version: OperatorVersion },
}

#[derive(Debug, Default)]
pub struct OperatorRegistry {
    entries: HashMap<(OperatorId, OperatorVersion), Operator>,
}

impl OperatorRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, operator: Operator) -> Result<(), OperatorError> {
        let key = (operator.id.clone(), operator.version);
        if self.entries.contains_key(&key) {
            return Err(OperatorError::Duplicate {
                id: operator.id.clone(),
                version: operator.version,
            });
        }
        self.entries.insert(key, operator);
        Ok(())
    }

    pub fn resolve(
        &self,
        id: &OperatorId,
        version: &OperatorVersion,
    ) -> Result<&Operator, OperatorError> {
        self.entries
            .get(&(id.clone(), *version))
            .ok_or_else(|| OperatorError::NotFound {
                id: id.clone(),
                version: *version,
            })
    }
}
