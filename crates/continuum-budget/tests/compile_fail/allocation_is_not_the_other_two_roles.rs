// 应编译失败：三个角色三个类型（§12.1）——`Allocation` 当不了另外两个角色。
//
// 样例内两条：`Allocation` 放进 `Reservation` 的位置、放进 `Remaining` 的位置
// ——两处都**类型不匹配**（E0308 那一类；不是私有性错误：**类型不同**才是本条
// 钉的判据）。两处各用一个函数调用取 `Allocation` 值（不写 `todo!()` 表达式
// ——那会让后一处被标成 unreachable，给 `.stderr` 添与本判据无关的警告行）。
//
// # 为什么逐角色各一条（同 `allocation_has_no_public_constructor` 的要害）
//
// 三合一（或抽一个代表）时，其中一个角色漂了照样绿。
//
// # `the_three_roles_are_three_types` 的落点
//
// 本条与 `reservation_is_not_the_other_two_roles` / `remaining_is_not_the_other_two_roles`
// 三份合起来就是 plan 点名的 `the_three_roles_are_three_types`；正控制三条在
// `tests/compile_pass/`（三者的入口各自可用）。
//
// # 编译期照片，运行期无照片（纯数据声明）
//
// 三个 newtype 各包一个 `Dimensions`，**类型不共用**——故「把余量填进上限的位置」
// 在**类型上**不可写。这是编译期命题，没有运行期照片。
use continuum_budget::{Allocation, Remaining, Reservation};

/// 取值位置用的函数（只被 `main` 调用、不运行——样例要的是**类型**）。
fn an_allocation() -> Allocation {
    todo!()
}

fn main() {
    // 第一处：`Allocation` 当 `Reservation` 用。
    let _: Reservation = an_allocation();

    // 第二处：`Allocation` 当 `Remaining` 用。
    let _: Remaining = an_allocation();
}
