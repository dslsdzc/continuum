// 反例（**应编不过**）：`RequiredStrength` **没有 `Default`**（§4.5／§276）。
//
// 由来：§276 的 `required_resolution_strength = f(effect, cost, reversibility, privacy)`
// 里 `f` 的形态、以及四个风险维之间的**序与量纲都没有给**（设计 §4.5，
// `docs/superpowers/specs/2026-10-06-p4-semantic-layer-design.md:763-773`）。
// 于是**一个「保守的」基线也无从写起**——无中生有的序会把「高成本」判成「低风险」。
// `Default` 恰好就是那样一条基线（它凭空给出一个门槛），故本类型不派生它。
//
// **正控制**：`tests/compile_pass/the_risk_and_binding_surface_is_usable.rs`
// ——`RequiredStrength::new(Opaque)` 那条路**编得过**。没有它，本份样例的绿可以来自
// 「`RequiredStrength` 这个名字整个不存在」这类与判据无关的原因。
//
// 本文件**不放别的断言**：一份样例守两句要求的话，额外的失败面会让「谁红」的预告对不上
// （Task 3 的 compile_pass 样例已记过同一教训）。
use continuum_canonical::RequiredStrength;

fn main() {
    let _ = RequiredStrength::default();
}
