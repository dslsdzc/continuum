//! Continuum 核心类型。本 crate 不含 I/O。

pub mod connector;
pub mod error;
pub mod model;
pub mod tool;

pub use error::{CoreError, ProviderError};
