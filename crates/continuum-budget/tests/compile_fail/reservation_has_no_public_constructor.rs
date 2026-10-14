// 应编译失败：`Reservation` 的内部字段（`owner` / `dims`）**私有**、
// **无公开构造函数**——crate 外造不出一枚（唯一构造入口是 `reserve`）。
//
// 判据与 `allocation_has_no_public_constructor` 逐字相同：**失败必须落在私有性上**。
// `Reservation` 的里子是 `owner` ＋ `dims` 两项（不是只包一个 `Dimensions`）：
// `settle` 的入参只有 `reservation` 与 `actual`（设计 §12.2 写死），**没有 owner 就
// 写不了结算账**，故 owner 随预扣走——这是落地时的一处申报，见 `src/account.rs`。
//
// **逐角色各一条**的理由同 `allocation_has_no_public_constructor` 的注释
// （三合一或抽代表时，其中一个角色漂了照样绿）。
use continuum_budget::{BudgetOwner, Dimensions, Reservation};
use continuum_semantics::IntentId;

fn main() {
    let owner = BudgetOwner::Intent(IntentId::from_str("i-1"));
    let dims = Dimensions {
        money: Some(1),
        wall_time: None,
        token: None,
        gpu_time: None,
        network_transfer: None,
    };
    // 结构体字面量形式——两个字段都私有，编译不过。
    let _ = Reservation { owner, dims };
}
