//! Task 6 的照片集：两个确定性函数（`composite_masked` / `outside_mask_difference`）
//! 与两个像素载体（`Raster` / `PixelDiff`）的用例（§11.2 的 (i)、(j)、(k)、(k2)、(l)
//! 与「不宣告裁决」一条）。
//!
//! **语料手写**：本文件所有掩码、源图、补丁、结果的宽高与字节都是手写字面值，
//! 不从被测物取——从被测物取语料，那些用例测的就是「被测物与它自己一致」（计划纪律 1(b)）。
//!
//! **本文件钉不到什么（写明）**：拍得到的只有这两个函数的返回值；
//! **拍不到「图里真有一段管线没接上合成」**——那一侧的失配正是 fail-open 本身
//! （设计 §5.4 末与 §11.5 第 2 条；§326 的「强制作一定被用上」不在本块的射程里）。

use continuum_image::{
    EditMask, ImageError, PixelDiff, Raster, composite_masked, outside_mask_difference,
};

/// 夹具源图：4×3、2 通道（24 字节）。像素语义由 backend 的约定决定（本块不定义像素格式），
/// 2 通道只取一个大于 1 的手写值——逐像素比对必须跨过「不止一维」这个形状。
fn fixture_source() -> Raster {
    Raster {
        width: 4,
        height: 3,
        channels: 2,
        pixels: vec![
            1, 2, 3, 4, 5, 6, 7, 8, //
            9, 10, 11, 12, 13, 14, 15, 16, //
            17, 18, 19, 20, 21, 22, 23, 24, //
        ],
    }
}

/// 夹具掩码：4×3、单通道（12 字节）。**mask 内 5 枚、mask 外 7 枚、整图 12 枚**——
/// 三种读法互不相同，`the_comparator_reports_the_difference_and_the_total` 靠它把
/// 「总数」的读法（mask 外的像素数）从另外两种里区分出来。
fn fixture_mask() -> EditMask {
    EditMask {
        width: 4,
        height: 3,
        pixels: vec![
            0, 1, 1, 0, //
            1, 0, 0, 0, //
            0, 0, 1, 1, //
        ],
    }
}

/// 只在 mask 内与 [`fixture_source`] 不同的 24 字节（mask 外逐字节相同）。
/// 供 (i) 的 `generated` 与 (l) 第一侧的 `result` 用。
fn inside_only_change() -> Vec<u8> {
    vec![
        1, 2, 101, 102, 103, 104, 7, 8, //
        105, 106, 11, 12, 13, 14, 15, 16, //
        17, 18, 19, 20, 107, 108, 109, 110, //
    ]
}

/// mask 内与 mask 外**都**与 [`fixture_source`] 不同的 24 字节——(j) 的对照件
/// （mask 内像素 1、mask 外像素 9 被改过）。
fn inside_and_outside_change() -> Vec<u8> {
    vec![
        1, 2, 101, 102, 5, 6, 7, 8, //
        9, 10, 11, 12, 13, 14, 15, 16, //
        17, 18, 190, 191, 21, 22, 23, 24, //
    ]
}

/// [`fixture_source`] 的「mask 外一个像素（像素 0）被改掉」版：同一像素的两个通道
/// 都改（99 / 100），其余逐字节相同。
fn one_outside_pixel_changed() -> Raster {
    let mut raster = fixture_source();
    raster.pixels[0] = 99;
    raster.pixels[1] = 100;
    raster
}

/// 夹具自检用：`source` 与 `other` 在 mask 内 / mask 外**何处**有差别。只服务本文件
/// 对夹具常量的守卫（钉 (i) 与 (j) 两张夹具的分工），不观测被测行为。
fn differences_by_region(source: &Raster, other: &Raster, mask: &EditMask) -> (bool, bool) {
    let channels = source.channels as usize;
    let mut inside_differs = false;
    let mut outside_differs = false;
    for index in 0..mask.pixels.len() {
        let start = index * channels;
        let end = start + channels;
        if source.pixels[start..end] == other.pixels[start..end] {
            continue;
        }
        if mask.pixels[index] != 0 {
            inside_differs = true;
        } else {
            outside_differs = true;
        }
    }
    (inside_differs, outside_differs)
}

