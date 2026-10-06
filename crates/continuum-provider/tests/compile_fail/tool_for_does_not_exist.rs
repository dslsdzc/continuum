//! 工具侧公开面上没有「按 id 交出裸适配器」的入口（设计 §7.1、§3.1）。
//!
//! 注册表持有的 `Arc<dyn ToolProvider>` 只用于路由与调用：公开面开的是 `list_tools` /
//! `describe_tool` / `invoke_tool` 三条，**没有** `tool_for`。本样钉的是 `tool_for`
//! 这一个拼法——它编译不过。
//!
//! **判据是「公开面清单 + 本样例」两条一起**：样例钉不住「任何名字的入口都不存在」
//! 这一全称否定，日后有人加一个叫 `provider_for_tool` 的公开方法，本样例照样过
//! （设计 §11 末段）。

fn main() {
    let registry = continuum_provider::ProviderRegistry::new();
    let _ = registry.tool_for(&continuum_core::tool::ToolId::new("t"));
}
