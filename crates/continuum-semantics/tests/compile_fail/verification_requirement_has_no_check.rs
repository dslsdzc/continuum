//! §7.2 的**结构性禁读**：`VerificationRequirement` **没有** `check`。
//!
//! 与 `verification_requirement_has_no_is_satisfied.rs` 同一条禁令、另一个名字：
//! 判定方法的名字（`check` / `evaluate` 这类）逐枚编不过，才是「公开方法集只有
//! `as_str` / `from_str`」的可判形态（只钉一个名字会漏掉其余）。
//!
//! 本样例由 `tests/type_level.rs` 的 `type_level_guarantees_hold` 驱动，与同名
//! `.stderr` **逐字比对**（`.stderr` 是实跑取值，不是手写）。

use continuum_semantics::VerificationRequirement;

fn main() {
    let vr = VerificationRequirement::from_str("只存不判");
    let _ = vr.check(&vr);
}
