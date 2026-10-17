//! `resolve_edit_region` 与 `RegionDetector`（§325 的唯一落点；设计 §4.2／§4.3）。
//!
//! **本文件只给 §325 的 MUST 一个落点与一个 fail-closed 的边界**：§4.3 表里那五条
//! 判据缺口（选取规则 / 置信度门槛 / 零检出处置 / 区域上限 / `SemanticObject` 的名字空间）
//! **一条都不在这里被填**（逐条见 [`resolve_edit_region`] 的文档注释与 `## 遗留` 第一节）。

use continuum_artifact::ArtifactId;

use crate::error::ImageError;
use crate::region::{
    EditMask, EditRegion, EditRegionKind, NormalizedAnchor, NormalizedBox, SemanticObjectRef,
};

/// §164 `:682-691` 的 Object / Region Detection 的**接口**。本块定义它，实现在 backend 一侧
/// （第 3 层选定的后端）。
///
/// **返回值的形状**：`Option<EditMask>`——`None` 表示「这一支找不到区域」，
/// `resolve_edit_region` 把它变成 `Err(RegionUnresolved { .. })`。
///
/// **两个方法的形状、参数与这个名字都是本计划自定的取值**：设计 §2.2 只说「本块定义它，
/// 其实现在 backend 一侧」，§4.2 只说「`detector` 的返回值就是掩码位图（`EditMask`）」，
/// **没有给方法名、参数与返回类型**。依据与代价见 `## 遗留` 第一节第 3 条：
/// 两个方法收 `source` 与 `source_dims`——**收 `source` 是必需的**，否则 `resolve_edit_region`
/// 的 `source` 实参在函数体内没有用途（一个死形参），而 §4.2 的签名里它有。
/// 两处都要归一化坐标，故两处都要画布尺寸（§4.2：「三处都要源图的画布尺寸」）。
pub trait RegionDetector {
    /// §164 `:684`：把一个**归一化点**解析成该点所在的语义区域。
    fn region_at(
        &self,
        source: &ArtifactId,
        source_dims: (u32, u32),
        point: &NormalizedAnchor,
    ) -> Option<EditMask>;

    /// §166 `:751` / `:769`：按**名字**找该对象的掩码。
    fn region_named(
        &self,
        source: &ArtifactId,
        source_dims: (u32, u32),
        name: &SemanticObjectRef,
    ) -> Option<EditMask>;
}

