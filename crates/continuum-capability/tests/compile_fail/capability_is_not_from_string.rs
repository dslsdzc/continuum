// 应编译失败：`Capability` 没有 `FromStr` —— 字符串不能还原出能力
// （设计 §2.3 第三条：它与 `Display` 只出不进）。
fn main() {
    let _: continuum_capability::Capability = "git.push:origin/main".parse().unwrap();
}
