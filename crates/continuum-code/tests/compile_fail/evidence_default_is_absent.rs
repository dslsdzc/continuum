// 应编译失败：`Evidence` **没有 `Default` 实现**，故 `Evidence::default()` 这条路
// 走不通（预期是「没有这个关联函数」那一类，常见码 `E0599`；**判据是实跑产出的那份
// `.stderr` 本身**，不是这个码）。
// **文件名即这条通道——本样例只钉「没有无参构造」这一条。**
//
// 它与 `evidence_literal_is_private`（兄弟样例）是**两条不同的编译失败通道**：
// 那份钉「字段私有」（绕过 `from_tool_result` 的写入口），本份钉「没有无参构造」
// （连拼字面量之外的另一条捷径也不存在）。**不许并成一份**——两条通道的失效条件
// 不同：字段若被开成 `pub`，那份样例会编译通过；别处若给 `Evidence` 加了
// `Default`，本份样例会编译通过。
//
// 它钉的仍是 §4.4 的「唯一产生点」是**结构事实**（P5a `src/evidence.rs` 的派生清单：
// 禁止 `Default` / `From` / `FromStr` / `Serialize` / `Deserialize`——`Default` 是一条
// 无参构造路径，加它等于把「唯一」变假）。
//
// **注释与代码一样是照片的一部分**：`trybuild` 把本文件逐行打印进
// `evidence_default_is_absent.stderr` 的输出里，故**改一句注释就会改那张照片**——
// 改任一处都要重跑取新值（见 `tests/type_level.rs` 文件头）。
use continuum_verify::Evidence;

fn main() {
    // §4.4：没有这条无参构造捷径。
    let _ = Evidence::default();
}
