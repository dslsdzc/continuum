//! Task 8 的编译期用例：`ContextPackage` 的唯一构造点。
//!
//! 四份 `tests/compile_fail/*.rs`（应编不过）＋一份 `tests/ui_pass/*.rs`（应编得过）。
//! 每份反例配同名 `.stderr`，由 `trybuild` 逐字比对；`.stderr` 取自实跑。
//! 正控制的用途只有一个：把「依赖没接上而编译失败」与「字段私有而编译失败」分开。

#[test]
fn the_package_has_no_second_construction_path_from_outside() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
    t.pass("tests/ui_pass/*.rs");
}
