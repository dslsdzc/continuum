// 应编译失败：`MemoryClaim` 的七个字段之外没有第八个可读字段。
// 样例：claim_has_no_status_or_anchor_accessor（C1）。
// 判据是实跑产出的同名 `.stderr`，不是这里写的码。
fn read_extra(c: &continuum_memory::MemoryClaim) {
    let _ = c.status();
    let _ = c.anchor();
}

fn main() {
    let _ = read_extra;
}
