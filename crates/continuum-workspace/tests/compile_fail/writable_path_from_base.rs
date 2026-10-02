// 应编译失败：Base 路径不能构造出可写类型
fn main() {
    let base = continuum_workspace::BaseWorkspace::new("/tmp").unwrap();
    let _ = continuum_workspace::WritablePath::from(base.root());
}
