// 应编译失败（J1-b）：`ContextPackage` 没有 `Default`（也没有 inherent 的 `default`）。
// 任何无参构造都会成为第二个构造点。
use continuum_context::ContextPackage;

fn main() {
    let _ = ContextPackage::default();
}
