//! Task 1 的照片集：`EditRegionKind` 的编码（§11.1）、`ImageError` 的四臂、
//! `EditRegion` 四支的载荷与两枚辅助载体的字段清单。
//!
//! **逐条标明照片的种类**：编译期照片（穷尽解构、不带 `..`，改动即编不过）与
//! 运行期照片（值级的往返与断言）分开写、不合混。

use continuum_image::{
    EditMask, EditRegion, EditRegionKind, ImageError, NormalizedAnchor, NormalizedBox,
    SemanticObjectRef,
};
use continuum_operator::{OperatorError, OperatorId, OperatorVersion};

/// §11.1：四臂各一条往返 + `ALL` 的逐臂成员断言 + 四串的手写期望。
///
/// **编译期照片**（不是运行期红）：`as_str` 的 `match` 入参是枚举自身、无通配臂，
/// 故加第五臂即**编译不过**（按纪律 1(c) 不算运行期红）；`parse` 收 `&str`、
/// **必有通配臂**，故它**不是**那张照片——解码侧的守卫是本用例的往返
/// （口径同 `crates/continuum-artifact/tests/artifact_type.rs`）。
///
/// **「四臂」这句话的射程**：它说的是**本块的编码**恰四枚，**不是**「规范只有四个
/// 区域种类」——§166 `:745-752` 列的是同一组的另两个名字（`Point` /
/// `Bounding Box` / `Brush Mask` / `Semantic Object`），本设计取 §325 的四个名字
/// （设计 §4.1）。
///
/// 红条件：`as_str` 少一档（**放宽档**）→ 该臂的往返与 `ALL` 那条红。
#[test]
fn every_edit_region_kind_round_trips_through_its_encoding() {
    // 四串的手写期望：期望值逐字写出，不从被测物取。
    assert_eq!(
        EditRegionKind::Point.as_str(),
        "point",
        "Point 的编码串不符"
    );
    assert_eq!(EditRegionKind::Box.as_str(), "box", "Box 的编码串不符");
    assert_eq!(EditRegionKind::Mask.as_str(), "mask", "Mask 的编码串不符");
    assert_eq!(
        EditRegionKind::SemanticObject.as_str(),
        "semantic_object",
        "SemanticObject 的编码串不符"
    );

    // `ALL` 的逐臂成员断言：四枚都在名单里。**不写 len() 断言**——`ALL` 的数目写在
    // 类型 `[EditRegionKind; 4]` 里，补一条 `len() == 4` 断言测的是 Rust 的 `len()`
    // ；「多一枚」会在 `as_str` 的穷尽 match 处编译不过（设计 §11.1 对同形写法的判据）。
    assert!(
        EditRegionKind::ALL.contains(&EditRegionKind::Point),
        "ALL 应含 Point"
    );
    assert!(
        EditRegionKind::ALL.contains(&EditRegionKind::Box),
        "ALL 应含 Box"
    );
    assert!(
        EditRegionKind::ALL.contains(&EditRegionKind::Mask),
        "ALL 应含 Mask"
    );
    assert!(
        EditRegionKind::ALL.contains(&EditRegionKind::SemanticObject),
        "ALL 应含 SemanticObject"
    );

    // 四臂各一条往返（遍历 ALL：四枚逐个走 as_str / parse）。
    for kind in EditRegionKind::ALL {
        let encoded = kind.as_str();
        assert_eq!(
            EditRegionKind::parse(encoded),
            Some(kind),
            "{encoded} 应解回 {kind:?}"
        );
    }
}

/// §166 的另四个名字**不进编码**（本计划新增的守卫，代价与收益见计划遗留第五节）。
///
/// 理由：`as_str` / `parse` 是本编码**唯一的一对产生点与逆**（照
/// `crates/continuum-artifact/src/artifact.rs:9-11` 的注记口径），收进别名会让
/// 「一型两名」可表达；而设计 §4.1 记录的两组名字之间的关系是**本设计的对应表**，
/// 不是编码的一部分。
///
/// 红条件：把 `"brush_mask"` 也收进 `parse`（**移除档**）→ 本条红。
#[test]
fn an_alias_from_166_is_not_in_the_encoding() {
    assert_eq!(
        EditRegionKind::parse("brush_mask"),
        None,
        "§166 的别名不应解出任何区域种类"
    );
    assert_eq!(
        EditRegionKind::parse("bounding_box"),
        None,
        "§166 的别名不应解出任何区域种类"
    );
}

