//! Task 5 的照片集：`resolve_edit_region` 四支的用例（§11.2 的 (d)–(h)、本计划补的
//! `SemanticObject` 一支与一条越界用例）。
//!
//! **桩的纪律写死**（设计 §11.2 的第二条预防）：`Point` 与 `SemanticObject` 两支的
//! detector **都是本文件自己写的桩**——[`FixedMaskStub`] 返回一枚固定的掩码、
//! [`NoRegionStub`] 两支都返回空。**桩的语料不取自本块的任何名单或常量**
//! （固定掩码的宽高、像素与名字都是手写值）：语料从被测物里取，那些用例测的就是
//! 「桩与它自己一致」（计划纪律 1(b)(iii) 的恒真假照片）。
//!
//! **期望语料手写**：各处 `source_dims`、`NormalizedBox` 的两角、掩码的期望像素
//! 都逐值手写，不从被测物取。
//!
//! **本文件钉不到什么**（§4.3 末句与 §11.5 第 3 条）：§4.3 表里那五条判据缺口
//! （选取规则 / 置信度门槛 / 零检出 / 区域上限 / `SemanticObject` 的名字空间）
//! **一条都不在本文件里被填**——拍得到的只有「桩的返回被原样传出去」与
//! 「空返回即 `Err`」两条，**拍不到「区域选得对不对」**（逐条写在对应用例的注释里）。

use continuum_artifact::ArtifactId;
use continuum_image::{
    EditMask, EditRegion, EditRegionKind, ImageError, NormalizedAnchor, NormalizedBox,
    RegionDetector, SemanticObjectRef, resolve_edit_region,
};

/// 夹具源制品的 id：一枚手写值（签名与 detector 的两个方法都收它；桩把收到的值
/// 原样忽略——本文件不观测它的传递，那是 §4.2 签名面的事）。
fn fixture_source() -> ArtifactId {
    ArtifactId::new("fixture-source")
}

/// `Point` / `SemanticObject` 两支的桩（**用例自己写**）：两个方法都返回同一枚
/// **固定掩码**。
///
/// 固定掩码的宽高（4×1）与像素（9 / 8 / 7 / 6）都是手写值，且与本文件各处的
/// `source_dims`（8×4）**不同**——「本函数不合成像素、不改写区域」这一条才有
/// 可观测的载体：若函数自己投影或改写区域，逐字节断言即红。
struct FixedMaskStub {
    mask: EditMask,
}

impl RegionDetector for FixedMaskStub {
    fn region_at(
        &self,
        _source: &ArtifactId,
        _source_dims: (u32, u32),
        _point: &NormalizedAnchor,
    ) -> Option<EditMask> {
        Some(self.mask.clone())
    }

    fn region_named(
        &self,
        _source: &ArtifactId,
        _source_dims: (u32, u32),
        _name: &SemanticObjectRef,
    ) -> Option<EditMask> {
        Some(self.mask.clone())
    }
}

/// 「找不到」的桩（**用例自己写**）：两个方法都返回空（`None`）。
///
/// 它同时服务 `Point` 与 `SemanticObject` 两支的零检出用例——两支各有一条用例、
/// **不抽代表**（`region` 字段的期望值各判各的）。
struct NoRegionStub;

impl RegionDetector for NoRegionStub {
    fn region_at(
        &self,
        _source: &ArtifactId,
        _source_dims: (u32, u32),
        _point: &NormalizedAnchor,
    ) -> Option<EditMask> {
        None
    }

    fn region_named(
        &self,
        _source: &ArtifactId,
        _source_dims: (u32, u32),
        _name: &SemanticObjectRef,
    ) -> Option<EditMask> {
        None
    }
}

