//! 边界层：子进程的内核层隔离（§256）。
//!
//! **Landlock 已落地（Task 6），bubblewrap 尚未（Task 7）。** [`Sandbox::spawn`] 在
//! Landlock 分支上于 fork 之后、exec 之前施加规则集（默认拒绝，只读放行系统路径，
//! 写权限只开 Task 根）；内核对 Landlock 的 ABI 为 0（该机制整个不存在）时以
//! [`SandboxError::MechanismUnavailable`] 拒绝启动，而不是放出一个零隔离的子进程
//! （设计第 4.3 节）。bubblewrap 分支仍 panic。两条分支的 [`Sandbox::capabilities`]
//! 都据实返回，故读本 crate 的接口约定无需先假定「有隔离」或「没有」。
//!
//! 只读强制分三层，本 crate 是最外一层。类型层（`continuum-workspace` 的
//! `WritablePath`）只做路径分量检查，结构上不可能检出「Task 根内有一个指向根外的
//! 符号链接」这类逃逸（设计第 4.1 节），该逃逸由本层的规则集或挂载命名空间承担，
//! 且由 `tests/isolation.rs` 显式覆盖。
//!
//! [`Sandbox::spawn`] 的 workspace 参数是 `&TaskWorkspace` 而非 `&Path`，故「把 Base
//! Workspace 当作 workspace 传进来」在类型上不可表达（设计第 4.2 节）。这只覆盖
//! workspace 这一个参数：**子进程的 argv 与环境变量由调用方给定**，其中不含 Base 路径
//! 是调用方义务，由第 13 节的用例验证，并由内核层兜底。
//!
//! 本 crate 不含写入 Base 的路径，也不依赖 Effect Journal 与 Policy——沙箱只负责
//! 约束子进程能做什么，不判断某次操作是否被授权。

mod landlock;

pub mod error;
pub mod sandbox;

pub use error::SandboxError;
pub use sandbox::{Sandbox, SandboxCapabilities};
