//! 两个确定性函数（设计 §5.2／§5.3）与它们的像素载体：`composite_masked`、
//! `outside_mask_difference`、`Raster`、`PixelDiff`。
//!
//! **本文件是 §326 在本块的兑现**：强制作（`composite_masked`——走过这一步的结果图
//! 按构造满足 §326）与检出（`outside_mask_difference`——确定性、不读模型自述）；
//! **类型层没有**（设计 §5.4，逐条写在两个公开函数的文档注释里）。
//! 两个函数都是**纯函数、无 IO、无执行路径**（切分 §四第 4 条只禁「执行路径」，
//! 不禁本块自己的判定函数；`ImageError` 也不给 IO 臂）。

use crate::error::ImageError;
use crate::region::EditMask;

/// 本块为两个确定性函数定的**最小图像像素载体**。
///
/// **本设计不定义像素格式**（设计 §5.2）：§168 `:826` 只给了「Alpha / 色彩空间是否异常」
/// 这一条**检查名**，没有给通道语义、色彩空间或 Alpha 的表示——故本型只承载
/// 「宽、高、通道数、字节序列」四件，**通道的语义由 backend 的约定决定**；
/// **缺的是「像素格式与色彩空间」这一步**（设计 §13 第 5 条）。
/// **这里不是「直接收 `Vec<u8>` 加两个 usize」**：三个量一起传时调用方可以传错配的对，
/// 而合成与比较**都要**这三个量一致才有意义；收成一个值，错配在类型上就只有一个来源。
/// **不变量**：`pixels.len() == width as usize * height as usize * channels as usize`
/// ——两个确定性函数在入口核它（`ImageError::PixelLengthMismatch`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Raster {
    pub width: u32,
    pub height: u32,
    pub channels: u8,
    pub pixels: Vec<u8>,
}

/// §168 `:832-835` 的 `outside-mask pixel difference` 的产物。
///
/// 设计只约束两句：「它必须含一个可以判 `== 0` 的差异量」与「返回 mask 外**不相等**的
/// 像素数与**总像素数**」（设计 §5.3）。**`differing` 这个字段名是设计给的**——设计 §11.2
/// 第 (l) 条的用例逐字写 `PixelDiff { differing: 0, .. }`（实测在设计 `:520`）；
/// **本计划自定的是它的整体形状与总数那一枚的名字**：设计未指明「总像素数」是整图还是
/// mask 外，**本计划取 mask 外**（理由与收件人见 `tests/pixels.rs` 的
/// `the_comparator_reports_the_difference_and_the_total` 与计划 `## 遗留` 第五节）。
///
/// 本型**不宣告裁决**：没有 `passed()` / `is_ok()` 一类判定方法——「差多少算不通过」的
/// 判据不在本块（设计 §5.3 的第二半、§7.3）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PixelDiff {
    /// mask 外**不相等**的像素数（逐像素计数：一个像素在两侧的通道字节序列不同即算一次）。
    pub differing: usize,
    /// **mask 外的像素数**（本计划自定的读法，不是整图的像素数，见上）。
    pub total: usize,
}

