//! 集成测试共用的脚手架。
//!
//! `tests/gate.rs` 与 `tests/backend_overlay.rs` 是两个各自独立的测试 crate，取不到对方的
//! 东西；共用的部分放这里，两边各以 `mod common;` 引进来。
//!
//! **单测模块（`src/gate.rs` 的 `mod tests`）用不上这一份**——那是另一个 target（被测 crate
//! 自身的 lib test），取不到 `tests/` 下的模块。那里有一条**同形**的守卫，注释里互相指认；
//! 改这里的判定条件时，那一处要一并改。

/// 核对子进程真的跑过那一条用例。
///
/// **只看退出码不够。** libtest 在过滤器对不上任何名字时以「0 tests run」**退出 0**，于是
/// 父进程照退出码报通过，而那条用例一次断言都没执行过——静默变绿。调用方是按名字传参的
/// （`enter_namespace("some_test", …)`），**一个拼写错误就会触发它**。
///
/// 实测踩到过：单测二进制里的用例全名带模块路径（`gate::tests::<函数名>`，而集成测试的用例
/// 名是平的、没有前缀），`--exact` 匹配不上，用例跑 0 条、父进程报 ok、耗时 0.01s——把断言
/// 改成必然为假也照样 ok。故这里要求子进程的输出里有 `1 passed`。
pub fn assert_child_ran_one(test_name: &str, stdout: &str) {
    assert!(
        stdout.contains("1 passed"),
        "子进程没有执行 {test_name}（过滤器对不上时 libtest 以 0 tests 退出 0，本用例会\
         静默变绿）。子进程输出：\n{stdout}"
    );
}
