// 应编译失败：`Allocation` 的内部字段**私有**、**无公开构造函数**——crate 外
// **造不出一枚**（唯一构造入口是分配方：`record_root` / `allocate`，见 `src/tree.rs`）。
//
// 形态照 P3A 对 `AuthorizedTool` 的样例与 `continuum-semantics` 的
// `a_specification_cannot_be_constructed_outside_the_crate`：判据是
// **失败必须落在私有性上**，不是拼错名字之类的别的错。错误码以实跑取值为准
// （`tests/type_level.rs` 的 `.stderr` 逐字比对）。
//
// # 为什么逐角色各一条（那是本条的要害）
//
// **三合一（或抽一个代表）时，其中一个角色漂了照样绿**——例如 `Remaining` 后来
// 开了公开构造函数而 `Allocation` 没有，一条合并的样例仍会过。这与「枚举断言
// 须逐项有照片」是同一条纪律，故**三个用例名、三对文件（每份各配一个 `.stderr`），
// 一个不少**。
//
// # 断言的作用域与时点（写死）
//
// 「除设计指定的那个入口外无第二个公开**构造路径**」是**在本 task 结束时**由
// 六份样例钉住的；「三个角色私有、无公开构造函数」这一句在 P4 落地前只是**设计
// 写死的承诺**（代码层面当时只能核「设计承诺了」），**本 task 是它第一次有代码层的照片**。
//
// # 「构造」与「读」要分清
//
// 本断言**只管构造**——三个角色的**读法**（`dims()`、`Remaining` 的逐维读者）
// 是公开的，它们**不构成**第二条构造路径（读不出可写的角色值）。判据是**类型**：
// `dims()` 返回 `&Dimensions`（不是角色），故拿它拼不出一个 `Allocation`。
use continuum_budget::{Allocation, Dimensions};

fn main() {
    let dims = Dimensions {
        money: Some(1),
        wall_time: None,
        token: None,
        gpu_time: None,
        network_transfer: None,
    };
    // 元组构造形式——字段私有，编译不过。
    let _ = Allocation(dims);
}
