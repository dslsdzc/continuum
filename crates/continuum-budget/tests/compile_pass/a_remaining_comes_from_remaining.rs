// 应编得过（正控制）：`Remaining` 由 `remaining()` 给出——设计指定的那个入口可用。
//
// 没有它，`remaining_has_no_public_constructor` 与
// `remaining_is_not_the_other_two_roles` 的绿可以来自「把入口删光」这类与判据
// 无关的坏法。**六条合起来才算钉住**：三反例 ＋ 三正控制。
//
// 只在**编译期**证明签名可用（`main` 不调用任何预算函数、不建库）。
use continuum_budget::{BudgetOwner, Remaining, remaining};
use continuum_persist::Tx;
use continuum_semantics::IntentId;

fn entry(tx: &Tx<'_>) -> Remaining {
    let owner = BudgetOwner::Intent(IntentId::from_str("i-1"));
    remaining(tx, &owner).unwrap()
}

fn main() {
    let _: fn(&Tx<'_>) -> Remaining = entry;
}
