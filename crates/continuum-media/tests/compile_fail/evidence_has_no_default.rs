// 应编译失败：`Evidence` **没有 `Default` 实现**，故 `Evidence::default()` 这条路
// 走不通（预期是「没有这个关联函数」那一类，常见码 E0599；**判据是实跑产出的那份
// `.stderr` 本身，不是这个码**）。
//
// 它与 `evidence_fields_are_private`（兄弟样例）是**两条不同的编译失败通道**：
// 那份钉「字段私有」（堵住绕过 `from_tool_result` 的字面量写入口），本份钉「没有无参
// 构造」（连字面量之外的另一条捷径也不存在）。**不许并成一份**——实跑证实两条通道在
// 同一份样例里互相遮蔽（`default` 那处的错误一出现，字面量那处的 E0451 就不报；
// 见兄弟样例的文件头与 task 报告）。两条的失效条件也不同：别处若给 `Evidence` 加上
// `#[derive(Default)]`，本份失效；字段若被改成 `pub`，兄弟样例失效。
//
// 它钉的仍是 P5a 设计「唯一产生点」是**结构事实**在本块的**使用侧**照片
// （P5a 侧的同名依据见 `crates/continuum-verify/tests/compile_fail/evidence_has_no_default.rs`，
// 按内容指）。
use continuum_verify::Evidence;

fn build_by_default() -> Evidence {
    // 没有这条无参构造捷径。
    Evidence::default()
}

fn main() {
    // 取一次函数项，免得 `dead_code` 警告混进本样例的 `.stderr`；
    // 函数体照常被类型检查，故上面那次调用仍是本份要的解析。
    let _ = build_by_default;
}
