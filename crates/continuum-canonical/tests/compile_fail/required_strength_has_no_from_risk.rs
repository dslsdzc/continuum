// 反例（**应编不过**）：本层**不提供**任何把 `ResolutionRisk` 映射成 `RequiredStrength`
// 的基线（§4.5，设计 `docs/superpowers/specs/2026-10-06-p4-semantic-layer-design.md:767-771`）。
//
// **公开面清单逐条**：那条基线**只要有**，就必然从下面两条路之一露出来，故两条各试一次：
//
// 1. **具名构造函数**：`RequiredStrength::from_risk(&risk)` —— 没有这个方法；
// 2. **`From` 那条路**：`RequiredStrength::from(risk)` —— 没有这个实现。
//
// 理由（不是洁癖）：「风险越高要求越严格」是一条**单调性**要求，而单调性需要一个**序**
// ——`cost` 的高、`reversibility` 的低、`privacy` 的高，四者之间的序规范没有给，
// 故**无中生有的序会把「高成本」判成「低风险」**，基线比没有更坏。
//
// **限度照实记**：本样例钉的是**这两个名字**。方法名是开放集、枚举不完，
// 故「本层没有发明任何别的形状的序」那句更宽的话**本层拿不出编译期照片**
// ——那一侧的半张照片在 `tests/risk.rs` 的源码清单扫描里（它同样只钉这两个模式）。
//
// **正控制**：`tests/compile_pass/the_risk_and_binding_surface_is_usable.rs`
// ——`ResolutionRisk` 可构造、`RequiredStrength::new(Opaque)` 可用。
use continuum_canonical::{Opaque, RequiredStrength, ResolutionRisk};

fn main() {
    let risk = ResolutionRisk {
        effect: Opaque::from_str("effect"),
        cost: Opaque::from_str("cost"),
        reversibility: Opaque::from_str("reversibility"),
        privacy: Opaque::from_str("privacy"),
    };

    // 一：没有具名构造函数。
    let _ = RequiredStrength::from_risk(&risk);

    // 二：也没有 `From<ResolutionRisk>` 那条路。
    let _ = RequiredStrength::from(risk);
}