/// §11.2 第 (d) 条：`Box` → 掩码的**投影**——给定手写的 `source_dims`（8×4）与手写的
/// 两角 `(0.25, 0.25)` / `(0.75, 0.75)`，断言返回掩码的宽高等于 `source_dims`，
/// 且**被置位的像素集合恰是手写期望的那个集合**（逐像素比对全部 32 个像素；
/// 不是只看 `pixels.len()`、不是只看「有像素被置位」）。
///
/// **`Box` 支是投影（无模型）**：§4.2 的表里，它是四支里**唯一不调 detector 的一支**——
/// 本用例用两个不同的桩（一个返回空、一个返回固定掩码）各解析一次，两个结果必须
/// 逐字节相同：若本支误经 detector，「返回空」那一次会 `Err`（`.expect` 即红）、
/// 「返回固定掩码」那一次会拿到 4×1 的掩码（宽高断言即红）。
///
/// **投影规则**（本计划自定，见 `src/resolve.rs` 的 `project_box`）：逐维取 min/max、
/// 夹取到 `[0, 1]`、低边 floor × 高边 ceil 的像元取交。手写期望按这条规则写出：
/// 列 2..6、行 1..3 被置位。
///
/// **红条件（取反档）**：把矩形的 `x` 那一维用 `height` 相乘（列号按高度取整）——
/// 列区间由 2..6 变成 1..3，逐像素比对在第一个不符的像素处红。
#[test]
fn the_box_arm_projects_the_normalized_rect_onto_the_source_canvas() {
    let source_dims = (8, 4);
    let region = EditRegion::Box(NormalizedBox {
        p: NormalizedAnchor { x: 0.25, y: 0.25 },
        q: NormalizedAnchor { x: 0.75, y: 0.75 },
    });

    let via_empty = resolve_edit_region(&region, &fixture_source(), source_dims, &NoRegionStub)
        .expect("Box 支是投影，不经 detector：空返回的桩下也必须解析成功");
    let via_fixed = resolve_edit_region(
        &region,
        &fixture_source(),
        source_dims,
        &FixedMaskStub {
            mask: EditMask {
                width: 4,
                height: 1,
                pixels: vec![9, 8, 7, 6],
            },
        },
    )
    .expect("Box 支是投影：桩返回什么都不影响它");

    assert_eq!(via_empty.width, 8, "返回掩码的宽必须等于 source_dims 的宽");
    assert_eq!(via_empty.height, 4, "返回掩码的高必须等于 source_dims 的高");
    assert_eq!(
        via_empty, via_fixed,
        "Box 支不读 detector：两个不同桩下的结果必须逐字节相同"
    );

    // 期望的置位像素集合，逐像素手写（4 行 × 8 列；行 1..3、列 2..6 被置位）。
    let expected: [u8; 32] = [
        0, 0, 0, 0, 0, 0, 0, 0, //
        0, 0, 1, 1, 1, 1, 0, 0, //
        0, 0, 1, 1, 1, 1, 0, 0, //
        0, 0, 0, 0, 0, 0, 0, 0, //
    ];
    assert_eq!(
        via_empty.pixels.len(),
        expected.len(),
        "掩码的像素数必须等于 source_dims 的面积"
    );
    for (index, expected_value) in expected.iter().enumerate() {
        assert_eq!(
            via_empty.pixels[index],
            *expected_value,
            "第 {index} 个像素（行 {} 列 {}）与手写期望不符",
            index / 8,
            index % 8
        );
    }
}

/// **运行期**，钉 Task 1 的「两角的名字不表达几何次序」（`NormalizedBox` 的 `p` / `q`）：
/// 同一个矩形的两角**对调**构造两个 `NormalizedBox` ⇒ 两次的掩码**逐字节相同**
/// （含宽高）。§166 没给「哪一角是左上」，故投影时逐维取 min/max。
///
/// **红条件（取反档）**：把投影写成「假定 `p` 是左上、不取 min/max」——对调后
/// 列区间变成空（`floor(6) > ceil(2)`），掩码与正序那一次不同，本条红。
#[test]
fn a_box_order_swapped_gives_the_same_mask() {
    let source_dims = (8, 4);
    let forward = EditRegion::Box(NormalizedBox {
        p: NormalizedAnchor { x: 0.25, y: 0.25 },
        q: NormalizedAnchor { x: 0.75, y: 0.75 },
    });
    let swapped = EditRegion::Box(NormalizedBox {
        p: NormalizedAnchor { x: 0.75, y: 0.75 },
        q: NormalizedAnchor { x: 0.25, y: 0.25 },
    });

    let first = resolve_edit_region(&forward, &fixture_source(), source_dims, &NoRegionStub)
        .expect("正序两角应解析成功");
    let second = resolve_edit_region(&swapped, &fixture_source(), source_dims, &NoRegionStub)
        .expect("对调两角应解析成功");

    assert_eq!(first.width, second.width, "两次的宽必须相同");
    assert_eq!(first.height, second.height, "两次的高必须相同");
    assert_eq!(
        first.pixels, second.pixels,
        "两角对调必须给出逐字节相同的掩码"
    );
}

