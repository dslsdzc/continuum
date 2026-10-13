// 应编译失败：`Specification` 的六个字段**私有**、且没有公开构造函数
// （§227 的六元组；唯一构造入口是 `SpecDraft::finalize`，`src/spec.rs`）
// ——故 crate 外**拼全六个字段**的结构体字面量仍然造不出来。
//
// 它钉的是 §227 两句 MUST 的**类型落点**里「Specification 只能由 `finalize` 造出」
// 那一半：`finalize` 要求调用方**显式给出** `Unknowns` 之一（不是 `Option`），
// 故「没查」不可表达（两句 MUST：「`unknowns` MUST 被正式保存」与
// 「系统 MUST NOT 把未知项自动转换成假设」——设计 §6.1）。
//
// **本样例不声称「crate 外拿不到 `Specification`」**：经 `load_specification`
// 从库里读得到。被挡住的只是「绕过 `finalize` 自己拼一个」。
//
// 正面那一半（`finalize` → 写库 → 读回）在 `tests/spec.rs` 的
// `an_intent_version_and_its_specification_are_written_in_one_transaction` 里；
// 「六字段清单逐项」的穷尽解构在 `src/spec.rs` 的 `#[cfg(test)]`（字段私有后
// 只有 crate 内做得到）。
use continuum_semantics::{Specification, Unknowns};

fn main() {
    // 六个字段全列齐，仍然编不过：它们是私有的。
    let _ = Specification {
        goals: Vec::new(),
        invariants: Vec::new(),
        acceptance_predicates: Vec::new(),
        forbidden_states: Vec::new(),
        preferences: Vec::new(),
        unknowns: Unknowns::NoneFound,
    };
}
