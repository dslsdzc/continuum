//! 密钥运行时的类型层只读保证（设计 §5.2、§8）。
//!
//! 「拿窄能力要不到宽凭据」与「没有取全部的入口」是**不可表达性**命题：运行期用例
//! 无论怎么写都只能证明「我没这么要过」，证明不了「要不到」。故用 `trybuild` 的编译
//! 失败用例。每个 `compile_fail/*.rs` 配一份同名 `.stderr`，把失败钉在预期的报错上
//! ——否则「因为拼错函数名而编译失败」也会让用例变绿。

#[test]
fn type_level_guarantees_hold() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
}
