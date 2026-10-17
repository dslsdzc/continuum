// 正控制：`Evidence::from_tool_result` 的**函数项**在本 crate 的编译环境里够得着
// ——**不构造值、不传八个实参**（设计 §6.2；计划 Task 6 的「正控制」段）。
//
// **用途只有一个**：把「因为依赖没接上而编译失败」与「因为字段私有而编译失败」分开
// ——没有本样例，`tests/compile_fail/` 两份样例的红可能是任何一种红。若 dev 边
// `continuum-verify` 没接上，本样例会**编译失败**、`trybuild` 当场报红（实跑形态见
// 报告与 `.tmp/` 的变异日志）；本样例绿、兄弟样例按预期红，两张照片才钉在「字段私有」
// 上，而不是「找不到 crate」上。
//
// 这里只取函数项：`Evidence` 类型可命名、`from_tool_result` 从 crate 外可见。
use continuum_verify::Evidence;

fn main() {
    // 只取函数项——不求值、不传参、不构造任何 `Evidence` 值。
    let _ = Evidence::from_tool_result;
}
