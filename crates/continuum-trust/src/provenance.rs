//! `provenance[]` 的元素：来源描述，不是标签。
//!
//! [`ContextProvenance`] 回答「从哪来、带着什么可追的载荷」；标签由
//! [`crate::label()`] 投影。变体名与 `ContextTrust` 的变体同名不表示同一层面。

use serde::{Deserialize, Serialize};

/// `ContextPackage.provenance[]` 的元素。**一个片段一条**（P6A 设计 §4.2）。
///
/// **它不是标签记录，是来源描述**：`label()` 把它投影成 [`crate::ContextTrust`]。
/// 这枚类型存在的理由是 §71：「进入模型的每一段重要数据都应该知道来源」。
///
/// 字段全部 `pub`：它是一份纯数据记录，没有不变量可挂。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContextProvenance {
    /// 用户 authored 的指令。§71 的 `source = user_instruction`。
    ///
    /// **`historical` 是「这一轮是不是当前指令」的结构判别位**：
    /// - `false`：本次任务的当前指令，只能落 `instructions[]`；
    /// - `true`：对话历史里的**早前**用户轮，只能落 `references[]`。
    ///
    /// 两种都投影到 `USER_INSTRUCTION`——来源仍是用户，但「当前指令 vs 历史」由这个位
    /// 与数组共同表达。
    UserInstruction { historical: bool },
    /// Runtime 本层 authored 的文本（系统提示、安全/格式约束、工具 schema、包框架）。
    /// **第三方 provider 送来的系统消息/工具 schema 不走这里**：它们走
    /// [`ContextProvenance::WebUntrusted`]。
    TrustedRuntime,
    /// 已物化在项目里的内容（项目文件、项目文档）。
    ProjectData,
    /// **外部不可信内容**：网页、邮件、远端文档、连接器返回，**以及第三方 provider
    /// 提供的系统消息/工具 schema**。`provider_supplied()` 是本变体的具名构造入口。
    WebUntrusted,
    /// 工具执行的结果。**`external` 是「这段内容可确定为外部抓取物」的判别位**：
    /// `true` → 标签取 `WEB_UNTRUSTED`；`false` → `TOOL_OUTPUT`。
    ToolOutput { external: bool },
    /// 记忆 claim 的派生片段。`claim_id` 是**弱引用**：本 crate 不依赖 P6C 的
    /// `MemoryClaimId`，只收它的字符串形式；P6A 传 `claim.id().as_str()`。
    Memory { claim_id: String },
    /// 模型 authored 的**非制品**内容（assistant 轮、模型 verifier 的判定、合成摘要）。
    /// **模型产出的 artifact 不走这里**：它走 `Artifact { producer: Model }`。
    ModelOutput,
    /// 制品这个**载体**。制品本身不是一种信任类：它的标签由 `producer` 决定。
    /// `ArtifactProducer::Unknown` 是调用方无法确定产出通道时的显式取值。
    Artifact { producer: ArtifactProducer },
}

impl ContextProvenance {
    /// 本次任务的当前用户指令（`historical: false`）。
    pub fn user_instruction() -> Self {
        ContextProvenance::UserInstruction { historical: false }
    }

    /// 对话历史里的早前用户轮（`historical: true`）。
    pub fn conversation_history() -> Self {
        ContextProvenance::UserInstruction { historical: true }
    }

    /// Runtime 本层 authored 的文本。
    pub fn trusted_runtime() -> Self {
        ContextProvenance::TrustedRuntime
    }

    /// 第三方 provider 提供的系统消息/工具 schema：语义同 `web_untrusted()`，
    /// 只是入口名说清来源（fail-safe 到 `WEB_UNTRUSTED`，不升成指令）。
    pub fn provider_supplied() -> Self {
        ContextProvenance::WebUntrusted
    }

    /// 已物化在项目里的内容。
    pub fn project_data() -> Self {
        ContextProvenance::ProjectData
    }

    /// 外部不可信内容（网页、邮件、远端文档、连接器返回）。
    pub fn web_untrusted() -> Self {
        ContextProvenance::WebUntrusted
    }

    /// 工具执行的结果；`external` 原样带判别位。
    pub fn tool_output(external: bool) -> Self {
        ContextProvenance::ToolOutput { external }
    }

    /// 记忆 claim 的派生片段；`claim_id` 是弱引用。
    pub fn memory(claim_id: impl Into<String>) -> Self {
        ContextProvenance::Memory {
            claim_id: claim_id.into(),
        }
    }

    /// 模型 authored 的非制品内容。
    pub fn model_output() -> Self {
        ContextProvenance::ModelOutput
    }

    /// 制品载体；`producer` 原样带生产者判别位。
    pub fn artifact(producer: ArtifactProducer) -> Self {
        ContextProvenance::Artifact { producer }
    }
}

/// artifact 的产出通道。**不是标签**：它是 `label()` 的判别器之一。
///
/// `Unknown` 是显式取值，**不是 `Default` 实现**——默认实现会让调用方静默漏标。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArtifactProducer {
    Model,
    Tool,
    External,
    Project,
    Unknown,
}
