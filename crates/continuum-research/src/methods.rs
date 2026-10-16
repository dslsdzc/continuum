//! `bind_research_methods`：把 `research/` 四条方法的 `realized_by` 填进方法库（设计 §7）。
//!
//! **这是本块调用 `MethodRegistry::bind` 的唯一入口**——本块不得自造 `MethodEntry`、
//! 不得 `register` 方法条目（P5b 设计第六节：四条 `research/` 方法的 id 由 P5b 的
//! `seeded()` 给出，本块只把「每一条已 seed 的 id」逐条 `bind` 一次）。
//! 四条映射逐条写死在函数体里，顺序与设计 §7 表的行序一致。

use continuum_method::{MethodError, MethodId, MethodRegistry};

/// 把 §7 表的四条 `realized_by` 填进方法库（P5b 设计第六节：每块对其领域目录里的
/// **每一条已 seed 的 id** 调用一次 `bind(id, &[...])`）。`research/` 四条逐条非空
/// （P5b 设计 §4.5：`has_counterpart(Research) == true` ⇒ 空即「漏 bind」）。
///
/// 这是本块调用 `MethodRegistry::bind` 的**唯一入口**。
/// **本函数不做二次登记**：不 `register` 方法条目（四块不得自造 `MethodEntry`）。
/// **本函数不吞错**：任一条 `bind` 返回 `Err` 即原样透出 `MethodError`。
/// 失败直接透出 `MethodError`，本块没有自己的错误类型。
///
/// **`research-question` 与 `research-synthesis` 不在上表里，这不是漏 bind**：
/// §187 的 `research/` 只有四条，而 §330 有七个步骤——`Synthesis` 在方法库里没有对应名。
pub fn bind_research_methods(registry: &mut MethodRegistry) -> Result<(), MethodError> {
    // 设计 §7 表的四条映射，逐条写死（`MethodId::new(..)` ＋ `&[&str]`），
    // 顺序与 §7 表的行序一致；「不吞错」＝每条后以 `?` 原样透出。
    registry.bind(
        &MethodId::new("retrieval"),
        &["research-search", "research-source-set"],
    )?;
    registry.bind(
        &MethodId::new("evidence-analysis"),
        &["research-evidence-extraction"],
    )?;
    registry.bind(
        &MethodId::new("contradiction-check"),
        &["research-contradiction-check"],
    )?;
    registry.bind(
        &MethodId::new("citation-verification"),
        &["research-citation-verification"],
    )?;
    Ok(())
}
