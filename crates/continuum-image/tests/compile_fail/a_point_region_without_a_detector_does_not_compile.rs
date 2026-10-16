// 应编译失败：`Point` 那一支没有 detector 就调不动（设计 §4.4 第一条；§11.4 第 1 条）。
//
// 钉的是「没有检测就没有区域」：`Point` 支必须经对象/区域检测（§164 :684 的
// Object / Region Detection），故 `resolve_edit_region` **没有**任何一条「不传
// detector 也能把 `Point` 变成掩码」的路径——少传 `detector` 一个实参，调用在类型上
// 就写不出来。
//
// **为什么与「少传 `source_dims`」分成两份**（另一份：
// `a_box_region_without_the_source_dims_does_not_compile.rs`）：设计 §11.4 第 1 条
// 把它们写成一句话（「少传 `detector`（或少传 `source_dims`）」），但那是**两个不同
// 的实参**缺了——一份样例只能钉住其中一个；两份的 `.stderr` 逐份不同（报的是不同的
// 形参位置），不许拿一个错误码套两处（计划「关于本计划的代码块」的纪律）。
//
// 判据是编译失败，且失败必须落在**实参个数**上（E0061）——不是「没有这个函数」之类的
// 别的错：`resolve_edit_region` 存在，只是收不下三个实参。

use continuum_artifact::ArtifactId;
use continuum_image::{EditRegion, NormalizedAnchor, resolve_edit_region};

fn main() {
    let region = EditRegion::Point(NormalizedAnchor { x: 0.5, y: 0.5 });
    let source = ArtifactId::new("fixture-source");

    // 少传 `detector`：只给 `region` / `source` / `source_dims`。
    let _ = resolve_edit_region(&region, &source, (8, 4));
}