/// **本计划自定**（计划遗留第一节第 4 条与第五节）：`Box` 的一个角越出单位正方形
/// （如 `x = 1.5`）⇒ `Ok`，且掩码的宽高仍是 `source_dims`、`pixels.len()` 满足不变量
/// （`宽 × 高`）、被置位的像素**不越界**。
///
/// **为什么是夹取而不是新增一枚错误臂**：`ImageError` 的四臂里**没有**一枚表达
/// 「坐标越界」——`RegionUnresolved` 说的是「解析不出区域」、`ShapeMismatch` 说的是
/// 「两载体形状不同」，两者都表达不了这一件事；而**臂数是设计 §3.1 钉死的四臂**，
/// 故本计划取夹取。**代价**：越界输入被静默地当成「贴边」；**收益**：不新增一枚设计
/// 没有的臂，也不 panic。§163 `:648` 给了 `x, y ∈ [0, 1]` 的约定而**没有给越界输入的
/// 处置**——这一处是本计划补的，**不是规范给的**（同一段也写在 `src/resolve.rs` 的
/// 函数文档里）。
///
/// **红条件（移除档）**：把夹取去掉 ⇒ `x = 1.5` / `y = 1.5` 的行列区间右端越出画布
/// （`1.5 × 8 = 12` 列、`1.5 × 4 = 6` 行），本用例在越界索引处 panic。
#[test]
fn a_box_reaching_outside_the_unit_square_is_clamped_to_the_canvas() {
    let source_dims = (8, 4);

    // >1 的一侧：两角之一 `(1.5, 1.5)` 在两个轴上都在单位正方形之外——
    // 夹到 `(1.0, 1.0)`（列区间 4..8、行区间 2..4）。两轴都越界是有意的：
    // 移除夹取后行区间会走到 6，越界索引在填充时就 panic（brief 预告的两种
    // 红线形态之一；只越界一个轴时，溢出的写入会落在缓冲区内的别的行上，
    // 红线形态变成像素比对而不是 panic）。
    let over = EditRegion::Box(NormalizedBox {
        p: NormalizedAnchor { x: 0.5, y: 0.5 },
        q: NormalizedAnchor { x: 1.5, y: 1.5 },
    });
    let over_mask = resolve_edit_region(&over, &fixture_source(), source_dims, &NoRegionStub)
        .expect("越界的角必须被夹取到画布，而不是退化成 Err 或 panic");

    assert_eq!(over_mask.width, 8, "夹取不改画布：宽仍是 source_dims 的宽");
    assert_eq!(over_mask.height, 4, "夹取不改画布：高仍是 source_dims 的高");
    assert_eq!(
        over_mask.pixels.len(),
        8 * 4,
        "不变量：pixels.len() == width as usize * height as usize"
    );

    // 期望集合：行 2..4、列 4..8 被置位（逐像素手写）。
    let expected: [u8; 32] = [
        0, 0, 0, 0, 0, 0, 0, 0, //
        0, 0, 0, 0, 0, 0, 0, 0, //
        0, 0, 0, 0, 1, 1, 1, 1, //
        0, 0, 0, 0, 1, 1, 1, 1, //
    ];
    for (index, expected_value) in expected.iter().enumerate() {
        assert_eq!(
            over_mask.pixels[index],
            *expected_value,
            "第 {index} 个像素（行 {} 列 {}）与手写期望不符——置位像素不得越界",
            index / 8,
            index % 8
        );
    }

    // 对称的一半：<0 的一侧（x = -0.5 夹到 0.0；列区间 0..2、行区间 1..3）。
    // brief 只给了 x = 1.5 的示例——这一半是同一条夹取规则的负侧，钉「低边夹取后的
    // 期望集合」。移除档的 panic 落在正侧：删除夹取后，负值经 `as u32` 的饱和转换
    // 仍是 0（越界只发生在高边，见上）。
    let under = EditRegion::Box(NormalizedBox {
        p: NormalizedAnchor { x: -0.5, y: 0.25 },
        q: NormalizedAnchor { x: 0.25, y: 0.75 },
    });
    let under_mask = resolve_edit_region(&under, &fixture_source(), source_dims, &NoRegionStub)
        .expect("低于 0 的角同样被夹取到画布");
    let expected_under: [u8; 32] = [
        0, 0, 0, 0, 0, 0, 0, 0, //
        1, 1, 0, 0, 0, 0, 0, 0, //
        1, 1, 0, 0, 0, 0, 0, 0, //
        0, 0, 0, 0, 0, 0, 0, 0, //
    ];
    assert_eq!(under_mask.width, 8, "负侧越界同样不改画布");
    assert_eq!(under_mask.height, 4, "负侧越界同样不改画布");
    for (index, expected_value) in expected_under.iter().enumerate() {
        assert_eq!(
            under_mask.pixels[index],
            *expected_value,
            "负侧越界：第 {index} 个像素（行 {} 列 {}）与手写期望不符",
            index / 8,
            index % 8
        );
    }
}