/// §325 `:2360-2366` 的唯一落点：把四种编辑区域表示解析成 §166 `:775` 的 `EditMask`。
///
/// **`source_dims` 是必需参数**：`Box` 支要把归一化矩形投影成像素掩码，而投影的坐标系就是
/// 源图的画布；`Point` 与 `SemanticObject` 两支也要把它交给 detector。**本函数不自己读**：
/// 从制品读尺寸要经 `Artifact.metadata`（那份 schema 规范未给，§13 第 5 条），
/// 且那是一次 IO，而本块不做 IO（§3.1）。
/// **`detector` 是必需参数**：`Point` 与 `SemanticObject` 两支都要经对象/区域检测
/// （§164 `:684`），故本函数**没有**任何一条「不传 detector 也能把 `Point` 变成掩码」的路径。
///
/// **本函数的四支共用同一个入口、同一个返回类型**——这是 §4.4 所说「本 crate 里没有第二条从
/// `Point` 到 `EditMask` 的路」的兑现方式。**这一句话的射程**：它只说「本 crate 里没有绕开的路」，
/// 说不了「别的 crate 不会自己写一个 Point→整图的转换」（后者没有落点）。
///
/// **本函数不核 detector 返回的掩码与 `source_dims` 是否一致**：§4.2 的表没有这条前置，
/// 故尺寸不符的掩码会 `Ok` 通过区域解析，直到 `composite_masked` 才被 `ShapeMismatch` 拦住。
/// 于是「区域解析成功」与「掩码可用于合成」是两件事，**只有前者在本函数的返回类型里**
/// （计划遗留第一节第 9 条）。
///
/// **四支各自的判据照 §4.2 的表**（本计划不重抄该表）：`Box` 投影 / `Mask` 原样 /
/// `SemanticObject` 经 detector 按名字 / `Point` **必须**经 detector 找语义区域、
/// **不得**退化成单像素。其中 `Box` 支的投影规则、取整规则与**越界夹取**都是本计划自定的
/// 取值（见 [`project_box`]）。
///
/// **§4.3 的处置（逐条照录，一条都不发明）**：
///
/// - **规范给了什么**：MUST（§325 `:2360`）、机制（§164 `:684` 的 Object / Region Detection）、
///   一个例子（§164 `:699-705` 的「hand + held object」）、坐标约定（§163 `:648`）。
/// - **规范没给什么（五条，逐条给出「规范里最接近的」与「为什么它不够」）**：
///   ①「点落在多个对象上时取哪一个」的判据（最接近的是 §164 `:699-705` 的
///   `hand + held object`，那是一例、不是选取规则）；②检测的**置信度阈值**与低于阈值时的
///   处置（规范全文没有置信度这个量）；③**零检出**时的处置（无）；④区域**大小的上限**
///   （最接近的是 §324 `:2343` 的 `preserve_outside_region = true`，它说的是 mask 外不变、
///   不是 mask 能有多大）；⑤`SemanticObject` 的**名字**从哪来（最接近的是 §166 `:769` 的
///   「左边这个人」，那是一句自然语言、不是名字空间）。
/// - **本设计取的那一条**：**零检出 ⇒ `Err(RegionUnresolved)`（fail-closed），不放大、
///   不退化、不请模型猜**。理由：§325 的 MUST 是**否定式**的（「而不是单像素修改」），
///   故最小的合法实现是「解析不出就拒绝」；放大到整图会让 §326 的
///   `outside_mask_change = FORBIDDEN` 变得无意义（mask 外为空集，任何改动都「合法」），
///   退化到单像素正被 §325 明文禁止。
/// - **代价（不写这一条，就只写了落点与边界）**：**零检出即 `Err` 的直接代价是合法点击
///   会被拒**。误检、稀疏对象、点击落在对象边缘之外（§164 `:699-705` 的
///   「hand + held object」这类多对象区域尤其）都会走到这一支；而
///   `ImageError::RegionUnresolved` 是**终止性的**——它不重试、不放宽、不给近似区域，
///   故每一次误拒都要**用户重选**（或重跑 detector）才能继续。取这个代价的理由是两侧
///   不对称：**误拒是一次可恢复的往返**，而「放大到整图」会让 §326 的判定失去意义。
///   **这一条不声称代价小**，只声称它比另一侧的代价小。
/// - **本条不解决什么**：本函数只是**给 §325 的 MUST 一个落点与一个 fail-closed 的边界**，
///   **没有让「Point → 语义区域」这件事变得有判据**；上表五条**仍在**。
/// - **本处无照片（写明）**：五条缺口都是**判据的边界**，而边界没有判据——故这里拍得到的
///   只有「桩的返回被原样传出去」与「空返回即 `Err`」两条（`tests/resolve_region.rs`），
///   **拍不到「区域选得对不对」**。
///
/// **`Box` 支的越界处置（本计划自定，见计划遗留第一节第 4 条与第五节）**：`NormalizedBox`
/// 的一个角越出 `[0, 1]`（§163 `:648` 的约定）时**夹取到画布**，而不是新增一枚错误臂——
/// `ImageError` 的四臂里**没有**一枚表达「坐标越界」（`RegionUnresolved` 说的是
/// 「解析不出区域」、`ShapeMismatch` 说的是「两载体形状不同」），而**臂数是设计 §3.1
/// 钉死的四臂**。**代价**：越界输入被静默地当成「贴边」；**收益**：不新增一枚设计没有的臂，
/// 也不 panic。§163 给了 `x, y ∈ [0, 1]` 的约定而**没有给越界输入的处置**——这一处是本计划
/// 补的，**不是规范给的**。
pub fn resolve_edit_region(
    region: &EditRegion,
    source: &ArtifactId,
    source_dims: (u32, u32),
    detector: &dyn RegionDetector,
) -> Result<EditMask, ImageError> {
    match region {
        EditRegion::Point(point) => {
            detector
                .region_at(source, source_dims, point)
                .ok_or(ImageError::RegionUnresolved {
                    region: EditRegionKind::Point,
                })
        }
        EditRegion::Box(bounding) => Ok(project_box(bounding, source_dims)),
        EditRegion::Mask(mask) => Ok(mask.clone()),
        EditRegion::SemanticObject(object) => detector
            .region_named(source, source_dims, object)
            .ok_or(ImageError::RegionUnresolved {
                region: EditRegionKind::SemanticObject,
            }),
    }
}

