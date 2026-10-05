// 应编译失败：`Capability` 没有 `From<&str>`。
// 与 `capability_is_not_from_string.rs` 是**两条独立的通道**（`From` 与 `FromStr`），
// 各需一张照片——设计 §2.3 第三条把两者并列点名，只钉住一条会漏掉另一条。
fn main() {
    let _ = continuum_capability::Capability::from("git.push:origin/main");
}
