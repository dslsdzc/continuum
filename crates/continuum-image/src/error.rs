//! `ImageError`（设计 §3.1）。

use crate::region::EditRegionKind;
use continuum_operator::OperatorError;

/// 四臂。逐臂的判据持有者与理由见设计 §3.1，本计划不重抄。
/// **不为「backend 不支持掩码编辑」留臂**：那是一个声明面的事（`backend_candidates` 的取值），
/// 不是一次调用的失败。**不给 IO 臂**：本块的两个确定性函数收 `&Raster`，读写落盘是调用方的事。
#[derive(Debug, thiserror::Error)]
pub enum ImageError {
    /// 注册表拒绝（同 (id, version) 已存在）。判定的持有者是 P1（`OperatorError::Duplicate`）。
    /// 本块**只带出来、不重述**——另立一枚 `ImageError::Duplicate` 就是
    /// 「注册表里有没有这个算子」的第二套词汇表。
    #[error(transparent)]
    Registry(#[from] OperatorError),

    /// §325：该区域解析不出语义区域（§4.3）。判定的持有者在本块（`resolve_edit_region`）。
    #[error("区域 {region:?} 解析不出语义编辑区域（§325）")]
    RegionUnresolved { region: EditRegionKind },

    /// §326：参与逐点运算的像素载体形状不一致——逐点合成与比较无定义。
    /// 掩码是单通道位图，**没有色彩通道这一维**，故掩码那一侧的第三分量是 `None`。
    #[error("像素载体形状不一致：{left:?} 与 {right:?}（§326）")]
    ShapeMismatch {
        left: (u32, u32, Option<u8>),
        right: (u32, u32, Option<u8>),
    },

    /// 一个载体的 `pixels` 字节数与它自己声明的形状不符——该值不满足自己的不变量。
    /// **与 `ShapeMismatch` 分得开**：那一个比的是**两个载体之间**的形状，
    /// 这一个比的是**一个载体内部**的字节数与它自己声明的形状。
    #[error("`pixels` 长度 {len} 与声明的形状不符（期望 {expected}）")]
    PixelLengthMismatch { len: usize, expected: usize },
}
