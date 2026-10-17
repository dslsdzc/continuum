// 应编译失败：`Confidence` 没有 `PartialOrd`（规范没有给档位序）。
// 样例：confidence_has_no_order（C6）。
use continuum_memory::Confidence;

fn compare(a: Confidence, b: Confidence) {
    let _ = a < b;
}

fn main() {
    let _ = compare;
}