/// `ImageError` 四臂的载荷形状（**编译期照片**）。
///
/// 四臂**逐臂穷尽解构、不带 `..`**：任一支加字段、少字段、改名即**编译不过**
/// （match 也不带通配臂：加第五臂同样编译不过）。
/// **明写：本用例是编译期照片，运行期无照片**——Rust 没有反射，把四个变体名
/// 手抄一遍再比对是手抄自己，不要把它写成一条恒真的运行期断言。
///
/// `ShapeMismatch` 的第三分量是 `Option<u8>` 的理由（设计 §3.1）：掩码是单通道
/// 位图（本 task 的 `EditMask` 没有 `channels` 字段），故掩码那一侧是 `None`；
/// `None` 的分量在比较时不参与。
#[test]
fn the_four_error_arms_carry_exactly_their_declared_fields() {
    // 解构桩：每臂的每个字段都绑定并逐项类型标注——形状即照片。
    fn assert_arm_shape(error: ImageError) {
        match error {
            ImageError::Registry(e) => {
                let _: OperatorError = e;
            }
            ImageError::RegionUnresolved { region } => {
                let _: EditRegionKind = region;
            }
            ImageError::ShapeMismatch { left, right } => {
                let _: (u32, u32, Option<u8>) = left;
                let _: (u32, u32, Option<u8>) = right;
            }
            ImageError::PixelLengthMismatch { len, expected } => {
                let _: usize = len;
                let _: usize = expected;
            }
        }
    }

    // 四臂各构造一个值，逐臂走一遍解构。
    assert_arm_shape(ImageError::Registry(OperatorError::Duplicate {
        id: OperatorId::new("generate-image"),
        version: OperatorVersion::new(1),
    }));
    assert_arm_shape(ImageError::RegionUnresolved {
        region: EditRegionKind::Point,
    });
    assert_arm_shape(ImageError::ShapeMismatch {
        left: (4, 3, None),
        right: (4, 3, Some(3)),
    });
    assert_arm_shape(ImageError::PixelLengthMismatch {
        len: 11,
        expected: 12,
    });
}

/// `RegionUnresolved` 的消息里带得出是哪一型没解析出来（**运行期**）。
///
/// 来历：`EditRegionKind` 要 `derive(Debug)` 正是因为它（设计 §3.1；本块不必为它
/// 写 `Display`，各处都用 `{:?}`）。
///
/// 红条件：把该臂的 `#[error]` 串改成一个不含 `region` 的固定串（**移除档**）
/// → 两条都红。
///
/// **这条照片的限度写清**：它钉「消息里带得出是哪一型」，**不钉**「这一型是不是
/// 判对了」——后者由 Task 5 的两条（`g`／`a_semantic_object_…`）承担。
#[test]
fn region_unresolved_names_the_kind_that_failed() {
    let point = ImageError::RegionUnresolved {
        region: EditRegionKind::Point,
    }
    .to_string();
    assert!(point.contains("Point"), "消息里应带出 Point，实际：{point}");

    let semantic = ImageError::RegionUnresolved {
        region: EditRegionKind::SemanticObject,
    }
    .to_string();
    assert!(
        semantic.contains("SemanticObject"),
        "消息里应带出 SemanticObject，实际：{semantic}"
    );
}

/// `EditMask` 的字段清单（**编译期照片**）：恰是 `width` / `height` / `pixels` 三件，
/// 逐项穷尽解构、不带 `..`。
///
/// **没有 `channels`**（与 Task 6 的 `Raster` 的差别正在此：它是单通道位图，不是
/// 图像）、**没有 `artifact` / `ArtifactId` 一类字段**（设计 §4.1：`Box` 支的掩码由
/// 投影得出，那一刻仓里还没有它的制品 id；把它变成 `ArtifactType::Blob` 制品要经
/// `BlobStore::put`，而本块不做 IO）。
///
/// **不变量** `pixels.len() == width as usize * height as usize` 的**判据在 Task 6
/// 的两个函数入口**，本 task 只有声明与文档注释。
///
/// 红条件：给 `EditMask` 加一个 `channels: u8` 字段（**移除档** → 穷尽解构先编译不过）。
#[test]
fn the_mask_carries_the_bitmap_and_no_channel_dimension() {
    let mask = EditMask {
        width: 4,
        height: 3,
        pixels: vec![0u8; 12],
    };
    let EditMask {
        width,
        height,
        pixels,
    } = mask;
    let _: u32 = width;
    let _: u32 = height;
    let _: Vec<u8> = pixels;
}

/// `EditRegion` 四支的载荷形状（**编译期照片**）：`Point(NormalizedAnchor)` /
/// `Box(NormalizedBox)` / `Mask(EditMask)` / `SemanticObject(SemanticObjectRef)`。
///
/// 四支**逐支穷尽解构、不带 `..`**；match 亦穷尽无通配臂，加第五支同样编译不过。
///
/// 红条件：把 `Box` 支的载荷改成 `(NormalizedAnchor, NormalizedAnchor)`
/// （**取反档**）→ 编译不过。
#[test]
fn the_four_region_arms_carry_exactly_the_four_declared_payloads() {
    fn assert_arm_shape(region: EditRegion) {
        match region {
            EditRegion::Point(anchor) => {
                let _: NormalizedAnchor = anchor;
            }
            EditRegion::Box(bounding) => {
                let _: NormalizedBox = bounding;
            }
            EditRegion::Mask(mask) => {
                let _: EditMask = mask;
            }
            EditRegion::SemanticObject(object) => {
                let _: SemanticObjectRef = object;
            }
        }
    }

    assert_arm_shape(EditRegion::Point(NormalizedAnchor { x: 0.25, y: 0.75 }));
    assert_arm_shape(EditRegion::Box(NormalizedBox {
        p: NormalizedAnchor { x: 0.0, y: 0.0 },
        q: NormalizedAnchor { x: 1.0, y: 1.0 },
    }));
    assert_arm_shape(EditRegion::Mask(EditMask {
        width: 2,
        height: 2,
        pixels: vec![1, 0, 0, 1],
    }));
    assert_arm_shape(EditRegion::SemanticObject(SemanticObjectRef::from_str(
        "左边这个人",
    )));
}

