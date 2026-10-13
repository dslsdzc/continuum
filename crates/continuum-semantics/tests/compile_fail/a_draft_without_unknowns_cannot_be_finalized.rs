// 应编译失败：`finalize` 的入参是**必填**的 `Unknowns`——少给它不行（调用一），
// 把它传成 `None` 也不行（调用二）。`SpecDraft` 里**没有** `unknowns` 字段。
//
// 钉的是 §227 两句 MUST 在**签名**上的落点（设计 §6.1）：`finalize` 要求调用方
// **显式给出** `Unknowns` 之一（不是 `Option<Unknowns>`）——「没查」因此**不可表达**。
//
// 两个面在同一个文件里，因为它们钉的是**同一个签名**（`Unknowns`，既不能省、
// 也不能是 `Option`）；这不是「一份样例守两句要求」——那两句话在 §227 里就是一句
// （「MUST 被正式保存」＝ 必须给出）。
//
// **正面那一半**：`tests/spec.rs` 各用例里的 `finalize(Unknowns::…)` 调用都编得过
// （那是本 crate 的运行期用例，若这条正路走不通，整份文件红）。
use continuum_semantics::{SpecDraft, Unknowns};

fn main() {
    // 一：少给 `unknowns`。
    let draft = SpecDraft {
        goals: Vec::new(),
        invariants: Vec::new(),
        acceptance_predicates: Vec::new(),
        forbidden_states: Vec::new(),
        preferences: Vec::new(),
    };
    let _ = draft.finalize();

    // 二：`unknowns` 不接受 `Option`。
    let draft2 = SpecDraft {
        goals: Vec::new(),
        invariants: Vec::new(),
        acceptance_predicates: Vec::new(),
        forbidden_states: Vec::new(),
        preferences: Vec::new(),
    };
    let _ = draft2.finalize(None::<Unknowns>);
}
