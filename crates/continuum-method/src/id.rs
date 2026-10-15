//! `MethodId`：方法的标识，**开放**（设计 §4.2）。

use std::fmt;

/// 方法的标识。**开放** newtype 而非封闭枚举：§187 的清单以「例如：」开头
/// （`docs/spec/04-method.md:114`），目录要能容纳名之外的条目（设计 §4.2）。
/// 内部串私有，构造入口只有 [`MethodId::new`]。
///
/// # 派生集（`Debug, Clone, PartialEq, Eq, Hash`）的来历
///
/// 设计 §4.2 的签名列里没有列派生，这五枚是由**两处必需**倒推的：
/// `MethodRegistry` 的 `HashMap<MethodId, MethodEntry>` 要 `Eq + Hash`（两枚），
/// `MethodError` 的 `Debug, Clone, PartialEq, Eq` 派生要 `MethodId` 有同四项
/// （四枚）——两处取并集恰是这五枚（**本计划自定**，记在实现计划 `## 遗留`
/// 第三节）。**不加 `Ord` / `PartialOrd`**（`select` 的排序键取 `as_str`）、
/// **不加 serde 派生**（本 crate 不序列化任何东西）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MethodId(String);

impl MethodId {
    /// 由任意串构造。
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// 返回构造时那枚串本身。
    ///
    /// `tests/identity.rs` 的 `method_id_returns_the_string_it_was_built_from`
    /// 用两枚夹具各钉一侧（大写字母 / 非 ASCII 与斜杠）。
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// `MethodError` 的格式串以 `{id}` 引用该字段，thiserror 要求该字段实现 Display。
// 理由与写法同 `crates/continuum-operator/src/definition.rs:36-42`
// （注释本体 `:36-37`，`Display` 本体 `:38-42`）。
impl fmt::Display for MethodId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
