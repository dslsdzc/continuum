//! C3 / C5 的编译期照片（设计 §8.2）。
//!
//! - C3：`LearningProposal::new` 的 target 收 `ProtectedSemantic` ⇒ 编不过。
//! - C5（正控制）：`core.authority_rules()`、`core.fingerprint()` 与一条合法的
//!   `LearningProposal::new` ⇒ 编得过。
//!
//! C5 是必需的：只有 `compile_fail` 时，一个把只读面也封死的 crate 会让 C1–C4 全绿。
//! `.stderr` 由实跑生成；换编译器版本或改样例注释都会改坐标，必须重取。

#[test]
fn type_level_guarantees_hold() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
    t.pass("tests/ui_pass/*.rs");
}
