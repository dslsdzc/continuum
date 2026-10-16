// 应编译失败：`CompletionVerdict` 的变体清单里**没有**「完成」这一臂，故「把完成判定
// 写成『完成』」这条构造写不出来（预期是「没有这个变体或关联项」那一类，常见码 E0599；
// **判据是实跑产出的那份 `.stderr` 本身，不是这个码**）。
// **文件名即这条通道——本样例只钉「『完成』这一臂不存在」。**
//
// 它钉的是 §8.2：「完成」为什么不是第三臂——`AllRequiredRequirementsVerified` 的输入是
// `CoverageVerdict`，而那个枚举**没有正臂**（§6.2），故合取**不可能为真**。
// 那句论证是构造性的（类型定义 + 合取结构），**本样例是它的实测补件**（设计 §12.3 第 3 条）。
//
// **它不声称「完成判定永远判不出完成」**：那句话的另一半（合取怎么算）在
// `src/completion.rs` 的 `judge_completion` 里，由 `tests/completion.rs` 覆盖；
// 本样例只钉类型层的那一半——**本类型里不存在一个可达的『完成』值**。
use continuum_verify::CompletionVerdict;

fn verdict() -> CompletionVerdict {
    // §8.2：这个臂不存在——「完成」在类型层没有位置。
    CompletionVerdict::Complete { failing: vec![] }
}

fn main() {
    // 取一次函数项，免得 `verdict` 触发 `dead_code` 警告混进本样例的 `.stderr`；
    // 函数体照常被类型检查，故上面那次构造仍是本样例要的那次解析。
    let _ = verdict;
}
