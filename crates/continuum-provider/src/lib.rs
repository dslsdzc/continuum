//! 外围中立接口。本 crate 只定义 trait，不含实现，也不引用任何实现类型（§346）。

pub mod connector;
pub mod model;
pub mod tool;

pub use connector::Connector;
pub use model::ModelProvider;
pub use tool::ToolProvider;
