//! 请求侧只收 `&AuthorizedTool`，一枚裸 `ToolId` 递不进去（设计 §7.5、§7.1）。
//!
//! `ProviderRegistry::invoke_tool` 的签名是 `(&AuthorizedTool, Value)`：工具 id 的来源
//! 只有授权证明的 `tool_id()`，函数**不另收 `ToolId`**。旧形状 `ToolInvocation`
//! （自带一枚 id）已裁删（裁决 §五第 22 条）——**本样例是那次删除在类型面上的照片**：
//! 按旧形状调用它编译不过。
//!
//! 与「`AuthorizedToolInvocation` 是唯一请求类型」互为表里：那一条由样例 4、5 钉，
//! 这一条钉的是**调用入口不再接受第二枚 id**。

fn main() {
    let registry = continuum_provider::ProviderRegistry::new();
    let _ = registry.invoke_tool(
        &continuum_core::tool::ToolId::new("t"),
        serde_json::json!({}),
    );
}
