// 应编译失败：`Evidence` 的八个字段**全部私有**，故**全字段结构体字面量**这条路在
// crate 外走不通（预期是「字段私有」那一类，常见码 `E0451`；**判据是实跑产出的那份
// `.stderr` 本身**，不是这个码——本项目为「凭记忆写错误码」付过代价）。
// **文件名即这条通道——本样例只钉结构体字面量这一条。**
//
// 它钉的是 §4.4 的「唯一产生点」是**结构事实**（使用侧读法，设计 §6.2）：`Evidence`
// 的构造函数只有 `Evidence::from_tool_result`（§89），字面量构造被私有字段挡住——
// 若字段可写，调用方可以绕过那个产生点自己拼一条证据，「唯一」就不成立。
//
// **它不声称「拿不到一枚 `Evidence`」**：`from_tool_result` 是 `pub fn`
// （`continuum-verify/src/evidence.rs`），crate 外经它就能拿到证据；被挡住的只是
// 「绕过它自己拼一个」——「拿得到」的那一半由正控制
// （`tests/ui_pass/evidence_is_reachable.rs`）单独拍。
//
// **八个字段的值一律用占位函数**：这条通道上字段的**值**是什么无关紧要——字面量在
// 走到求值之前就被字段私有检查拒掉，实跑里本样例只产生「字段私有」这一条错。而值若
// 想写真，`EvidenceStrength` / `EvidenceScope` 两个载荷的里子是 `serde_json::Value`，
// 本 crate 的 dev 依赖只有 `continuum-verify` 与 `trybuild` 两条（不引 `serde_json`），
// 故以 `placeholder()` 占位：返回类型即字段类型，不会引入第二条错，也不是发散表达式
// （没有 `unreachable_code` 警告混进本样例的 `.stderr`）。
//
// **注释与代码一样是照片的一部分**：`trybuild` 把本文件逐行打印进
// `evidence_literal_is_private.stderr` 的输出里，故**改一句注释就会改那张照片**——
// 改任一处都要重跑取新值（见 `tests/type_level.rs` 文件头）。
use continuum_verify::Evidence;

/// 占位值：类型由各字段推断；函数体不会被执行到（字面量过不了字段私有检查）。
fn placeholder<T>() -> T {
    todo!()
}

fn main() {
    // §4.4：字段全部私有——这条路走不通。
    let _ = Evidence {
        id: placeholder(),
        evidence_type: placeholder(),
        subject: placeholder(),
        claim: placeholder(),
        producer: placeholder(),
        artifact_refs: placeholder(),
        strength: placeholder(),
        scope: placeholder(),
    };
}
