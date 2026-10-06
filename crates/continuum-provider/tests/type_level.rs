//! 类型层保证的编译失败样例驱动器（设计 §11）。
//!
//! 「工具侧的适配器出不了 crate」「crate 外构造不出 `AuthorizedToolInvocation`」
//! 「裸 `ToolId` 递不进 `invoke_tool`」是**不可表达性**命题：运行期用例无论怎么写
//! 都只能证明「我没这么做」，证明不了「做不到」。故用 `trybuild` 的编译失败用例。
//! 每个 `compile_fail/*.rs` 配一份同名 `.stderr`，把失败钉在预期的报错上——
//! 否则「因为拼错函数名而编译失败」也会让用例变绿。
//!
//! **globs 覆盖 `tests/compile_fail/` 下的每一份 `.rs`**：新增样例不需要改本文件，
//! 但也就意味着**新增样例必须自带 `.stderr`**，否则它会以「缺 `.stderr`」的形态红。
//!
//! **证明力的边界**（设计 §11 末段）：这些样例钉的是**写下来的那几个拼法**，
//! 不是「任何名字的公开入口都不存在」这一全称否定。故判据是**公开面清单 + 样例**
//! 两条一起，清单由评审核对。

#[test]
fn type_level_guarantees_hold() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
}
