// 应编译通过（正控制）：把「依赖没接上而编译失败」与「字段私有而编译失败」分开。
// 只接收 `&ContextPackage` 并调用只读访问器，不构造值。
use continuum_context::ContextPackage;

fn read_instructions(package: &ContextPackage) -> usize {
    package.instructions().len()
}

fn main() {
    // 函数存在即可；不构造 `ContextPackage`。
    let _ = read_instructions;
}
