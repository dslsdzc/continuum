// 应编得过（正控制）：`Reservation` 由 `reserve` 给出——设计指定的那个入口可用。
//
// 没有它，`reservation_has_no_public_constructor` 与
// `reservation_is_not_the_other_two_roles` 的绿可以来自「把入口删光」这类与判据
// 无关的坏法。**六条合起来才算钉住**：三反例 ＋ 三正控制。
//
// 只在**编译期**证明签名可用（`main` 不调用任何预算函数、不建库）。
use continuum_budget::{BudgetOwner, Dimensions, Reservation, reserve};
use continuum_persist::Tx;
use continuum_semantics::IntentId;

fn entry(tx: &Tx<'_>) -> Reservation {
    let owner = BudgetOwner::Intent(IntentId::from_str("i-1"));
    let dims = Dimensions {
        money: Some(1),
        wall_time: None,
        token: None,
        gpu_time: None,
        network_transfer: None,
    };
    reserve(tx, owner, dims, 0).unwrap()
}

fn main() {
    let _: fn(&Tx<'_>) -> Reservation = entry;
}
