// 应编译失败：三个角色三个类型（§12.1）——`Reservation` 当不了另外两个角色。
//
// 样例内两条：`Reservation` 放进 `Allocation` 的位置、放进 `Remaining` 的位置
// ——两处都**类型不匹配**（E0308 那一类）。见同目录
// `allocation_is_not_the_other_two_roles` 的注释（逐角色各一条的理由与落点）。
use continuum_budget::{Allocation, Remaining, Reservation};

/// 取值位置用的函数（只被 `main` 调用、不运行——样例要的是**类型**）。
fn a_reservation() -> Reservation {
    todo!()
}

fn main() {
    // 第一处：`Reservation` 当 `Allocation` 用。
    let _: Allocation = a_reservation();

    // 第二处：`Reservation` 当 `Remaining` 用。
    let _: Remaining = a_reservation();
}
