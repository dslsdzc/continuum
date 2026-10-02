//! 边界层：子进程的内核层隔离（§256）。
//!
//! 只读强制分三层，本 crate 是最外一层。类型层（`continuum-workspace` 的
//! `WritablePath`）只做路径分量检查，结构上不可能检出「Task 根内有一个指向根外的
//! 符号链接」这类逃逸（设计第 4.1 节），该逃逸由本层的规则集或挂载命名空间承担。
//!
//! [`Sandbox::spawn`] 的参数是 `&TaskWorkspace` 而非 `&Path`：Base Workspace 无法
//! 出现在这个位置上，故「把 Base 交给子进程」在类型上不可表达（设计第 4.2 节）。
//!
//! 本 crate 不含写入 Base 的路径，也不依赖 Effect Journal 与 Policy——沙箱只负责
//! 约束子进程能做什么，不判断某次操作是否被授权。

pub mod error;
pub mod sandbox;

pub use error::SandboxError;
pub use sandbox::{Sandbox, SandboxCapabilities};
