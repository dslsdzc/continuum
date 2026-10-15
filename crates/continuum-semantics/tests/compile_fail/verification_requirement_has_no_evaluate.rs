//! §7.2 的**结构性禁读**：`VerificationRequirement` **没有** `evaluate`。
//!
//! 与另两份 `verification_requirement_has_no_*` 同一条禁令、第三个名字——
//! 三份合起来把「公开方法集只有 `as_str` / `from_str`」钉在三个具体名字上
//! （方法名是开放集，枚举不完；限度写在 `tests/type_level.rs` 的文件头）。
//!
//! 本样例由 `tests/type_level.rs` 的 `type_level_guarantees_hold` 驱动，与同名
//! `.stderr` **逐字比对**（`.stderr` 是实跑取值，不是手写）。

use continuum_semantics::VerificationRequirement;

fn main() {
    let vr = VerificationRequirement::from_str("只存不判");
    let _ = vr.evaluate(&vr);
}