/// §326 `:2378-2388` 的硬合成：mask 外的像素从 `source` 逐点复制，mask 内的取自 `generated`。
/// 纯函数、无 IO、无执行路径（切分 §四第 4 条只禁「执行路径」，不禁本块自己的判定函数）。
///
/// **三条入口前置条件，逐条给出**：
/// 1. `(mask.width, mask.height) == (source.width, source.height)`；
/// 2. `generated` 与 `source` 的宽、高**与通道数**三者都相同；
/// 3. 三个载体的 `pixels` 长度与各自声明的形状相符。
/// 第 1、2 条不成立即 `ShapeMismatch`（掩码那一侧的第三分量是 `None`）；
/// 第 3 条不成立即 `PixelLengthMismatch`，**这一条不走 `ShapeMismatch`**。
/// 第 2 条与第 1 条同级：mask 内的像素取自 `generated`，逐点复制要求二者**逐像素一一
/// 对应**；尺寸或通道数不同时「mask 内取自 `generated`」这一步没有定义（设计 §5.2）。
/// **这里不是「尺寸不符时按左上角对齐凑合」**：§168 `:825` 的「分辨率是否保持」是一条
/// 独立的检查，合成时按坐标截取会让「掩码与源图不同坐标系」这件事在结果里不可见——
/// 而它是 §326 的前提。
///
/// **掩码的读法**：`mask.pixels` 的**非零**字节表示该像素在 mask 内（位图约定；
/// 掩码的编码格式本块不定义，设计 §13 第 5 条）。
///
/// **§5.4 的处置（逐条照录）**：
///
/// - **三层，逐层给出它保证到哪**：**强制作（使之合法）**＝`hard-composite`（Task 2 的第 4 行）
///   ——走过这一步的结果图**按构造**满足 §326；**检出（发现不合法）**＝`verify-outside-mask`
///   （第 5 行）与 [`outside_mask_difference`]——**确定性**，且**不读模型自述**
///   （§168 `:837-843` 逐字「这比让模型说『我只修改了手。』可靠得多」）；**类型层**＝**没有**。
/// - **类型层拦不住的依据**（设计 §5.4）：`ArtifactType`
///   （`crates/continuum-artifact/src/artifact.rs:14-21`）是**内容类型的枚举**，它没有、
///   也不该有「这张 `Image` 与那张 `Image` 在 mask 外逐点相同」这种**跨制品关系**的表达力。
///   故 §326 的 FORBIDDEN 是**校验**与**强制作**，不是类型约束；**本设计不为此向 P5e 要新变体**。
/// - **失配方向（这是本条要回答的那一问）**：
///   **A. 当 `preserve_outside_region = true` 的节点没有接上 `hard-composite` 时，不报错**
///   ——产出是一张 mask 外被改动过的 `Image`，它与其他 `Image` 在类型上**完全一样**，
///   只有跑了比较器才看得见。**这一侧是 fail-open。**
///   故 §326 在本块的兑现是「提供使之成立的步骤 ＋ 提供检出它的检查器」，**不是**「保证
///   它一定被用上」——后者要求「谁保证带这条参数的节点一定接了 `hard-composite`」，
///   而规范未给（§13 第 2 条）。
///   **B. 反向的一侧**：若 `hard-composite` 被接上但掩码与源图不同坐标系（或 `generated`
///   与源图形状不同）⇒ `Err(ShapeMismatch)`。**这一侧是 fail-closed**，故它**不能**用来
///   代替 A：它拦的是「掩码／补丁配错」，拦不了「根本没有合成」。
/// - **本处无照片（写明）**：拍得到的只有这两个函数的返回值；**拍不到「图里真有一段管线
///   没接上合成」**——那一侧的失配正是 fail-open 本身（`tests/pixels.rs` 与设计 §11.5 第 2 条）。
pub fn composite_masked(
    source: &Raster,
    generated: &Raster,
    mask: &EditMask,
) -> Result<Raster, ImageError> {
    // 前置第 1 条：掩码与源图的宽、高相同（掩码没有通道维，第三分量是 None）。
    ensure_same_shape(raster_shape(source), mask_shape(mask))?;
    // 前置第 2 条：generated 与源图的宽、高与通道数三者相同。
    ensure_same_shape(raster_shape(source), raster_shape(generated))?;
    // 前置第 3 条：三个载体的 `pixels` 长度与各自声明的形状相符（掩码是单通道位图）。
    let source_len = source.pixels.len();
    ensure_length_is_consistent(source.width, source.height, source.channels, source_len)?;
    let generated_len = generated.pixels.len();
    ensure_length_is_consistent(
        generated.width,
        generated.height,
        generated.channels,
        generated_len,
    )?;
    let mask_len = mask.pixels.len();
    ensure_length_is_consistent(mask.width, mask.height, 1, mask_len)?;

    let channels = source.channels as usize;
    let pixel_count = source.width as usize * source.height as usize;
    let mut pixels = Vec::with_capacity(source_len);
    for index in 0..pixel_count {
        // 掩码的非零字节表示该像素在 mask 内（位图约定；编码格式本块不定义）。
        let from = if mask.pixels[index] != 0 {
            generated
        } else {
            source
        };
        let start = index * channels;
        pixels.extend_from_slice(&from.pixels[start..start + channels]);
    }

    Ok(Raster {
        width: source.width,
        height: source.height,
        channels: source.channels,
        pixels,
    })
}

