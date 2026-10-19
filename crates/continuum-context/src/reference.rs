use continuum_core::model::Message;
use continuum_core::tool::ToolResult;

/// §280 的 `references[]` 元素。规范未给元素类型；设计定（§2.4 第 1 条）。
#[derive(Debug, Clone, PartialEq)]
pub enum Reference {
    /// 普通引用文本：文档、URL 内容、Project 片段、Intent 的文本投影。
    Text(String),
    /// 历史对话轮。复用 §315 的 `Message`（role + content），不另造 `Turn`。
    Conversation(Message),
    /// 工具结果。复用 §316 的 `ToolResult`，不造 wrapper。
    ToolOutput(ToolResult),
}
