// 反例（**应编不过**）：门槛是 `resolve` 的**必填入参**——少了它，调用**编不过**
// （§4.5：`required_resolution_strength` 是必填，「不提供默认值」）。
//
// 具体的形状有两条，本文件各钉一条（「签名层面可断言」那两条，设计 §4.5 的
// `design:767-773`）：
//
// 1. **少传该参数**：下面第一次调用只给到 `confidence` 为止；
// 2. **不收 `Option<RequiredStrength>`**：第二次调用把 `None` 传在该参数的位置上。
//
// 两条都在同一个文件里，因为它们钉的是**同一个签名**的两个面
// （`&RequiredStrength`，既不能省、也不能是 `Option<…>`）；这不是「一份样例守两句要求」
// ——那两句话在规范里就是一句（「必填入参」＝「不能省」∧「没有默认值」）。
//
// **正控制**：`tests/compile_pass/the_risk_and_binding_surface_is_usable.rs` 里那个
// 五个参数齐全的调用**编得过**。
use continuum_canonical::{
    Candidate, CandidateFacts, Opaque, RequiredStrength, ResolutionConfidence, resolve,
};

fn main() {
    let cs: Vec<(Candidate, CandidateFacts)> = Vec::new();
    let confidence = ResolutionConfidence::new(Opaque::from_str("calibrated"));

    // 一：少传 `required`（本函数收五个参数，这里只给四个）。
    let _ = resolve(&cs, None, &[], &confidence);

    // 二：`required` 不接受 `Option`——传 `None` 编不过。
    let _ = resolve(&cs, None, &[], &confidence, None::<RequiredStrength>);
}