/// **编译期照片**：两个载体的字段清单。**明写两条**：
///
/// （i）**本用例是编译期照片、运行期无照片**——它钉的是两型的**纯数据声明**
/// （字段名、字段型、字段数目）；运行期那几行只是让穷尽解构有载体，不观测任何行为。
/// **红条件（移除档）**：给 `Raster` 加一个 `color_space` 字段 ⇒ 本条用例的穷尽解构
/// （**不带 `..`**）**先编译不过**——本用例无运行期断言可红，红的形态就是编译失败。
/// （ii）`PixelDiff` 的**形状**：`differing` 这个**字段名是设计给的**——设计 §11.2 第 (l) 条
/// 的用例逐字写 `PixelDiff { differing: 0, .. }`（实测在设计 `:520`）；**本计划自定的是它的
/// 整体形状与总数那一枚的名字**，因为设计 §5.3 只约束「它必须含一个可以判 `== 0` 的差异量」
/// 与「返回 mask 外**不相等**的像素数与总像素数」两句（见计划 `## 遗留` 第五节）。
///
/// `Raster` 的文档注释（**本块不定义像素格式**）写在 `src/pixels.rs` 上。
#[test]
fn the_two_carriers_declare_exactly_their_fields() {
    // `Raster`：逐项穷尽解构、**不带 `..`**——加字段 / 改名字 / 改型都在这里编译不过。
    let raster = Raster {
        width: 4,
        height: 3,
        channels: 2,
        pixels: vec![0; 24],
    };
    let Raster {
        width,
        height,
        channels,
        pixels,
    } = raster;
    let _: u32 = width;
    let _: u32 = height;
    let _: u8 = channels;
    let _: Vec<u8> = pixels;

    // `PixelDiff`：同样逐项穷尽解构（字段清单 = `differing` + `total`），
    // 且 `differing` 可判 `== 0`（设计 §5.3 的那一条约束）。
    let diff = PixelDiff {
        differing: 0,
        total: 7,
    };
    let PixelDiff { differing, total } = diff;
    let _: usize = differing;
    let _: usize = total;
    assert!(
        differing == 0,
        "差异量必须可以判 `== 0`（设计 §5.3 的约束）"
    );
}

/// §11.2 第 (i) 条：`composite_masked` 的正例——mask 外逐点等于 `source`、
/// mask 内逐点等于 `generated`（**逐像素、逐字节比对**，不是抽样）。
/// 夹具：`source` 与 `generated` 在 mask 内不同、mask 外相同（下面的夹具自检钉死这两条）。
///
/// **两张夹具的分工**（设计 §11.3 那一行的落脚）：本条与下一条（(j)）是两张夹具——
/// (i) 的 mask 外无差别、(j) 的 mask 外有差别，**合起来**才是「强制作」的照片。
/// 「把 `composite_masked` 改成原样返回 `generated`」那一档（**放宽档**）只红 (j)；
/// (i) **仍绿**——它的夹具在 mask 外本来就没有差别。**这是预期，不是漏拍**；
/// **若连 (j) 都不红，说明夹具里没有一个 mask 外被改过的对照件**（对照件在 (j) 里）。
///
/// **红条件（取反档）**：把两支的取值写反（mask 内取 `source`、mask 外取 `generated`）
/// ⇒ 本条在第一个不符的像素处红。
#[test]
fn composite_copies_each_region_from_its_own_source() {
    let source = fixture_source();
    let mask = fixture_mask();
    let generated = Raster {
        width: 4,
        height: 3,
        channels: 2,
        pixels: inside_only_change(),
    };

    // 夹具自检：钉 (i) 与 (j) 的分工——本夹具必须「mask 内有差别、mask 外无差别」。
    // 若 mask 外有差别，本用例会越界承重（「原样返回 generated」那一档会连 (i) 一起红，
    // 与 §11.3 的预告不符）。
    let (inside_differs, outside_differs) = differences_by_region(&source, &generated, &mask);
    assert!(
        inside_differs,
        "夹具（i）：mask 内必须至少一处不同，否则本条对合成器无差别"
    );
    assert!(
        !outside_differs,
        "夹具（i）：mask 外必须逐字节相同（分工见本条文档注释）"
    );

    let out = composite_masked(&source, &generated, &mask).expect("形状一致必须 Ok");

    assert_eq!(out.width, source.width, "输出的宽必须与源图相同");
    assert_eq!(out.height, source.height, "输出的高必须与源图相同");
    assert_eq!(out.channels, source.channels, "输出的通道数必须与源图相同");
    assert_eq!(
        out.pixels.len(),
        source.pixels.len(),
        "输出的字节数必须与源图相同"
    );

    let channels = source.channels as usize;
    for index in 0..mask.pixels.len() {
        let start = index * channels;
        let end = start + channels;
        if mask.pixels[index] != 0 {
            assert_eq!(
                out.pixels[start..end],
                generated.pixels[start..end],
                "mask 内像素 {index} 必须逐字节等于 generated"
            );
        } else {
            assert_eq!(
                out.pixels[start..end],
                source.pixels[start..end],
                "mask 外像素 {index} 必须逐字节等于 source"
            );
        }
    }
}

