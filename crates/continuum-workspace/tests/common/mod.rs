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
        stdout.contains(" 1 passed"),
        "子进程没有执行 {test_name}（过滤器对不上时 libtest 以 0 tests 退出 0，本用例会\
         静默变绿）。子进程输出：\n{stdout}"
    );
}

/// 环境不具备本用例所需条件时的**显式**跳过标记，带本条的执行/跳过条数。
///
/// 不静默通过：cargo 在用例通过时不回显 stderr，用 `--nocapture` 运行即可见。
/// 改判为 `panic!` 会把「本机能力不具备」误报成「实现有缺陷」——见设计遗留里
/// 「overlay 的跳过窗口」一条：无 `unshare` 的机器上这些用例会全部跳过而套件仍绿，
/// 处置是**读数时可见**（条数写在标记里），不是把窗口换成红色。
pub fn skip(test: &str, reason: &str) {
    eprintln!("【跳过】{test}：{reason}。本条：执行 0、跳过 1。用 `--nocapture` 可见本行。");
}

/// 与 [`skip`] 对称：本条真的在子进程里跑过了（含断言）。
///
/// 两行合起来是「需要命名空间的用例这次到底跑了几条」的读数凭据；本机为
/// `执行 1、跳过 0`（`src/gate.rs` 的单测模块里有一条同形的标记）。
pub fn ran(test: &str) {
    eprintln!("【运行】{test}：本条：执行 1、跳过 0。");
}
