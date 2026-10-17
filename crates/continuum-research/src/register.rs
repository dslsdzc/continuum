//! `register_research_operators`：先核后写地把一批算子注册进 `OperatorRegistry`（设计 §4.1）。
//!
//! **「先核后写」不在这里发明**：它是注册这个动作的语义——`OperatorError::Duplicate`
//! 的判定在 P1 的 `OperatorRegistry::register`，本函数只是把同一条判定在写之前跑完。
//! 核的实现只用 `OperatorRegistry` 的 `pub` 面（`resolve`），不读它的私有 `entries`。

use continuum_operator::{Operator, OperatorError, OperatorRegistry};

/// 把给定的一批算子注册进给定注册表（`&all_operators()` 是常规实参）。
/// **先核后写**：先逐枚核「(id, version) 是否已在表里」与「批内是否自撞」，全部通过后再逐枚注册；
/// 任一枚不成即返回 `Err`，此时注册表的内容与调用前逐枚相同（不留半注册）。
///
/// 一枚研究算子与既有算子同 (id, version) 是注册期的错误，**不静默跳过**。
/// **本函数不新造错误类型**：要报的失败只有「(id, version) 撞车」一件事，
/// 而它的判定已有一枚臂（`OperatorError::Duplicate`，判定在 `OperatorRegistry::register`）。
///
/// **「先核后写」为什么值得写**：不写它，一次带重复项的注册会留下半注册的表，
/// 而调用方拿到的 `Err` 说的是「这次调用没成功」——调用方据 `Err` 重试一次，
/// 第二批会从重复项处再失败，表里那一半却已经在了。
pub fn register_research_operators(
    registry: &mut OperatorRegistry,
    operators: &[Operator],
) -> Result<(), OperatorError> {
    // ① 先逐枚核「(id, version) 是否已在表里」——经 `OperatorRegistry::resolve`，
    //    不读它的私有 `entries`。
    for op in operators {
        if registry.resolve(&op.id, &op.version).is_ok() {
            return Err(OperatorError::Duplicate {
                id: op.id.clone(),
                version: op.version,
            });
        }
    }

    // ② 再核「批内是否自撞」：同一批里两枚的 (id, version) 相同也是撞车。
    //    判定只看 (id, version)，其余字段的不同不影响。
    for (i, earlier) in operators.iter().enumerate() {
        for later in &operators[i + 1..] {
            if earlier.id == later.id && earlier.version == later.version {
                return Err(OperatorError::Duplicate {
                    id: later.id.clone(),
                    version: later.version,
                });
            }
        }
    }

    // ③ 全部通过后再逐枚注册。走到这里时 `register` 不会再为表中的重复报错
    //    （① 已核过表、② 已核过批内），但错误仍原样透出——要报的失败仍只有
    //    `OperatorRegistry::register` 那一枚臂。
    for op in operators {
        registry.register(op.clone())?;
    }
    Ok(())
}
