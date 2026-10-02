// 应编译失败：BaseWorkspace 没有 writable_root
fn main() {
    let base = continuum_workspace::BaseWorkspace::new("/tmp").unwrap();
    let _ = base.writable_root();
}
