//! C1 / C2 / C4 的编译期照片（设计 §8.2）。
//!
//! - C1：crate 外写 `ProtectedCore` 的全字段字面量 ⇒ 编不过（字段私有）。
//! - C2：crate 外给 `ProtectedCore` 的字段赋值 ⇒ 编不过（字段私有）。
//! - C4：`ProtectedCore::default()` ⇒ 编不过（无 `Default`）。
//!
//! `.stderr` 由实跑生成；换编译器版本或改样例注释都会改这些坐标，必须重取。

#[test]
fn type_level_guarantees_hold() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
}
