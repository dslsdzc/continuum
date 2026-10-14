//! §106 的「每个选项带后果」落在**构造期**：不存在「只带选项」的构造路径。
//!
//! 两个面在同一份里（钉的是**同一个记录形态** `DecisionOption`）：
//!
//! 1. `DecisionOption::new` **少给第二个参数**（后果没给）——编不过；
//! 2. **结构体字面量少给 `consequence` 字段**——编不过。
//!
//! 这两个面和「等长由类型保证」是同一件事的两条边：`Decision.options` 的元素只能由
//! 「选项 ＋ 后果」成对构造，故 `options.len() == consequences.len()` 不需要检查。
//! 正控制是 `tests/compile_pass/decision_option_new_takes_option_and_consequence.rs`
//! （两参的 `new` 确实编得过、两个字段确实读得出）。
//!
//! 本样例由 `tests/type_level.rs` 的 `type_level_guarantees_hold` 驱动，与同名
//! `.stderr` **逐字比对**（`.stderr` 是实跑取值，不是手写）。

use continuum_canonical::Opaque;
use continuum_semantics::DecisionOption;

fn main() {
    // (1) 「只带选项」：第一个参数给了、后果没给。
    let _ = DecisionOption::new(Opaque::from_str("使用云 GPU"));
    // (2) 「只带选项」的另一面：字段字面量少 `consequence`。
    let _ = DecisionOption {
        option: Opaque::from_str("使用云 GPU"),
    };
}
