// 应编译失败：`Capability` 没有公开构造函数，唯一产出路径是 `mint`
// （设计 §2.3 第三条：字段私有、无公开构造函数）。
fn main() {
    let _ = continuum_capability::Capability::new(
        continuum_capability::CapabilityKind::Git(continuum_capability::GitAction::Push),
        String::from("origin/main"),
        0,
    );
}