/// §11.2 第 (j) 条——**强制作的那一侧**：夹具的 `generated` 在 **mask 内与 mask 外都不同**
/// 于 `source`；断言输出的 mask 外**逐点等于** `source`（mask 内等于 `generated`）。
///
/// **红条件（放宽档）**：把 `composite_masked` 改成「原样返回 `generated`」⇒ 本条红
/// （mask 外不再等于 `source`），而 (i) **仍绿**（它的夹具在 mask 外本来就没有差别）。
/// **两张夹具的分工**（设计 §11.3 那一行的落脚）：这一档只红 (j) 是**预期**；
/// **若连 (j) 都不红，说明夹具里没有一个 mask 外被改过的对照件**——(i) 与 (j) 两张夹具
/// 合起来才是「强制作」的照片，**只有 (i) 的夹具时此档全绿**。
#[test]
fn composite_forces_the_outside_back_to_the_source() {
    let source = fixture_source();
    let mask = fixture_mask();
    let generated = Raster {
        width: 4,
        height: 3,
        channels: 2,
        pixels: inside_and_outside_change(),
    };

    // 夹具自检：本夹具必须「mask 内与 mask 外都有差别」——对照件就是这么来的。
    let (inside_differs, outside_differs) = differences_by_region(&source, &generated, &mask);
    assert!(inside_differs, "夹具（j）：mask 内必须至少一处不同");
    assert!(
        outside_differs,
        "夹具（j）：mask 外必须至少一处不同（『原样返回 generated』那一档的对照件）"
    );

    let out = composite_masked(&source, &generated, &mask).expect("形状一致必须 Ok");

    assert_eq!(out.width, source.width, "输出的宽必须与源图相同");
    assert_eq!(out.height, source.height, "输出的高必须与源图相同");
    assert_eq!(out.channels, source.channels, "输出的通道数必须与源图相同");

    let channels = source.channels as usize;
    for index in 0..mask.pixels.len() {
        let start = index * channels;
        let end = start + channels;
        if mask.pixels[index] != 0 {
            assert_eq!(
                out.pixels[start..end],
                generated.pixels[start..end],
                "mask 内像素 {index} 必须逐字节等于 generated"
            );
        } else {
            assert_eq!(
                out.pixels[start..end],
                source.pixels[start..end],
                "mask 外像素 {index} 必须逐字节等于 source（强制作）"
            );
        }
    }
}