/// §168 `:832-835` 的 `outside-mask pixel difference`。
/// 确定性：逐点比较，无模型、无阈值。返回 mask 外**不相等**的像素数与总像素数。
///
/// **前置条件与 [`composite_masked`] 同形三条**（`mask` / `source` / `result` 三者宽高相同；
/// `source` 与 `result` 通道数相同；三个载体的 `pixels` 长度与各自声明的形状相符）。
/// 前两条不成立即 `ShapeMismatch`（掩码那一侧的第三分量是 `None`）；第三条不成立即
/// `PixelLengthMismatch`。**这里不是「比较器只读不写、不必设前置」**：比较器的产出
/// （差多少）会被下游当成判据的量，**量在未定义输入上算出来的值比 `Err` 更坏**（设计 §5.3）。
///
/// **「总数」的读法（本计划自定）**：[`PixelDiff::total`] 取 **mask 外的像素数**
/// （理由与收件人见 [`PixelDiff`] 与计划 `## 遗留` 第五节）。**计数的单位是像素**：
/// mask 外某像素在两侧的通道字节序列不同即算一次（任一字节不同即可）——
/// 差异量是「不相等像素数」，不是「不相等字节数」。
///
/// **本函数不判「通过」**：它给出差多少，「差多少算不通过」的判据不在本块（设计 §5.3）。
/// **理由**：返回 `bool` 就把一条判据（阈值）塞进了本块，而 §326 给的是 `= FORBIDDEN`
/// （阈值为 0 的那一个特例）、§168 给的是「可以直接确定性检查」——**两者都不足以让本块
/// 替 P5a 定「多少算不过」**。故本块给**量**，判**在 P5a**；`PixelDiff` 上没有
/// `passed()` / `is_ok()` 一类判定方法（设计 §5.3 的第二半、§7.3）。
///
/// **§5.4 的处置（逐条照录）**：
///
/// - **三层，逐层给出它保证到哪**：**强制作（使之合法）**＝`hard-composite`（Task 2 的第 4 行）
///   ——走过这一步的结果图**按构造**满足 §326；**检出（发现不合法）**＝`verify-outside-mask`
///   （第 5 行）与 [`outside_mask_difference`]——**确定性**，且**不读模型自述**
///   （§168 `:837-843` 逐字「这比让模型说『我只修改了手。』可靠得多」）；**类型层**＝**没有**。
/// - **类型层拦不住的依据**（设计 §5.4）：`ArtifactType`
///   （`crates/continuum-artifact/src/artifact.rs:14-21`）是**内容类型的枚举**，它没有、
///   也不该有「这张 `Image` 与那张 `Image` 在 mask 外逐点相同」这种**跨制品关系**的表达力。
///   故 §326 的 FORBIDDEN 是**校验**与**强制作**，不是类型约束；**本设计不为此向 P5e 要新变体**。
/// - **失配方向（这是本条要回答的那一问）**：
///   **A. 当 `preserve_outside_region = true` 的节点没有接上 `hard-composite` 时，不报错**
///   ——产出是一张 mask 外被改动过的 `Image`，它与其他 `Image` 在类型上**完全一样**，
///   只有跑了比较器才看得见。**这一侧是 fail-open。**
///   故 §326 在本块的兑现是「提供使之成立的步骤 ＋ 提供检出它的检查器」，**不是**「保证
///   它一定被用上」——后者要求「谁保证带这条参数的节点一定接了 `hard-composite`」，
///   而规范未给（§13 第 2 条）。
///   **B. 反向的一侧**：若 `hard-composite` 被接上但掩码与源图不同坐标系（或 `generated`
///   与源图形状不同）⇒ `Err(ShapeMismatch)`。**这一侧是 fail-closed**，故它**不能**用来
///   代替 A：它拦的是「掩码／补丁配错」，拦不了「根本没有合成」。
/// - **本处无照片（写明）**：拍得到的只有这两个函数的返回值；**拍不到「图里真有一段管线
///   没接上合成」**——那一侧的失配正是 fail-open 本身（`tests/pixels.rs` 与设计 §11.5 第 2 条）。
pub fn outside_mask_difference(
    source: &Raster,
    result: &Raster,
    mask: &EditMask,
) -> Result<PixelDiff, ImageError> {
    // 前置条件与 `composite_masked` 同形三条（§5.3）：mask / source / result 三者宽高相同、
    // source 与 result 通道数相同、三个载体的 `pixels` 长度与各自声明的形状相符。
    ensure_same_shape(raster_shape(source), mask_shape(mask))?;
    ensure_same_shape(raster_shape(source), raster_shape(result))?;
    let source_len = source.pixels.len();
    ensure_length_is_consistent(source.width, source.height, source.channels, source_len)?;
    let result_len = result.pixels.len();
    ensure_length_is_consistent(result.width, result.height, result.channels, result_len)?;
    let mask_len = mask.pixels.len();
    ensure_length_is_consistent(mask.width, mask.height, 1, mask_len)?;

    let channels = source.channels as usize;
    let pixel_count = source.width as usize * source.height as usize;
    let mut differing = 0usize;
    let mut total = 0usize;
    for index in 0..pixel_count {
        // mask 内的像素不参与：只统计 mask 外的像素。
        if mask.pixels[index] != 0 {
            continue;
        }
        total += 1;
        let start = index * channels;
        if source.pixels[start..start + channels] != result.pixels[start..start + channels] {
            differing += 1;
        }
    }

    Ok(PixelDiff { differing, total })
}

