//! 正控制：`VerificationRequirement` 的**两个公开方法**确实可用——`from_str` 构造、
//! `as_str` 读回（§7.2 的禁读是「没有判定方法」，不是「没有公开面」）。
//!
//! 它是三份 `verification_requirement_has_no_*` 反例（`tests/compile_fail/`）的
//! **正控制**：没有它，那三份的绿可以来自「这个名字整个不存在」这类与判据无关
//! 的原因（恒绿的守卫比没有更坏）。
//!
//! 本样例**应编得过**——由 `tests/type_level.rs` 的 `type_level_guarantees_hold`
//! 里的 `t.pass` 驱动（`compile_pass/` 一侧没有 `.stderr`）。

use continuum_semantics::VerificationRequirement;

fn main() {
    let vr = VerificationRequirement::from_str("只存不判，但存下来的这一串要读得到");
    // 公开方法集只有这两个（加 `Debug` / `Clone` 两个非语义派生）。
    let _: &str = vr.as_str();
}
