//! Task 10：`SessionPlan` 的编译期形状与反序列化负例。

#[test]
fn type_level() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/compile_fail/session_plan_*.rs");
    cases.pass("tests/ui_pass/session_plan_is_reachable.rs");
}
