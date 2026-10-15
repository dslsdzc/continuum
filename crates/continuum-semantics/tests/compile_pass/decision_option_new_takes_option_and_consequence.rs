//! 正控制（与 `a_decision_option_cannot_be_built_without_a_consequence` 对钉）：
//! `DecisionOption::new(option, consequence)` **编得过**。
//!
//! §106 的「每个选项带后果」是「两个都要」，**不是**「没有公开面」——若有人把
//! `new` 改成一参、或把字段删光让上面那份反例「因为 API 整体没了」而绿，
//! 本样例即红。两个字段的读取（`option` / `consequence`）也一并证明可用。

use continuum_canonical::Opaque;
use continuum_semantics::DecisionOption;

fn main() {
    let option = Opaque::from_str("使用云 GPU");
    let consequence = Opaque::from_str("¥8–12，保持 4K");
    let pair = DecisionOption::new(option, consequence);
    assert_eq!(pair.option.as_str(), "使用云 GPU");
    assert_eq!(pair.consequence.as_str(), "¥8–12，保持 4K");
}
