// 应编译失败：§253 禁止的 `full_access` 形状在词汇表里没有对应成员
// （设计 §2.3 第一条：不是「禁止作默认」，是没有这个成员）。
fn main() {
    let _ = continuum_capability::CapabilityKind::FullAccess;
}
