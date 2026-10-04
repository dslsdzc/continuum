//! 类型层只读保证的唯一直接证据（设计 §2.3、§8）。
//!
//! 「`full_access` 写不出来」「字符串还原不出能力」是**不可表达性**命题：运行期
//! 测试无论怎么写都只能证明「我没这么做」，证明不了「做不到」。故用 `trybuild`
//! 的编译失败用例。每个 `compile_fail/*.rs` 配一份同名 `.stderr`，把失败钉在
//! 预期的报错上——否则「因为拼错函数名而编译失败」也会让用例变绿。

#[test]
fn capability_cannot_be_built_outside_its_issuing_point() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
}
