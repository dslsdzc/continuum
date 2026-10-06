//! §316 Tool Adapter 的 ToolProvider。

use async_trait::async_trait;
use continuum_capability::AuthorizedTool;
use continuum_core::model::CallId;
use continuum_core::tool::{ToolDescriptor, ToolId, ToolResult};
use continuum_core::ProviderError;
use serde_json::Value;

/// §316 `invoke` 的请求侧：**只装得下「已授权」**（设计 §7.5，裁决 §一第 2 条）。
///
/// 依据：`docs/superpowers/specs/2026-10-05-p3-bcdf-set-decisions.md` §一第 2 条——
/// 「改的是请求侧，不是响应侧」（响应在调用完成之后才存在，往那里加位置解决不了问题）。
///
/// **可见性三件套（裁决 C2，2026-10-05；设计 §7.5 的「可见性」段）**：
/// **类型 `pub`**——它出现在**公开 trait 的方法签名**里，必须 `pub`；
/// **构造入口 `pub(crate)`**——**归 Task 4，本 task 时点上尚不存在**：届时那唯一的构造点是
/// `ProviderRegistry::invoke_tool`（见 [`crate::ProviderRegistry`] 的文档），且必须写成
/// `pub(crate)`，只有 `continuum-provider` 内的注册表构造它。**为什么不是 `pub`**：若它是
/// `pub`，任何持有一枚真 [`AuthorizedTool`] 的代码都能自行拼出请求、直接调裸适配器的
/// [`ToolProvider::invoke`]——那就是「同一件事两个产生点」，而两条路**都编译得过**。
/// （此处只作文字指引、不作 `[`…::invoke_tool`]` 形式的链接。）
/// **字段私有**——`authorization` / `input` 只经访问器读。
///
/// 两条性质：
/// 1. **没有 [`AuthorizedTool`] 就构造不出它**，而 `pub(crate)` 让这条**更强**：**crate 外的代码
///    即使手里有一枚真的 `AuthorizedTool`，也构造不出这个请求**——它只能把证明交给注册表，
///    由注册表替它构造。「谁构造」是注册表一个产生点，与「谁持有证明」（F）是两件事。
///    （照片：Task 6 的样例 4，预期 `E0603`。）
/// 2. **工具 id 只有一个来源**：授权证明的 [`AuthorizedTool::tool_id`]。本类型**不另收 `ToolId`**，
///    故「出示的 id 与被授权的 id 不是一个」这一种可能**不存在**。（照片：Task 6 的样例 3。）
///
/// **为什么装的是 `&AuthorizedTool`，不是 `Credential`**：这里承载的是**强制点 (1) 的证明**，
/// 不是密钥材料。凭据是 `continuum-secrets` 的词汇、走连接器路径（设计 §7.5 末段；若将来确需把
/// 凭据交给模型/工具适配器，那是一个**第二个位置**，见 `## 遗留`）。
pub struct AuthorizedToolInvocation<'a> {
    authorization: &'a AuthorizedTool,
    input: Value,
}

impl<'a> AuthorizedToolInvocation<'a> {
    /// 这次调用获准了什么。**消费方是工具适配器**（[`ToolProvider`] 的实现）：它在 `invoke` 的
    /// 实现体内逐枚取 `Capability::scope()`，把这次动作限定在该作用域内。
    /// **F 只持有并下传整枚值，不读 `granted()`**（设计 §7.5；`AuthorizedTool::tool_id()` 由注册表取，用于路由）。
    pub fn authorization(&self) -> &AuthorizedTool {
        self.authorization
    }

    /// 这次调用的输入。**原样带过来、原样读出去**：本类型不对它作任何加工。
    pub fn input(&self) -> &Value {
        &self.input
    }
}

#[async_trait]
pub trait ToolProvider: Send + Sync {
    async fn list_tools(&self) -> Result<Vec<ToolDescriptor>, ProviderError>;
    async fn describe_tool(&self, id: &ToolId) -> Result<ToolDescriptor, ProviderError>;
    /// **请求参数已换**（设计 §7.5、裁决 §一第 2 条）。四项方法名与其余三项一字不动。
    async fn invoke(
        &self,
        call: AuthorizedToolInvocation<'_>,
    ) -> Result<ToolResult, ProviderError>;
    async fn cancel(&self, call: &CallId) -> Result<(), ProviderError>;
}
