//! 类型层只读保证的唯一直接证据（设计第 4.1 节）。
//!
//! 「Base 路径构造不出 `WritablePath`」是**不可构造性**命题：运行期测试无论怎么写
//! 都只能证明「我没这么做」，证明不了「做不到」。故用 `trybuild` 的编译失败用例。
//! 每个 `compile_fail/*.rs` 配一份同名 `.stderr`，把失败钉在预期的报错上——
//! 否则「因为拼错函数名而编译失败」也会让用例变绿。

#[test]
fn base_path_cannot_be_made_writable() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
}
