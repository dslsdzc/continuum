// 应编译失败（J2 的编译期半）：字段私有，crate 外写不进去。
fn main() {
    let mut package: continuum_context::ContextPackage = todo!();
    package.instructions = vec![];
}
