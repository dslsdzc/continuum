//! 图像领域算子（设计 §1.1）：五枚算子的声明、注册入口、编辑区域的四种表示与解析入口
//! （`resolve_edit_region`）、`ImageError`。
//! 本子项目只声明算子与判定，不建执行路径（设计 §1.2）。

pub mod error;
pub mod operators;
pub mod region;
pub mod register;
pub mod resolve;

pub use error::ImageError;
pub use operators::{V01_OPERATOR_IDS, all_operators};
pub use region::{
    EditMask, EditRegion, EditRegionKind, NormalizedAnchor, NormalizedBox, SemanticObjectRef,
};
pub use register::register_image_operators;
pub use resolve::{RegionDetector, resolve_edit_region};
