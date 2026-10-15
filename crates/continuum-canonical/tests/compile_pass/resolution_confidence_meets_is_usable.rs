// 正控制（应编译**通过**，且运行时通过）：门槛判定 `c.meets(&required)` 是公开可用的，
// 且两枚值各自有**公开的构造入口**（`ResolutionConfidence::new(Opaque)` /
// `RequiredStrength::new(Opaque)`）。
//
// 没有这一份，`tests/compile_fail/resolution_confidence_has_no_numeric_reader.rs` 与
// `tests/compile_fail/resolution_confidence_cannot_be_built_from_a_score.rs` 的绿
// 可以来自任何与判据无关的原因（例如 `ResolutionConfidence` 这个名字不存在、
// 或构造入口改名了）。这就是本仓「正控制不得自证」那条纪律要的形状：
// 反例断「那两条路走不通」，正例断「该走得通的那条走得通」。
//
// 它同时钉住三件事，各一句：
// 1. `meets` 的入参就是 `&RequiredStrength`（不是 `Option<…>`——§4.5 的「门槛必填」）；
// 2. `meets` 返回 `bool`（不是某个带分数的结构）；
// 3. 两枚值都从**不透明值**来，本层不解释它（§16 第 6 条同形：取值域规范未给）。
use continuum_canonical::{Opaque, RequiredStrength, ResolutionConfidence};

fn main() {
    let confidence = ResolutionConfidence::new(Opaque::from_str("calibrated"));
    let required = RequiredStrength::new(Opaque::from_str("calibrated"));

    // 类型注解即断言：返回的是 `bool`。
    let met: bool = confidence.meets(&required);
    assert!(met, "同档 ⟹ 达到要求");

    // 另一侧：异档 ⟹ 未达到（判据不是恒真）。
    let unmet: bool = confidence.meets(&RequiredStrength::new(Opaque::from_str("other")));
    assert!(!unmet, "异档 ⟹ 未达到要求");

    // **本样例只到「可构造、可判、返回 bool」为止**：判据为何取等值、
    // 以及它在四态里的后果，由 `tests/resolver.rs` 断言，不在这里再断一遍
    // ——一份样例守两句要求的话，那句额外的断言会成为**别的**变异体的第二个失败面，
    // 让「谁红」的预告与实际对不上（Task 3 的 compile_pass 样例已记过同一教训）。
}
