//! §150 `EngineeringBaseline`（十字段）与生成端口。
//!
//! 字段名逐字、全 `pub`；本类型没有比 §150 更强的约束可挂。比较由消费方按 `as_str` 做
//! （`Opaque` 没有 `PartialEq`）。

use continuum_canonical::Opaque;

use crate::error::{PortError, ProductError};

/// §150 的十个字段。
#[derive(Debug, Clone)]
pub struct EngineeringBaseline {
    pub product_goal: Opaque,
    pub target_users: Vec<Opaque>,
    pub required_capabilities: Vec<Opaque>,
    pub hard_constraints: Vec<Opaque>,
    pub architecture: Opaque,
    pub technology_choices: Vec<Opaque>,
    pub minimum_complete_system: Vec<Opaque>,
    pub known_unknowns: Vec<Opaque>,
    pub engineering_risks: Vec<Opaque>,
    pub acceptance_criteria: Vec<Opaque>,
}

/// 生成端口的输入：只带产品意图的文本与已知约束；具体提示词不在 P7A。
#[derive(Debug, Clone)]
pub struct BaselineRequest {
    pub idea: Opaque,
    pub constraints: Vec<Opaque>,
}

/// §150 的模型判断端口。P7A 定义 trait；装配方实现并转调既有模型调用路径。
/// **P7A 不依赖 provider／runtime。**
#[async_trait::async_trait]
pub trait EngineeringBaselineGenerator {
    async fn generate(&self, request: &BaselineRequest) -> Result<EngineeringBaseline, PortError>;
}

impl EngineeringBaseline {
    /// 基线可用性：`product_goal` 与 `minimum_complete_system` 非空。
    ///
    /// **不判**工程好坏、不判 `target_users` 等其余字段。
    #[allow(dead_code)]
    pub(crate) fn is_usable(&self) -> Result<(), ProductError> {
        if self.product_goal.as_str().is_empty() {
            return Err(ProductError::BaselineUnusable {
                missing: "product_goal",
            });
        }
        if self.minimum_complete_system.is_empty() {
            return Err(ProductError::BaselineUnusable {
                missing: "minimum_complete_system",
            });
        }
        Ok(())
    }
}
