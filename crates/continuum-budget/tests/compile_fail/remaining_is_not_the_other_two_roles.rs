// 应编译失败：三个角色三个类型（§12.1）——`Remaining` 当不了另外两个角色。
//
// 样例内两条：`Remaining` 放进 `Allocation` 的位置、放进 `Reservation` 的位置
// ——两处都**类型不匹配**（E0308 那一类）。见同目录
// `allocation_is_not_the_other_two_roles` 的注释（逐角色各一条的理由与落点）。
use continuum_budget::{Allocation, Remaining, Reservation};

/// 取值位置用的函数（只被 `main` 调用、不运行——样例要的是**类型**）。
fn a_remaining() -> Remaining {
    todo!()
}

fn main() {
    // 第一处：`Remaining` 当 `Allocation` 用。
    let _: Allocation = a_remaining();

    // 第二处：`Remaining` 当 `Reservation` 用。
    let _: Reservation = a_remaining();
}