/// `Box` 支的投影（**无模型**；§4.2 的表——四支里**唯一不调 detector 的一支**）。
///
/// **规则三步，第 2、3 步的取值都是本计划自定**（设计 §4.2 只说「归一化矩形 ×
/// `source_dims` → 掩码（投影，无模型）」，**没有给取整规则与越界处置**）：
///
/// 1. **逐维取 min/max**：`NormalizedBox` 的两角名字不表达几何次序（§166 没给
///    「哪一角是左上」，见 `region.rs` 里该型的注），故两角对调给出同一个掩码；
/// 2. **夹取到 `[0, 1]`**：越界处置的比照（为什么夹取而不新增一枚错误臂、代价与收益）
///    写在 [`resolve_edit_region`] 的文档注释里；
/// 3. **低边 floor、高边 ceil 的像元取交**：第 `col` 列被置位 ⇔
///    `col ∈ [floor(x0 × 宽), ceil(x1 × 宽))`，行同理。取整规则也是本计划自定的取值——
///    它使任何**正面积**的矩形至少置位一个像素（归一化宽度 > 0 时
///    `ceil(高边 × 宽) > floor(低边 × 宽)`），且「低边含、高边不含」与夹取后的 `[0, 1]`
///    合成一条「矩形闭包覆盖到的像元都被置位」的语义。
///
/// 置位值取 `1`、其余取 `0`；掩码的宽高就是 `source_dims`（本函数不改画布），
/// 故不变量 `pixels.len() == 宽 as usize × 高 as usize` 在返回值上成立。
fn project_box(bounding: &NormalizedBox, source_dims: (u32, u32)) -> EditMask {
    let (width, height) = source_dims;

    // 第 1、2 步：逐维 min/max（两角对调不改变结果），再夹取到单位区间。
    let x0 = bounding.p.x.min(bounding.q.x).clamp(0.0, 1.0);
    let x1 = bounding.p.x.max(bounding.q.x).clamp(0.0, 1.0);
    let y0 = bounding.p.y.min(bounding.q.y).clamp(0.0, 1.0);
    let y1 = bounding.p.y.max(bounding.q.y).clamp(0.0, 1.0);

    // 第 3 步：像元取交（低边 floor、高边 ceil，右端开）。夹取后的四值都在 [0, 1]，
    // 故两个端点都不会越出画布。
    let col_start = (x0 * f64::from(width)).floor() as u32;
    let col_end = (x1 * f64::from(width)).ceil() as u32;
    let row_start = (y0 * f64::from(height)).floor() as u32;
    let row_end = (y1 * f64::from(height)).ceil() as u32;

    let mut pixels = vec![0u8; width as usize * height as usize];
    for row in row_start..row_end {
        let row_offset = row as usize * width as usize;
        for col in col_start..col_end {
            pixels[row_offset + col as usize] = 1;
        }
    }

    EditMask {
        width,
        height,
        pixels,
    }
}