/// 一个 `Raster` 的形状三元组——第三分量是 `Some(通道数)`（设计 §3.1 的 `ShapeMismatch`）。
fn raster_shape(raster: &Raster) -> (u32, u32, Option<u8>) {
    (raster.width, raster.height, Some(raster.channels))
}

/// 一具 `EditMask` 的形状三元组——掩码是单通道位图，**没有色彩通道这一维**，
/// 故第三分量是 `None`（设计 §3.1）。
fn mask_shape(mask: &EditMask) -> (u32, u32, Option<u8>) {
    (mask.width, mask.height, None)
}

/// 前置第 1、2 条的形状比较——**两个公开函数唯一的形状比较点**（否则两处会各自漂移，
/// 本仓「同一件事两套词汇表」的形状）。不满足即 `ShapeMismatch { left, right }`。
///
/// 判据：`(宽, 高)` 必须相同；**两侧都有通道维时**通道数也必须相同——掩码那一侧是
/// `None`（没有可比的一维），故掩码与图像的比较只比宽高。**`left` / `right` 的顺序**
/// 是本实现定的：`left` 取 `source` 那一侧（两个函数的共同参照物）、`right` 取被比的
/// 另一侧——设计没给顺序。
fn ensure_same_shape(
    left: (u32, u32, Option<u8>),
    right: (u32, u32, Option<u8>),
) -> Result<(), ImageError> {
    let dims_equal = (left.0, left.1) == (right.0, right.1);
    let channels_equal = match (left.2, right.2) {
        (Some(left_channels), Some(right_channels)) => left_channels == right_channels,
        _ => true,
    };
    if dims_equal && channels_equal {
        Ok(())
    } else {
        Err(ImageError::ShapeMismatch { left, right })
    }
}

/// 前置第 3 条——**两个公开函数唯一的字节数核对点**：`len` 必须等于
/// `宽 × 高 × 通道数`（掩码传 `1` 作为通道数——它是单通道位图）。不满足即
/// `PixelLengthMismatch { len, expected }`——**这一条不走 `ShapeMismatch`**：它比的是
/// **一个载体内部**的两件事（自己声明的形状与自己字节序列的长度），而 `ShapeMismatch`
/// 的字段是 `(宽, 高, 通道数)`，**两个 `usize` 的长度值表达不进那个形状**（设计 §5.2 末）。
fn ensure_length_is_consistent(
    width: u32,
    height: u32,
    channels: u8,
    len: usize,
) -> Result<(), ImageError> {
    let expected = width as usize * height as usize * channels as usize;
    if len == expected {
        Ok(())
    } else {
        Err(ImageError::PixelLengthMismatch { len, expected })
    }
}