/// §11.2 第 (e) 条：`Mask` 原样通过——返回的 `EditMask` 与入参的掩码**逐字节相同**
/// （含宽高与像素）。
///
/// **夹具的尺寸是故意与 `source_dims` 不同的**（3×2 vs 8×4）：§4.2 的表对本支没有
/// 「尺寸与源图一致」这条前置，掩码的尺寸随掩码走（设计 §4.1）——故它必须原样通过，
/// 「能不能用于合成」由下游 `composite_masked` 的 `ShapeMismatch` 拦。
///
/// **射程写清**：这一条钉的是「**调用方已给的位图不被本函数改写**」（§166 `:750`
/// 逐字「调用方已给出掩码位图」），**不钉**「该位图的尺寸与 `source_dims` 一致」——
/// §4.2 的表**没有**这条前置（计划遗留第一节第 9 条：一个尺寸不符的掩码会 `Ok`
/// 通过区域解析，直到 `composite_masked` 才被拦住）。
///
/// **红条件（取反档）**：把 `Mask` 支也走一遍投影 ⇒ 宽高与像素都变（3×2 变成 8×4），
/// 逐字段断言红。
#[test]
fn the_mask_arm_passes_the_bitmap_through_verbatim() {
    let source_dims = (8, 4);
    let given = EditMask {
        width: 3,
        height: 2,
        pixels: vec![0, 7, 128, 255, 1, 2],
    };
    let region = EditRegion::Mask(given.clone());

    let out = resolve_edit_region(&region, &fixture_source(), source_dims, &NoRegionStub)
        .expect("Mask 支原样通过，不经 detector");

    assert_eq!(out.width, 3, "宽必须原样（不是 source_dims 的宽）");
    assert_eq!(out.height, 2, "高必须原样（不是 source_dims 的高）");
    assert_eq!(out.pixels, vec![0, 7, 128, 255, 1, 2], "像素必须逐字节原样");
    assert_eq!(out, given, "整体必须与入参逐字节相同（含宽高）");
}

