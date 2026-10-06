//! 工具侧公开面上没有枚举适配器的入口（设计 §7.1、§3.1）。
//!
//! 与 `tool_for_does_not_exist.rs` 同一条性质的第二个拼法：注册表不交出它持有的
//! `Arc<dyn ToolProvider>` 集合，**没有** `tool_providers()`。「已登记了哪些适配器」
//! 不是公开面的一部分——要枚举器材只能用 `list_tools()`，它交出的是一组 `ToolDescriptor`。
//!
//! **判据是「公开面清单 + 本样例」两条一起**，全称否定不在本样例的证明力之内
//! （设计 §11 末段）。

fn main() {
    let registry = continuum_provider::ProviderRegistry::new();
    let _ = registry.tool_providers();
}
