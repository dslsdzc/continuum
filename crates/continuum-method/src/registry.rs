//! `MethodRegistry`：登记、解析、绑定与选取（设计 §4.5）。

use crate::{MethodDomain, MethodEntry, MethodError, MethodId};
use std::collections::HashMap;

/// 方法库的登记表。形状照 `OperatorRegistry`
/// （`crates/continuum-operator/src/registry.rs:14-21`）：私有 `entries` 字段
/// （`HashMap<MethodId, MethodEntry>`，键就是方法的 id、无版本维度）、
/// `new()` 取 `Self::default()`。
///
/// `#[derive(Debug, Default)]` 是**本计划自定**的形状申报（设计 §4.5 的签名列里
/// 只有 `new`）：`Debug` 让用例失败时可打印，`Default` 使 `new()` 取
/// `Self::default()`。代价是 `MethodRegistry::default()` 随之成为公开入口、
/// 与 `new()` 并存；收益是与既有 `OperatorRegistry` 的形状一致。来历见实现计划的
/// `## 遗留` 第三节。
#[derive(Debug, Default)]
pub struct MethodRegistry {
    entries: HashMap<MethodId, MethodEntry>,
}

impl MethodRegistry {
    /// 空登记表。
    pub fn new() -> Self {
        Self::default()
    }

    /// 登记一条方法。同一 [`MethodId`] 已存在时返回 [`MethodError::Duplicate`]，
    /// **不覆盖**既有条目。
    ///
    /// # 具名消费点（照实）
    ///
    /// `register` 在本 crate 的**唯一生产调用者**是 `seeded()`（Task 4；
    /// 四个算子块被禁止调它——设计 §4.5）。§187 的十八个 id 互不相同，故
    /// [`MethodError::Duplicate`] 在**生产路径上不可达**——它的照片只能是
    /// `tests/registry.rs` 里手建的两条同 id 条目。这不是缺口，是判据 4 的射程。
    pub fn register(&mut self, entry: MethodEntry) -> Result<(), MethodError> {
        let id = entry.id.clone();
        if self.entries.contains_key(&id) {
            return Err(MethodError::Duplicate { id });
        }
        self.entries.insert(id, entry);
        Ok(())
    }

    /// 按 id 解析一条方法。未注册时返回 [`MethodError::NotFound`]。
    ///
    /// 判定形状照 `OperatorRegistry::resolve`（`crates/continuum-operator/src/registry.rs:36`）。
    pub fn resolve(&self, id: &MethodId) -> Result<&MethodEntry, MethodError> {
        self.entries
            .get(id)
            .ok_or_else(|| MethodError::NotFound { id: id.clone() })
    }

    /// 把某条方法的 `realized_by` 设为给定算子 id（文本）——四个算子块填内容的
    /// **唯一入口**（设计 §4.5）。
    ///
    /// 方法不存在时返回 [`MethodError::NotFound`]，**且不新建条目**（没有半写；
    /// `tests/registry.rs` 的 `bind_returns_not_found_for_an_unregistered_id_and_creates_nothing`
    /// 钉住这一侧、`binding_twice_overwrites_the_previous_value` 钉住下面的覆盖语义）。
    /// 命中时**只改 `realized_by`**，逐串 `to_string()` **覆盖**旧值
    /// （「重复 bind 覆盖旧值」——同一块重复调它即覆盖；一对一配对下覆盖不丢
    /// 别人的贡献，设计 §5.1）。
    pub fn bind(&mut self, id: &MethodId, realized_by: &[&str]) -> Result<(), MethodError> {
        let entry = self
            .entries
            .get_mut(id)
            .ok_or_else(|| MethodError::NotFound { id: id.clone() })?;
        entry.realized_by = realized_by.iter().map(|s| s.to_string()).collect();
        Ok(())
    }

    /// **按领域选取的入口**（切分文档 §五 P5b 行）：返回该领域的全部条目，
    /// **按 id 升序**排列（设计 §4.5）。
    ///
    /// 排序键取 `id.as_str()`——**不给 [`MethodId`] 派生 `Ord`**（设计 §4.2 的
    /// 签名列里没有它，加派生是超出设计的一步）。排序键的取值是**本计划自定**，
    /// 申报见实现计划的 `## 遗留` 第三节。
    pub fn select(&self, domain: MethodDomain) -> Vec<&MethodEntry> {
        let mut selected: Vec<&MethodEntry> = self
            .entries
            .values()
            .filter(|entry| entry.domain == domain)
            .collect();
        selected.sort_by_key(|entry| entry.id.as_str());
        selected
    }
}
