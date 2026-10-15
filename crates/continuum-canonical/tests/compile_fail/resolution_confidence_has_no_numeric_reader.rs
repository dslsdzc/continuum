// 应编译失败：`ResolutionConfidence` **没有**能把值变成一个数的读者（§4.4 第 2 条，
// `docs/superpowers/specs/2026-10-06-p4-semantic-layer-design.md:741-745`）。
// §275（`docs/spec/05-normative.md:1428-1448`）只让这个值决定「是否允许自动执行」，
// 故方法集里只有 `meets(&RequiredStrength) -> bool`。
//
// **本样例钉的是两个名字**（`to_f64` / `as_probability`），不是「没有任何别的名字」那句
// 更宽的话——方法名是**开放集**，`trybuild` 枚举不完（与
// `candidate_source_has_no_pick_one_method.rs` 同一限度）。
//
// 正控制：`tests/compile_pass/resolution_confidence_meets_is_usable.rs`
// （`c.meets(&req)` 编得过）。没有它，本样例的绿可以来自「`ResolutionConfidence`
// 这个名字整个不存在」这类与判据无关的原因。
use continuum_canonical::{Opaque, ResolutionConfidence};

fn main() {
    let c = ResolutionConfidence::new(Opaque::from_str("calibrated"));

    // 应编译失败：没有 `to_f64` 这个读者。
    let _ = c.to_f64();

    // 应编译失败：也没有 `as_probability`。
    let _ = c.as_probability();
}
