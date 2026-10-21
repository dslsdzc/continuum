//! Task 4：`AuthorizedTask` 唯一构造点的编译期照片。

#[test]
fn compile_fail() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/compile_fail/*.rs");
    cases.pass("tests/ui_pass/*.rs");
}
