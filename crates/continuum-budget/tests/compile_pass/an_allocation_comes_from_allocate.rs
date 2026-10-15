// 应编得过（正控制）：`Allocation` 由**分配方**给出——设计指定的那个入口可用。
//
// 没有它，`allocation_has_no_public_constructor` 与
// `allocation_is_not_the_other_two_roles` 的绿可以来自「把入口删光」这类与判据
// 无关的坏法（恒绿的守卫比没有更坏）。**六条合起来才算钉住**：三反例 ＋ 三正控制。
//
// 只在**编译期**证明签名可用（trybuild 的 pass 样例会被编译并运行 `main`；
// 本样例的 `main` 不调用任何预算函数、不建库——`entry` 只被引用，不执行）。
use continuum_budget::{Allocation, BudgetOwner, Dimensions, allocate};
use continuum_persist::Tx;
use continuum_semantics::IntentId;

fn entry(tx: &Tx<'_>) -> Allocation {
    let parent = BudgetOwner::SystemMaintenance;
    let child = BudgetOwner::Intent(IntentId::from_str("i-1"));
    let dims = Dimensions {
        money: Some(10),
        wall_time: None,
        token: None,
        gpu_time: None,
        network_transfer: None,
    };
    allocate(tx, parent, child, dims, 0).unwrap()
}

fn main() {
    // 引用 `entry`（避免 dead_code），不调用它。
    let _: fn(&Tx<'_>) -> Allocation = entry;
}
