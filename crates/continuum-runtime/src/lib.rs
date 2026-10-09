//! Runtime 的库面。
//!
//! 本 crate 的判据在**二进制**（设计下篇第 9 节：端到端测试驱动真实二进制），
//! 故 P3 子项目 F 之前这里只导出**端到端观察不到**的那部分——命令行解析的结果类型。
//!
//! **F 的 Task 2 起多了三样**，各自的理由都不是「想导出」，而是**bin 用不了别的**：
//!
//! - [`sandbox_select`]：`TaskError::SandboxSelect` 的 `#[from]` 目标是它，而该变体
//!   要随 [`TaskError`] 一起进 lib（变体一个不改）；
//! - [`error`] / [`TaskError`]：工具调用路径是 lib 函数（F 设计 §6.4），它要返回与
//!   命令路径**同一批**失败——另建一个 lib 侧错误类型会给同一批失败造出第二个类型；
//! - [`tool_call`]：**强制点 (2) 的铸币判定**由两条路径共用（F 设计 §3.2、§5.1），
//!   共享即只有一份定义，故它连同它调用的裁决小件一起落在这里。
//!
//! **P3 子项目 G 的 Task 1 起多了两样**（G 是模型调用路径，与 F 在同一个 crate 上
//! 各写各的，撞车点在 `error.rs` 与本文件的合并，按内容合）：
//!
//! - [`error::ModelCallError`] 与它的 [`model_call`]：模型调用路径的失败面。落点与
//!   `TaskError` 同一个文件是设计 §10.2 指定的（G 只增补那个文件，不动 F 的部分）；
//!   分类表与转换另起 `model_call`，理由见该模块的文档注释。
//!
//! 编排（`task_cmd` / `recover_cmd`）与 `startup` 留在 bin 内：它们要验证的性质
//! 「Base 出现该改动、Task 仍在」「第二次拒绝运行且只存在一条记录」本来就只有
//! 驱动真实二进制才看得见（设计第 9 节那张表），放进 lib 不会让它们更好验。
//! 同理，`main.rs` 里那份迁移装配清单不在这里导出——`tests/migrations.rs` 要对照的
//! 是**实际落库**的集合，不是那份清单的转录，故它驱动二进制，不 import。

pub mod cli;
pub mod error;
pub mod model_call;
pub mod sandbox_select;
pub mod tool_call;

pub use error::{ModelCallError, TaskError};
