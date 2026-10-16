// 应编得过（正控制）：`Evidence` 这个**类型名**在 crate 外**可命名**。
//
// 没有它，`evidence_fields_are_private` 那份反例的绿可以来自「本 crate 里根本写不出
// 这个名字」（例如 dev 依赖没配好）——那与「字段私有」无关，是恒绿的假照片
// （恒绿的守卫比没有更坏，同 `continuum-budget` 的三份正控制）。
//
// **语料不从被测清单里取**：本份的语料是**名字本身**（函数签名里的类型路径），
// 不是被测样例里的任何一个片段。**只用名字、不构造值**：签名取
// `Option<continuum_verify::Evidence>`——包一层 `Option` 是为了不要求造出真的
// `Evidence`（构造要走 `from_tool_result` 与配套的七个类型，那是另一件事）；
// 本份里没有一处 `Evidence` 的值构造，故它也不提供第二条构造路径。
fn evidence_is_nameable(
    e: Option<continuum_verify::Evidence>,
) -> Option<continuum_verify::Evidence> {
    e
}

fn main() {
    // trybuild 的 pass 样例会被编译并运行 `main`；这里只取函数项，不执行。
    let _: fn(Option<continuum_verify::Evidence>) -> Option<continuum_verify::Evidence> =
        evidence_is_nameable;
}
