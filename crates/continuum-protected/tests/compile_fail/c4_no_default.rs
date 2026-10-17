// 应编译失败：`ProtectedCore` 没有 `Default`，没有「空核心」的默认值。
use continuum_protected::ProtectedCore;

fn main() {
    let _core = ProtectedCore::default();
}