/// §11.2 第 (k) 条：形状不符 ⇒ `Err(ShapeMismatch { .. })`，**逐对给用例、不抽代表**——
/// 四对各一条：① `mask` 与 `source` 的宽高不同；② `generated` 与 `source` 的宽高不同；
/// ③ `generated` 与 `source` 的**通道数**不同；④ `outside_mask_difference` 的 `result`
/// 与 `source` 的宽高不同。**并判准 `left` 与 `right` 两个字段的值**——掩码那一侧的
/// 第三分量是 `None`（见 Task 1 的 `ShapeMismatch`）。
///
/// **为什么第③对与第①②对同级**（设计 §5.2）：mask 内的像素取自 `generated`，逐点复制
/// 要求二者**逐像素一一对应**；尺寸或通道数不同时「mask 内取自 `generated`」这一步
/// 没有定义，函数的行为是未定义的。**它不是可省略的健全性检查。**
///
/// **`left` / `right` 的取法**：`left` 取 `source` 那一侧、`right` 取被比的另一侧——
/// 设计没给顺序，本实现定（写在 `src/pixels.rs` 的 `ensure_same_shape` 上）。
///
/// **红条件（放宽档）**：删掉第③对的那条判定（`generated` 与 `source` 的比较里不看
/// 通道数、只看宽高）⇒ 第三条红（没有错误可判，走到 `Ok` 臂）、其余三条**仍绿**。
#[test]
fn a_shape_mismatch_is_rejected_on_every_pair() {
    let source = fixture_source();
    let mask = fixture_mask();

    // ① `mask` 与 `source` 的宽高不同：掩码 5×3（15 字节；与源图 4×3 不同）。
    let wider_mask = EditMask {
        width: 5,
        height: 3,
        pixels: vec![0; 15],
    };
    match composite_masked(&source, &source, &wider_mask) {
        Err(ImageError::ShapeMismatch { left, right }) => {
            assert_eq!(
                left,
                (4, 3, Some(2)),
                "①left 必须是 source 的形状（宽、高、通道数）"
            );
            assert_eq!(
                right,
                (5, 3, None),
                "①right 必须是掩码的形状（掩码没有通道维：None）"
            );
        }
        Ok(_) => panic!("①mask 与 source 宽高不同必须 Err，而不是 Ok"),
        Err(other) => panic!("①应为 ShapeMismatch，实际 {other:?}"),
    }

    // ② `generated` 与 `source` 的宽高不同：补丁 4×4（32 字节；比源图高一横行）。
    let taller_generated = Raster {
        width: 4,
        height: 4,
        channels: 2,
        pixels: vec![0; 32],
    };
    match composite_masked(&source, &taller_generated, &mask) {
        Err(ImageError::ShapeMismatch { left, right }) => {
            assert_eq!(left, (4, 3, Some(2)), "②left 必须是 source 的形状");
            assert_eq!(
                right,
                (4, 4, Some(2)),
                "②right 必须是 generated 的形状（宽高不同）"
            );
        }
        Ok(_) => panic!("②generated 与 source 宽高不同必须 Err，而不是 Ok"),
        Err(other) => panic!("②应为 ShapeMismatch，实际 {other:?}"),
    }

    // ③ `generated` 与 `source` 的**通道数**不同：补丁 4×3、3 通道（36 字节）。
    //    这一对与①②同级：通道数不同时「mask 内取自 generated」逐像素一一对应没有定义。
    let three_channel_generated = Raster {
        width: 4,
        height: 3,
        channels: 3,
        pixels: vec![0; 36],
    };
    match composite_masked(&source, &three_channel_generated, &mask) {
        Err(ImageError::ShapeMismatch { left, right }) => {
            assert_eq!(left, (4, 3, Some(2)), "③left 必须是 source 的形状");
            assert_eq!(
                right,
                (4, 3, Some(3)),
                "③right 必须是 generated 的形状（通道数不同）"
            );
        }
        Ok(_) => panic!("③generated 与 source 通道数不同必须 Err，而不是 Ok"),
        Err(other) => panic!("③应为 ShapeMismatch，实际 {other:?}"),
    }

    // ④ `outside_mask_difference` 的 `result` 与 `source` 的宽高不同：结果 3×3（18 字节）。
    let smaller_result = Raster {
        width: 3,
        height: 3,
        channels: 2,
        pixels: vec![0; 18],
    };
    match outside_mask_difference(&source, &smaller_result, &mask) {
        Err(ImageError::ShapeMismatch { left, right }) => {
            assert_eq!(left, (4, 3, Some(2)), "④left 必须是 source 的形状");
            assert_eq!(
                right,
                (3, 3, Some(2)),
                "④right 必须是 result 的形状（宽高不同）"
            );
        }
        Ok(_) => panic!("④result 与 source 宽高不同必须 Err，而不是 Ok"),
        Err(other) => panic!("④应为 ShapeMismatch，实际 {other:?}"),
    }
}

