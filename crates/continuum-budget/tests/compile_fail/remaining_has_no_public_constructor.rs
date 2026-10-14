// 应编译失败：`Remaining` 的内部字段**私有**、**无公开构造函数**——crate 外
// 造不出一枚（唯一构造入口是 `remaining()`，它**现算**：`allocation − settled −
// reserved` 逐维）。
//
// 判据与 `allocation_has_no_public_constructor` 逐字相同：**失败必须落在私有性上**。
//
// **逐角色各一条**的理由同 `allocation_has_no_public_constructor` 的注释——
// 这里再点一次最要紧的那半：**若 `Remaining` 后来开了公开构造函数而 `Allocation`
// 没有，一条合并的样例仍会过**，故三份一个不少。
use continuum_budget::{Dimensions, Remaining};

fn main() {
    let dims = Dimensions {
        money: Some(0),
        wall_time: None,
        token: None,
        gpu_time: None,
        network_transfer: None,
    };
    // 元组构造形式——字段私有，编译不过。
    let _ = Remaining(dims);
}
