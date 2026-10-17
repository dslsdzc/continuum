//! 注册入口（设计 §3.1）：[`register_image_operators`]——**先查重后写**。

use continuum_operator::{Operator, OperatorRegistry};

use crate::error::ImageError;

/// 把给定的一批算子注册进给定注册表（`&all_operators()` 是常规实参）。
/// **先查重后写**：先逐枚核「注册表里是否已有同 (id, version)」（判定在 P1 的
/// `OperatorRegistry::register`，`crates/continuum-operator/src/registry.rs:24-34`），
/// 全部通过后再逐枚注册；任一枚被拒即返回 `Err`，
/// 此时注册表的内容与调用前逐枚相同（不留半注册）。
/// 一枚图像算子与既有算子同 (id, version) 是注册期的错误，**不静默跳过**。
/// **参数带一批算子**（不是只吃 `all_operators()`）：§9 的批次边界要能按 §343 换，
/// 且 §11.2 第 (c2) 条（不留半注册）才写得出来。
///
/// **「先查重后写」这一句的来历（设计 §3.1 订正过）**：初稿写的是「先**核**后写：
/// 先逐枚核 §3.5 的前提」——而 **§3.5 自述本块没有那条注册期前提**（两枚
/// `Deterministic` 算子的候选集合是单元集，「名单内／名单外」没有可触发的形态），
/// 故「先核」的内容落不下来。**以查重为准的理由**：§11.2 第 (c2) 条要拍的正是
/// 「不留半注册」，而它是查重（`registry.rs:26` 的 `contains_key`）与写入
/// （`:32` 的 `insert`）之间的**次序**——核的东西只能是注册表现状，
/// 不可能是 §3.5 那条本块没有的规则。
///
/// **这条函数拦不住哪一类**（别把它当成一条全覆盖的守卫）：它只查注册表里的重复，
/// **不查**「这个算子的候选 backend 是否逐位可复现」——那条规则与领域无关，
/// 属 P5e 一处（设计 §3.5），本块不建第二份；也**不查**「这个算子的端口取值域对不对」
/// ——那是 Task 2 的 (a) 与 Task 3 的边表。
pub fn register_image_operators(
    registry: &mut OperatorRegistry,
    operators: &[Operator],
) -> Result<(), ImageError> {
    // 先查重（不写）：逐枚核注册表现状——`entries` 私有、无迭代器、无 `len()`
    //（`crates/continuum-operator/src/registry.rs:15-17`），故只能经 `resolve` 观测。
    // 记下**第一枚**与既有算子同 (id, version) 的：它的注册必被拒。
    let rejected = operators
        .iter()
        .find(|operator| registry.resolve(&operator.id, &operator.version).is_ok());

    // 后写：被记下的那一枚排到最前先过。对既存项，P1 的 register 在 `contains_key`
    //（`registry.rs:26`）命中即返回 `Duplicate`、**不到 `insert`（`:32`）**——它被拒时
    // 本批一枚都还没写入（重复项在批的任意位置都先被拒，不留半注册）；
    // 没有既存重复时，序列就是原批、按原序逐枚注册。
    // 这是本函数**唯一**一处把 `OperatorError` 转进 `ImageError::Registry` 的地方——
    // 「什么算重复」的判定与 `Duplicate` 的构造都留在 P1，本块只带出来、不重述。
    for operator in rejected.into_iter().chain(operators) {
        registry.register(operator.clone())?;
    }
    Ok(())
}