/// §11.2 第 (k2) 条：一个载体的 `pixels` 字节数与它自己声明的形状不符 ⇒
/// `Err(PixelLengthMismatch { len, expected })`——**掩码侧与 `Raster` 侧各一条**、
/// **不抽代表**；**并判准 `len` 与 `expected` 两个字段的值**，且用 `matches!` 断言它
/// **不是 `ShapeMismatch`**。
///
/// **它同时钉住「两臂没被并成一臂」**（设计 §3.1 的四臂判据）：若实现把前置第 3 条并进
/// `ShapeMismatch`，本节第 (k) 条的四对断言**照样绿**，只有本条的臂别断言会红。
///
/// **理由（为什么单出一臂）**：（k）比的是**两个载体之间**的关系，本条比的是
/// **一个载体内部**的两件事——它自己声明的形状与它自己字节序列的长度；而
/// `ShapeMismatch` 的字段是 `(宽, 高, 通道数)`，**两个 `usize` 的长度值表达不进
/// 那个形状**（设计 §5.2 末）。
///
/// **红条件（移除档）**：把这一条并进 `ShapeMismatch` ⇒ 两条的臂别断言红。
#[test]
fn a_pixel_length_mismatch_is_its_own_arm_on_both_sides() {
    let source = fixture_source();
    let mask = fixture_mask();

    // 掩码侧：声明仍是 4×3（与源图宽高一致，形状这一关全过），但只有 11 字节（期望 12）。
    let short_mask = EditMask {
        width: 4,
        height: 3,
        pixels: vec![0; 11],
    };
    let mask_err = composite_masked(&source, &source, &short_mask)
        .expect_err("掩码的字节数与声明的形状不符必须 Err");
    assert!(
        !matches!(&mask_err, ImageError::ShapeMismatch { .. }),
        "臂别（掩码侧）：必须是 PixelLengthMismatch，不得并进 ShapeMismatch"
    );
    match mask_err {
        ImageError::PixelLengthMismatch { len, expected } => {
            assert_eq!(len, 11, "len 必须是掩码实际的字节数");
            assert_eq!(expected, 12, "expected 必须是 宽 × 高 × 1 = 4 × 3");
        }
        other => panic!("应为 PixelLengthMismatch {{ len: 11, expected: 12 }}，实际 {other:?}"),
    }

    // `Raster` 侧：`result` 声明 4×3、2 通道（与源图形状一致），但只有 23 字节（期望 24）。
    let short_result = Raster {
        width: 4,
        height: 3,
        channels: 2,
        pixels: vec![0; 23],
    };
    let raster_err = outside_mask_difference(&source, &short_result, &mask)
        .expect_err("Raster 的字节数与声明的形状不符必须 Err");
    assert!(
        !matches!(&raster_err, ImageError::ShapeMismatch { .. }),
        "臂别（Raster 侧）：必须是 PixelLengthMismatch，不得并进 ShapeMismatch"
    );
    match raster_err {
        ImageError::PixelLengthMismatch { len, expected } => {
            assert_eq!(len, 23, "len 必须是 result 实际的字节数");
            assert_eq!(expected, 24, "expected 必须是 宽 × 高 × 通道数 = 4 × 3 × 2");
        }
        other => panic!("应为 PixelLengthMismatch {{ len: 23, expected: 24 }}，实际 {other:?}"),
    }
}

