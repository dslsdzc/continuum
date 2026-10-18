//! 编译期照片的唯一驱动：`tests/compile_fail/` 下的样例逐个必须编不过，
//! 且错误文本与 `.stderr` 一致。`.stderr` 由 `TRYBUILD=overwrite` 实跑取得。

#[test]
fn compile_failures_are_photos() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
}
