// 应编译失败：§284 的五个 scope 里没有 `Session`。
// 样例：memory_scope_has_no_session（C3）。
fn main() {
    let _ = continuum_memory::MemoryScope::Session;
}