/// §11.2 第 (l) 条，**两侧**：mask 外全同 ⇒ 差异量为 `0`（**并判总数**）；
/// 把 mask 外**一个**像素改掉 ⇒ 差异量恰为 `1`。**两侧都要**：只钉「全同 ⇒ 0」会让
/// 一个恒返回 `0` 的实现全绿（比较器什么都看不见）。
///
/// **「总数」是哪一种总数（本计划自定，已入计划 `## 遗留` 第五节）**：设计 §5.3
/// （实测在设计 `:510`）逐字只写「返回 mask 外**不相等**的像素数与**总像素数**」，
/// **没有指明后者是整图的像素数还是 mask 外的像素数**（两种读法都通；
/// `docs/spec/03-product-drive.md:832` 的 §168 只给了 `outside-mask pixel difference`
/// 这个检查名，没有给这两个量的定义）。**本计划取「mask 外的像素数」**，理由是
/// 分子与分母同域：分母若换成整图，则「mask 外全同」时读到的比例是 `0 / 整图像素数`，
/// 与「mask 外的改动占比」不是同一个量。**若设计作者取的是另一种读法，本条的期望值
/// 与红条件随之改、而动作不变**（收件人见计划 `## 遗留` 第一节第 11 条）。
///
/// **夹具必须含一个掩码**（mask 外的像素数 ≠ 整图的像素数），否则两种读法在读数上
/// 不可分辨，本条既钉不住本计划的读法、也钉不住另一种。本夹具 mask 内 5 枚、mask 外
/// 7 枚、整图 12 枚——**三种读法在读数上互不相同**（7 / 5 / 12），故「总数」的读法
/// 被钉死。**「差异量」的计数单位是像素**：一个像素在两侧的通道字节序列不同即算一次
/// （第二侧改的是**同一像素的两个通道**——逐字节计数的实现会读到 2，本断言红）。
///
/// **红条件（取反档，两条）**：把计数改成「mask 内」的 ⇒ 本侧红（mask 内的差别被读成
/// 差异量；第二侧同时红——mask 外的改动被漏掉）；把总数写成「整图的像素数」⇒
/// 第一条的总数断言红（期望 7，实际 12）。
#[test]
fn the_comparator_reports_the_difference_and_the_total() {
    let source = fixture_source();
    let mask = fixture_mask();

    // 第一侧：mask 外全同（mask 内有差别也不计入）⇒ 差异量为 0、总数为 mask 外的像素数。
    let result_inside_changed = Raster {
        width: 4,
        height: 3,
        channels: 2,
        pixels: inside_only_change(),
    };
    let all_same_outside =
        outside_mask_difference(&source, &result_inside_changed, &mask).expect("形状一致必须 Ok");
    assert_eq!(
        all_same_outside.differing, 0,
        "mask 外全同 ⇒ 差异量为 0（mask 内的差别不参与计数）"
    );
    assert_eq!(
        all_same_outside.total, 7,
        "总数必须是 mask 外的像素数（12 − 5 = 7），不是整图 12、不是 mask 内 5"
    );

    // 第二侧：mask 外**一个**像素（像素 0）被改掉——两个通道都改，其余逐字节相同。
    let result_one_changed = one_outside_pixel_changed();
    let one_changed =
        outside_mask_difference(&source, &result_one_changed, &mask).expect("形状一致必须 Ok");
    assert_eq!(
        one_changed.differing, 1,
        "mask 外一个像素被改掉 ⇒ 差异量恰为 1（逐像素计数：同一像素的两个通道都改也只计 1）"
    );
    assert_eq!(
        one_changed.total, 7,
        "总数不因改动而变：仍是 mask 外的像素数"
    );
}

/// §5.3 的第二半、§7.3：比较器**不宣告裁决**——返回类型是 `Result<PixelDiff, ImageError>`、
/// **不是 `Result<bool, _>`**；`PixelDiff` 上**没有** `passed()` / `is_ok()` 一类判定方法。
///
/// **照片的落点分两处，写清**：
/// （i）**返回类型**那一半是**编译期照片**——本用例把返回值绑到 `PixelDiff` 上并读它的
/// 字段（`let diff: PixelDiff = …`）；返回类型若是 `Result<bool, _>`，本用例**编不过**
/// （`.expect` 得到的是 `bool`，显式类型标注与读 `diff.differing` 都会失败）。
/// （ii）「`PixelDiff` 上**没有**判定方法」那一半**不是穷尽解构能证的**（解构只钉字段清单，
/// 钉不到方法的有无），它由 **Task 9 Step 3 的源码面复核 (g)**
/// （`grep -rn "Result<bool\|-> bool"`）取。
///
/// **理由写进注释（§5.3）**：返回 `bool` 就把一条判据（阈值）塞进了本块，而 §326 给的是
/// `= FORBIDDEN`（阈值为 0 的那一个特例）、§168 给的是「可以直接确定性检查」——**两者都
/// 不足以让本块替 P5a 定「多少算不过」**。故本块给**量**，判**在 P5a**。
///
/// **红条件（移除档）**：把 `outside_mask_difference` 的返回类型改成
/// `Result<bool, ImageError>` ⇒ 本用例编不过。
#[test]
fn the_comparator_declares_no_verdict() {
    let source = fixture_source();
    let mask = fixture_mask();
    let result = one_outside_pixel_changed();

    // 绑到 `PixelDiff` 上（不是 `bool`）：读到的两个字段都是**量**。
    let diff: PixelDiff =
        outside_mask_difference(&source, &result, &mask).expect("形状一致必须 Ok");
    assert_eq!(
        diff.differing, 1,
        "读到的差异量是 1（量），不是 true / false（结论）"
    );
    assert_eq!(diff.total, 7, "读到的总数是 mask 外的像素数");
}
