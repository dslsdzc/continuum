// 应编译失败：`Capability` 的字段私有，结构体字面量构造被拒
// （设计 §2.3 第三条：唯一产出路径是 `mint`）。
use continuum_capability::{Capability, CapabilityKind, GitAction, Issuer};

fn main() {
    let _ = Capability {
        kind: CapabilityKind::Git(GitAction::Push),
        scope: String::from("origin/main"),
        expiry: 0,
        issuer: Issuer::PolicyWithExplicitApproval,
    };
}