/// 四支逐支报告自己的判别式（**运行期**，四支逐支不抽代表）。
///
/// `kind()` 是本计划自定的一处读法（设计只要求 `RegionUnresolved { region }`
/// 表达得出是哪一型，没给读法的名字；代价与理由见计划遗留第五节）。
///
/// 红条件：把 `kind()` 的 `SemanticObject` 那一支写成 `EditRegionKind::Point`
/// （**取反档**）→ 第四条红。
#[test]
fn every_region_arm_reports_its_own_kind() {
    let point = EditRegion::Point(NormalizedAnchor { x: 0.5, y: 0.5 });
    assert_eq!(point.kind(), EditRegionKind::Point, "Point 支应报告 Point");

    let bounding = EditRegion::Box(NormalizedBox {
        p: NormalizedAnchor { x: 0.0, y: 0.0 },
        q: NormalizedAnchor { x: 1.0, y: 1.0 },
    });
    assert_eq!(bounding.kind(), EditRegionKind::Box, "Box 支应报告 Box");

    let mask = EditRegion::Mask(EditMask {
        width: 1,
        height: 1,
        pixels: vec![1],
    });
    assert_eq!(mask.kind(), EditRegionKind::Mask, "Mask 支应报告 Mask");

    let object = EditRegion::SemanticObject(SemanticObjectRef::from_str("左边这个人"));
    assert_eq!(
        object.kind(),
        EditRegionKind::SemanticObject,
        "SemanticObject 支应报告 SemanticObject"
    );
}

/// 两枚载体的字段清单（**编译期照片**）：`NormalizedAnchor` 的两个字段逐项、
/// `NormalizedBox` 的两个角逐项，均穷尽解构、不带 `..`。
///
/// **两角的名字不表达几何次序**：§166 只给「Bounding Box」一个名字，**哪一角是
/// 左上规范未给**，故本型不表达次序，投影时逐维取 min/max——两角对调给出同一个
/// 掩码（那条运行期用例在 Task 5 建，它要 `resolve_edit_region`）。
/// **字段名 `p` / `q` 是本计划自定**（设计只说「它的两个角」，没给名字；取不作
/// 方向暗示的两个名字，是为了不发明「哪一角是左上」这条规范没给的约定）。
#[test]
fn the_two_carriers_declare_exactly_their_fields() {
    let anchor = NormalizedAnchor { x: 0.125, y: 0.5 };
    let NormalizedAnchor { x, y } = anchor;
    let _: f64 = x;
    let _: f64 = y;

    let bounding = NormalizedBox {
        p: NormalizedAnchor { x: 0.0, y: 0.0 },
        q: NormalizedAnchor { x: 1.0, y: 1.0 },
    };
    let NormalizedBox { p, q } = bounding;
    let _: NormalizedAnchor = p;
    let _: NormalizedAnchor = q;
}

/// `SemanticObjectRef` 的不透明串往返（**运行期**）。
///
/// **这一条的限度与设计 §4.1 的关系写清**（否则读者会以为本用例与设计相抵）：
/// `SemanticObjectRef` 的形状是**不透明串**，它**不填**设计 §4.3 表第 5 行那条
/// 缺口（「名字从哪来：自由文本？受控词表？」）——一枚不透明串既不规定名字的
/// 来源，也不封闭取值域，故把 `SemanticObjectRef` 定成不透明串**不是「给规范没给
/// 的东西定形状」**；设计 §4.1 的「不给它定形状」讲的是**不替它定名字空间**。
///
/// 红条件：把 `SemanticObjectRef` 换成封闭枚举（**移除档**）→ 本条**编译不过**
/// （按纪律 1(c) 不算红），**故本条的守卫是运行期往返，不是「加一臂会红」**。
#[test]
fn a_semantic_object_ref_is_an_opaque_string() {
    // §166 `:769` 给的正是一句自然语言（「左边这个人」）——一个表外取值。
    let natural_language = SemanticObjectRef::from_str("左边这个人");
    assert_eq!(
        natural_language.as_str(),
        "左边这个人",
        "不透明串应原样往返"
    );

    // 一个类名字符串同样原样往返：不透明串不封闭取值域。
    let name_like = SemanticObjectRef::from_str("left_person");
    assert_eq!(name_like.as_str(), "left_person", "不透明串应原样往返");
}
