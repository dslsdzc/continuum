// 应编译失败：`InitialVerifierInput` 的字段清单里**没有** `worker_declaration` 这个位置，
// 故「把 Worker 的完成声明塞进初判输入」这条通道写不出来（预期是「结构体没有这个字段」
// 那一类，常见码 E0560；**判据是实跑产出的那份 `.stderr` 本身，不是这个码**——
// 本项目为「凭记忆写错误码」付过代价）。
// **文件名即这条通道——本样例只钉「那个位置不存在」。**
//
// 它钉的是 §7.4 第 1 条 / §263 的结构性兑现：初判输入面**只有** Contract / Artifact /
// Evidence 三类（`src/input.rs` 的 `assemble` 逐字照录），Worker 的完成声明不在其列；
// §342 第 10 条「完成不由 Worker 自行宣布」由此没有输入位置——
// **这是构造性论证（类型层），不是「我没找到办法塞进去」**（设计 §7.4 第 1 条的原话）。
//
// **它不声称「`InitialVerifierInput` 什么都装不进」**：三个字段是 `pub` 的，字面量构造
// 这条路径存在（`src/input.rs` 的张力记录照实写明了）；被挡住的是**第四类**——多出来的
// `worker_declaration`。字段清单里没有它的位置，这正是本样例的射程。
use continuum_verify::{ContractView, InitialVerifierInput, WorkerCompletionDeclaration};

fn assemble() -> InitialVerifierInput {
    InitialVerifierInput {
        contract: ContractView {
            requirements: Vec::new(),
        },
        artifacts: Vec::new(),
        evidence: Vec::new(),
        // §7.4 第 1 条：这个位置不存在——多出来的第四项。
        worker_declaration: WorkerCompletionDeclaration::from_str("模型宣告：我完成了"),
    }
}

fn main() {
    // 取一次函数项，免得 `assemble` 触发 `dead_code` 警告混进本样例的 `.stderr`；
    // 函数体照常被类型检查，故上面那次字面量仍是本样例要的那次解析。
    let _ = assemble;
}
