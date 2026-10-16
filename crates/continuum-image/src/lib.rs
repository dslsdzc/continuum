//! 图像领域算子（设计 §1.1）：编辑区域的四种表示与 `ImageError`。
//! 本子项目只声明算子与判定，不建执行路径（设计 §1.2）。

pub mod error;
pub mod region;

pub use error::ImageError;
pub use region::{
    EditMask, EditRegion, EditRegionKind, NormalizedAnchor, NormalizedBox, SemanticObjectRef,
};
