// 应编译失败：§253 禁止的那个形状**字面照抄**也写不出来 —— `Capability` 上
// 没有 `full_access` 这个字段（设计 §2.3 第一条：不是「禁止作默认」，是没有这个字段）。
// 与 `capability_has_no_full_access.rs` 不是重复：那条钉的是词汇表里的成员，
// 这条钉的是 §253 原文给的字面形状。
fn main() {
    let _ = continuum_capability::Capability { full_access: true };
}
