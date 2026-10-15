//! §7.2 的**结构性禁读**：`VerificationRequirement` **没有** `is_satisfied`。
//!
//! 《工程》§2.5：该字段可以存储，但在 OPEN-001 解决前**无法判定其是否被满足**
//! （「这不阻断本层建设，阻断的是第 8 层的验证收口」）。故本层只存不判——
//! 补一个满足判据就是发明判据（OPEN-001 的三个可选方向 a/b/c 一个都不选）。
//!
//! 本样例由 `tests/type_level.rs` 的 `type_level_guarantees_hold` 驱动，与同名
//! `.stderr` **逐字比对**（`.stderr` 是实跑取值，不是手写）——「因为拼错函数名而
//! 编译失败」不会让它变绿：比对的是预期的那条错。
//!
//! **它钉的只是这一个名字**：方法名是开放集，`trybuild` 枚举不完。「本层没有
//! 以它为输入做判定的函数」那半句的可判形态在 `tests/contract.rs` 的出现点清单
//! 与 Task 21 的复核里。

use continuum_semantics::VerificationRequirement;

fn main() {
    let vr = VerificationRequirement::from_str("只存不判");
    let _ = vr.is_satisfied();
}
