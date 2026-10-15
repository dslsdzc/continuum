// 应编译失败：本层**不建检测器**——`Specification` 的公开面上没有
// `detect_conflicts` / `find_undefined_behaviour` 这类函数（§214 的
// 「检测冲突 / 未定义行为」，设计 §6.1）。
//
// **为什么这不是漏项**：§214 的「什么算冲突」规范没给判据（设计 §16 第 10 条），
// 故本层只把 `Conflict` / `UndefinedBehavior` 作为 `Unknowns::Enumerated` 的成员
// **保存**，不发明检测算法。
//
// **本样例钉的是那两个名字**，不是「没有任何别的检测函数」那句更宽的话——函数名是
// **开放集**，`trybuild` 枚举不完（与 `continuum-canonical` 的
// `candidate_source_has_no_pick_one_method` 同一限度）。**限度之外的那一半**由
// Task 21 的 `src/` grep 复核。
//
// **本文件的前置两行编得过**（`SpecDraft` → `finalize` → `Specification` 的公开
// 构造路径）——若那条正路走不通，`.stderr` 里会出现别处的错误，逐字比对即红；
// 故这两条失败不来自「API 整体不可用」，只落在最后两个不存在的方法上。
// 「只保存」那一半的运行期照片在 `tests/spec.rs` 的
// `conflict_and_undefined_behaviour_are_types_only`（两枚成员写入、读回、内容原样）。
use continuum_semantics::{SpecDraft, Unknowns};

fn main() {
    let spec = SpecDraft {
        goals: Vec::new(),
        invariants: Vec::new(),
        acceptance_predicates: Vec::new(),
        forbidden_states: Vec::new(),
        preferences: Vec::new(),
    }
    .finalize(Unknowns::NoneFound);

    // 应编译失败：`Specification` 上没有冲突检测器。
    let _ = spec.detect_conflicts();

    // 应编译失败：也没有未定义行为检测器。
    let _ = spec.find_undefined_behaviour();
}
