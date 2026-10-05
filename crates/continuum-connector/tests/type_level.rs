//! 连接器边界层的类型层只读保证（设计 §3.2 第 4 条、§5.1）。
//!
//! 两份样例钉的都是**不可表达性**命题，运行期用例证明不了：
//! - 「`Effect` 臂只收 `AuthorizedEffect`」——裸 `Capability` 填不进去；
//! - 「绑定没有 `effect` 字段」——效应是**推出来的**，不是声明的。
//!
//! 每个 `compile_fail/*.rs` 配一份同名 `.stderr`，把失败钉在预期的报错上——否则
//! 「因为拼错函数名而编译失败」也会让用例变绿。

/// 名字描述的是这组样例共同守住的东西（类型层保证），不是其中某一条的来历。
/// 与 `continuum-capability` / `continuum-secrets` 的 `tests/type_level.rs` 同形。
#[test]
fn type_level_guarantees_hold() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
}