/// §11.2 第 (f) 条：`Point` + detector 给出的区域 ⇒ 返回的掩码**逐字节等于** detector
/// 给的掩码（本函数不合成像素、不改写区域）。
///
/// 桩（[`FixedMaskStub`]）返回的是 4×1、像素 9 / 8 / 7 / 6 的手写掩码——尺寸与内容
/// 都与 `source_dims`（8×4）和任何投影结果无关，故「返回的等于桩给的」不是任何
/// 投影语义碰巧满足的。
///
/// **红条件（取反档）**：把 `Point` 支改成「以该点为中心造一个固定大小的方框」
/// ⇒ 返回的是投影掩码而不是桩的掩码，逐字节断言红。
#[test]
fn the_point_arm_returns_the_detectors_region_verbatim() {
    let detector = FixedMaskStub {
        mask: EditMask {
            width: 4,
            height: 1,
            pixels: vec![9, 8, 7, 6],
        },
    };
    let region = EditRegion::Point(NormalizedAnchor { x: 0.3, y: 0.7 });

    let out = resolve_edit_region(&region, &fixture_source(), (8, 4), &detector)
        .expect("桩给出了区域，必须 `Ok`");

    assert_eq!(out.width, 4, "宽必须等于桩给的掩码的宽");
    assert_eq!(out.height, 1, "高必须等于桩给的掩码的高");
    assert_eq!(out.pixels, vec![9, 8, 7, 6], "像素必须等于桩给的掩码的像素");
    assert_eq!(out, detector.mask, "整体必须与桩给的掩码逐字节相同");
}

/// §11.2 第 (g) 条：`Point` + detector 找不到 ⇒
/// `Err(ImageError::RegionUnresolved { region: EditRegionKind::Point })`——
/// **判准 `region` 字段的值恰是 `Point`**。
///
/// **「且不是单像素掩码、不是整图掩码」这一半的写法写死**：`Err` 那一支没有掩码
/// 可看，故本条钉的是「**返回了 `Err` 而不是 `Ok`**」——「退化成单像素」的实现会
/// 返回 `Ok`（1×1 掩码），本条即红；「放大到整图」的实现同样返回 `Ok`（8×4 掩码），
/// 本条同样红。`Ok` 那一臂的 panic 就是这两档的落点。
///
/// **理由（设计 §4.3）**：§325 的 MUST 是**否定式**的（「而不是单像素修改」），
/// 故最小的合法实现是「解析不出就拒绝」；放大到整图会让 §326 的
/// `outside_mask_change = FORBIDDEN` 变得无意义（mask 外为空集，任何改动都「合法」），
/// 退化到单像素正被 §325 明文禁止。
///
/// **这条钉不到什么**（§4.3 末句与 §11.5 第 3 条）：它钉的是**零检出的处置**，
/// **钉不到**「区域选得对不对」——§4.3 表里那五条缺口（选取规则 / 置信度门槛 /
/// 零检出 / 区域上限 / `SemanticObject` 的名字空间）在这一点上**没有判据**，
/// 故本处只能拍「桩的返回被原样传出去」与「空返回即 `Err`」两条。
///
/// **红条件（取反档，两条）**：把零检出改成「合成一个单像素掩码并返回 `Ok`」
/// ⇒ 本条红；改成「放大到整图并返回 `Ok`」⇒ 本条同样红。
#[test]
fn a_point_with_no_region_is_rejected_and_never_degenerates() {
    let region = EditRegion::Point(NormalizedAnchor { x: 0.5, y: 0.5 });

    match resolve_edit_region(&region, &fixture_source(), (8, 4), &NoRegionStub) {
        Err(ImageError::RegionUnresolved { region: kind }) => {
            assert_eq!(
                kind,
                EditRegionKind::Point,
                "判准 `region` 字段的值恰是 Point"
            );
        }
        Ok(mask) => panic!(
            "零检出必须 Err，而不是 Ok——「退化成单像素」与「放大到整图」两档都会走到这里；\
             实际返回的掩码：{mask:?}"
        ),
        Err(other) => panic!("应为 RegionUnresolved {{ region: Point }}，实际 {other:?}"),
    }
}

