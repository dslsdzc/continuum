//! §9.2 的 trybuild 驱动：显式路径，不用通配。

#[test]
fn type_level_guarantees_hold() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/no_default.rs");
    t.compile_fail("tests/compile_fail/no_ord.rs");
    t.pass("tests/ui_pass/control.rs");
}
