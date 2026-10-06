//! 外围中立接口。本 crate 只定义 trait 与登记表，不含实现，也不引用任何实现类型（§346）。
//!
//! 注册表（`registry`）加进来之后**实现仍然不在本 crate 里**：注册表持有的只是
//! `Arc<dyn ModelProvider>` / `Arc<dyn ToolProvider>`，没有一个是实现类型（设计 §4.1）。
//!
//! **工具侧的暴露面**（设计 §3.1）：只开只读入口 `ProviderRegistry::list_tools` /
//! `ProviderRegistry::describe_tool`，以及唯一的调用入口 `invoke_tool`（后续 task）。
//! **没有 `tool_for`、也没有 `tool_providers()`**——它们要是存在，适配器就出了 crate。
//! **它们缺席的证据归 Task 6 的编译失败样例，本任务不对此作任何主张**——本处没有能证明
//! 「这两个名字编不过」的用例，只有上面这段说明文字，而说明文字不是证据。

pub mod connector;
pub mod model;
pub mod registry;
pub mod tool;

pub use connector::Connector;
pub use model::ModelProvider;
pub use registry::{ProviderRegistry, RegistryError, ToolCallError};
pub use tool::{AuthorizedToolInvocation, ToolProvider};
