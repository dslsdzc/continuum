// 应编译失败：`Box` 那一支没有源图尺寸就投不出掩码（设计 §4.4 第一条；§11.4 第 1 条）。
//
// 钉的是「没有源图尺寸就投不出掩码」：`Box` 支要把归一化矩形投影成像素掩码
// （`src/resolve.rs` 的 `project_box`），而投影的坐标系就是源图的画布；`source_dims`
// 是必需形参——少传它、只把 detector 传在第 3 个位置之后，调用在类型上写不出来。
//
// **为什么与「少传 `detector`」分成两份**（另一份：
// `a_point_region_without_a_detector_does_not_compile.rs`）：设计 §11.4 第 1 条
// 把它们写成一句话（「少传 `detector`（或少传 `source_dims`）」），但那是**两个不同
// 的实参**缺了——一份样例只能钉住其中一个；两份的 `.stderr` 逐份不同（报的是不同的
// 形参位置），不许拿一个错误码套两处（计划「关于本计划的代码块」的纪律）。
//
// 判据是编译失败，且失败必须落在**实参**上——不是「没有这个函数」之类的别的错：
// `resolve_edit_region` 存在，第 3 个实参（detector）落在了 `(u32, u32)` 的位置。

use continuum_artifact::ArtifactId;
use continuum_image::{
    EditMask, EditRegion, NormalizedAnchor, NormalizedBox, RegionDetector, SemanticObjectRef,
    resolve_edit_region,
};

/// 桩：只为本样例占住「detector 该在的位置」——样例钉的是 `source_dims` 缺了，
/// detector 的值不参与那条命题。
struct DetectorStub;

impl RegionDetector for DetectorStub {
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

fn main() {
    let region = EditRegion::Box(NormalizedBox {
        p: NormalizedAnchor { x: 0.25, y: 0.25 },
        q: NormalizedAnchor { x: 0.75, y: 0.75 },
    });
    let source = ArtifactId::new("fixture-source");
    let detector = DetectorStub;

    // 少传 `source_dims`：detector 落在了 `(u32, u32)` 该在的第 3 个位置。
    let _ = resolve_edit_region(&region, &source, &detector);
}
