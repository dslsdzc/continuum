// 应编译失败（J1-a）：`ContextPackage` 的六个字段私有，crate 外不能写字面量。
// 样例不 `use` 任何 P6B／P6C 的类型，失败只应来自私有字段。
use continuum_context::ContextPackage;

fn main() {
    let _ = ContextPackage {
        instructions: vec![],
        references: vec![],
        artifacts: vec![],
        memories: vec![],
        provenance: vec![],
        trust_labels: vec![],
    };
}
