//! 外围中立接口。本 crate 只定义 trait 与登记表，不含实现，也不引用任何实现类型（§346）。
//!
//! 注册表（`registry`）加进来之后**实现仍然不在本 crate 里**：注册表持有的只是
//! `Arc<dyn ModelProvider>` / `Arc<dyn ToolProvider>`，没有一个是实现类型（设计 §4.1）。

pub mod connector;
pub mod model;
pub mod registry;
pub mod tool;

pub use connector::Connector;
pub use model::ModelProvider;
pub use registry::{ProviderRegistry, RegistryError, ToolCallError};
pub use tool::ToolProvider;
