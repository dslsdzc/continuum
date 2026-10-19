//! §4.7 的 `ContinuousProductDrive`、`ProductPhase` 与只前进的纯相位机。
//!
//! 相位是**产品级标签**：进入 `PostShippableImprovement` 只能经 Task 16（C2 后）的
//! `begin_post_shippable_work`，本模块的 `advance_product_phase` 从不返回它。

use crate::shippable::ShippableVerdict;

/// §336 的用户开关。默认 false；只在 Product 创建后改变。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContinuousProductDrive {
    pub enabled: bool,
}

impl Default for ContinuousProductDrive {
    fn default() -> Self {
        Self { enabled: false }
    }
}

/// §155／§336 的生命周期相位。三值互斥；**只前进，不回退**。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProductPhase {
    PreShippable,
    Shippable,
    PostShippableImprovement,
}

impl ProductPhase {
    /// 落库编码的逆（小写、多词下划线）；认不出就是 `None`。
    pub(crate) fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "pre_shippable" => Self::PreShippable,
            "shippable" => Self::Shippable,
            "post_shippable_improvement" => Self::PostShippableImprovement,
            _ => return None,
        })
    }
}

/// 开关＋相位；与 §231 的 Product 分开存（`product_drive` 表），
/// **不是 §231 的第八个字段**。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProductDriveState {
    pub continuous_product_drive: ContinuousProductDrive,
    pub phase: ProductPhase,
}

impl Default for ProductDriveState {
    fn default() -> Self {
        Self {
            continuous_product_drive: ContinuousProductDrive::default(),
            phase: ProductPhase::PreShippable,
        }
    }
}

/// 只前进：`PostShippableImprovement` 与 `Shippable` 不被任何 verdict 覆盖；
/// `PreShippable` 只在 `Shippable` 时前进。
///
/// 函数体内没有 `PostShippableImprovement` 的返回值：进入 Post 只能经
/// Task 16（C2 后）的 `begin_post_shippable_work`。
pub fn advance_product_phase(
    current: ProductPhase,
    verdict: &ShippableVerdict,
) -> ProductPhase {
    match (current, verdict) {
        (ProductPhase::PostShippableImprovement, _) => {
            ProductPhase::PostShippableImprovement
        }
        (ProductPhase::Shippable, _) => ProductPhase::Shippable,
        (ProductPhase::PreShippable, ShippableVerdict::Shippable) => {
            ProductPhase::Shippable
        }
        (ProductPhase::PreShippable, _) => ProductPhase::PreShippable,
    }
}
