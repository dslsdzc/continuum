// 应编译失败：GateApproval 的字段私有，crate 外无法直接构造
fn main() {
    let _ = continuum_workspace::GateApproval(());
}
