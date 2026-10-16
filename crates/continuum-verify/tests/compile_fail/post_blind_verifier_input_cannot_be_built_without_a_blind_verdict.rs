// 应编译失败：`PostBlindVerifierInput::new` 的第一个参数是 `BlindVerdict`，
// 只给「完成声明」一个实参 ⇒ 函数参数表里缺第一个位置，编不过（预期是「实参个数不足」
// 那一类，常见码 E0061；**判据是实跑产出的那份 `.stderr` 本身，不是这个码**）。
// **文件名即这条通道——本样例只钉「没有盲判结论，这一类输入写不出来」。**
//
// 它钉的是 §7.4 第 2 条 / §29 的构造顺序：盲判**先于**比较 Worker 完成声明——
// 构造点的第一个参数正是 `BlindVerdict`（`src/input.rs`），故「先比较、后盲判」在类型层
// 写不出来。**这是构造性论证（类型层），不是运行期检查。**
//
// **它不声称「`PostBlindVerifierInput` 只能经 `new` 构造」**：两个字段是 `pub` 的，
// 结构体字面量同样可行（`src/input.rs` 照实写明）；被挡住的是**缺盲判结论**的那一次调用形态。
use continuum_verify::{PostBlindVerifierInput, WorkerCompletionDeclaration};

fn build() -> PostBlindVerifierInput {
    // §7.4 第 2 条：第一个实参（`BlindVerdict`）不在——这一次调用写不出来。
    PostBlindVerifierInput::new(WorkerCompletionDeclaration::from_str("模型宣告：我完成了"))
}

fn main() {
    // 取一次函数项，免得 `build` 触发 `dead_code` 警告混进本样例的 `.stderr`；
    // 函数体照常被类型检查，故上面那次调用仍是本样例要的那次解析。
    let _ = build;
}