/// **本计划补的第四支**（计划遗留第一节第 2 条）：`SemanticObject` + detector 对名字
/// 返回「找不到」⇒
/// `Err(ImageError::RegionUnresolved { region: EditRegionKind::SemanticObject })`，
/// **判准 `region` 字段是 `SemanticObject` 而不是 `Point`**（与 `Point` 那条各判各的，
/// 不抽代表）。
///
/// **来历写进注释**：设计 §11.2 的 (d)–(h) 覆盖了 `Box` / `Mask` / `Point` 三支，
/// **`SemanticObject` 那一支没有照片**，而 §4.2 的表列了四支、§3.1 的 `RegionUnresolved`
/// 注又逐字写着它也覆盖它（「Point 点空处、`SemanticObject` 名不副实」）——
/// **枚举到的那几支之外，第四支是 fail-open**。本计划把它补上。
///
/// **红条件（取反档）**：把 `resolve_edit_region` 的两处 `RegionUnresolved` 都写成
/// `region: Point` ⇒ 本条红（上一条仍绿——它的期望本来就是 `Point`）。
#[test]
fn a_semantic_object_with_no_match_is_rejected_with_its_own_kind() {
    let region = EditRegion::SemanticObject(SemanticObjectRef::from_str("fixture-object"));

    match resolve_edit_region(&region, &fixture_source(), (8, 4), &NoRegionStub) {
        Err(ImageError::RegionUnresolved { region: kind }) => {
            assert_eq!(
                kind,
                EditRegionKind::SemanticObject,
                "判准 `region` 字段是 SemanticObject 而不是 Point"
            );
        }
        Ok(mask) => panic!("零检出必须 Err，而不是 Ok；实际返回的掩码：{mask:?}"),
        Err(other) => panic!("应为 RegionUnresolved {{ region: SemanticObject }}，实际 {other:?}"),
    }
}

/// §11.2 第 (h) 条：同一个 `Point` 与同一个 `SemanticObject` **各**在两次调用里给出
/// **逐字节相同**的返回值（函数自身无随机）。**两支各一条，不抽代表。**
///
/// **它是 (f) 的对照**（设计 §11.2 的预防）：只钉 (f) 的话，一个「函数自己造一个区域」
/// 的实现也能满足 (f)——它造出的区域在两次调用里也相同，(h) 照样绿；**(f) 与 (h)
/// 合起来**才要求「返回的等于 detector 给的」而「两次相同」。
///
/// **红条件（取反档）**：在函数里掺一次与入参无关的扰动（如按调用计数器偏移一个像素）
/// ⇒ 点、对象两条都红。
#[test]
fn the_same_region_yields_the_same_return_across_calls() {
    let detector = FixedMaskStub {
        mask: EditMask {
            width: 2,
            height: 2,
            pixels: vec![1, 2, 3, 4],
        },
    };
    let source = fixture_source();

    // 点那一支：同一个 `NormalizedAnchor` 值两次调用。
    let point_region = EditRegion::Point(NormalizedAnchor { x: 0.125, y: 0.875 });
    let point_first =
        resolve_edit_region(&point_region, &source, (8, 4), &detector).expect("第一次调用应 `Ok`");
    let point_second =
        resolve_edit_region(&point_region, &source, (8, 4), &detector).expect("第二次调用应 `Ok`");
    assert_eq!(point_first.width, point_second.width, "两次的宽必须相同");
    assert_eq!(point_first.height, point_second.height, "两次的高必须相同");
    assert_eq!(
        point_first.pixels, point_second.pixels,
        "两次的像素必须逐字节相同"
    );

    // 对象那一支：同一个 `SemanticObjectRef` 值两次调用。
    let object_region = EditRegion::SemanticObject(SemanticObjectRef::from_str("fixture-object"));
    let object_first =
        resolve_edit_region(&object_region, &source, (8, 4), &detector).expect("第一次调用应 `Ok`");
    let object_second =
        resolve_edit_region(&object_region, &source, (8, 4), &detector).expect("第二次调用应 `Ok`");
    assert_eq!(object_first.width, object_second.width, "两次的宽必须相同");
    assert_eq!(
        object_first.height, object_second.height,
        "两次的高必须相同"
    );
    assert_eq!(
        object_first.pixels, object_second.pixels,
        "两次的像素必须逐字节相同"
    );
}
