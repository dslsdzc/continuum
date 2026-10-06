// 应编译失败：`TaskSkillRequirement` 的字段私有，故**结构体字面量**这条路在 crate 外走不通
// （E0451）。这是「非空」那条守卫（设计 §5.1 末段）的**结构面**照片之一：
// 绕过 `try_new`（空集合在那里被拒）自己拼一个空需求，在类型层就不可能。
//
// **本样例只钉「字段私有」这一条通道**；「没有 `From<Vec<_>>` 便利转换」是另一条通道，
// 由 `a_requirement_cannot_be_built_by_conversion.rs` 单独钉。**两条通道必须分成两份文件**，
// 这不只是风格：实测（rustc 1.95）把两条错写进同一个函数体时，
// **rustc 只报第一条，第二条根本不出现**——那份 `.stderr` 里就只有一段错，
// 「另一条通道被打开」不会有任何可观测的差别（＝那条通道没有照片）。
// 分开之后：把 `dimensions` 改 `pub` 只让本文件变红，加一个 `impl From<Vec<SkillDimension>>`
// 只让那一份变红——反之亦然。
//
// 若本文件因**别的**缘故失败（名字拼错、导入缺失、`SkillDimension` 走错路径），
// `.stderr` 里就会出现不是 E0451 的错，比对不过——这正是 `.stderr` 存在的理由。
use continuum_model_registry::{SkillDimension, TaskSkillRequirement};

fn main() {
    let _ = TaskSkillRequirement {
        dimensions: vec![SkillDimension::Coding],
    };
}
