//! Protected Core：§322 的五个字段槽位与 §321 的五类语义（设计 §3、§4）。
//!
//! 本 crate 是 ENG-001 方向 (a) 的强制面：学习侧（`continuum-learning`）只能读
//! `ProtectedCore` 的指纹，看不到它的私有字段，也没有任何修改既有核心的入口。
//! 本 crate 的 workspace 内依赖为空数组，这是硬断言：任何指向 workspace 内别的
//! crate 的直接边都会让 `continuum-runtime` 的依赖方向用例变红。外部依赖只有
//! `sha2`（摘要）与 `thiserror`（`ProtectedError` 的 `Display`），它们不在这张
//! workspace 边表里。
//!
//! 五个字段私有，唯一构造点是 [`seal`]（收 [`ProtectedCoreParts`]）；封存后的核心
//! 只提供五个同名访问器与 [`ProtectedCore::fingerprint`]。
//!
//! # 规范编码
//!
//! [`ProtectedCore::fingerprint`] 每次按五个字段现算，不是第六个字段。编码固定按
//! `authority → capability → contract → audit → effect` 的次序逐组进行：
//!
//! 1. 一字节组名长度，后接组名的 ASCII 字节；
//! 2. 一字节状态：`0` 表示 `Unavailable`，`1` 表示 `Committed`；
//! 3. 仅 `Committed` 再编入 8 字节大端版本号与 32 字节摘要。
//!
//! 不使用分隔符猜边界。`CoreFingerprint` 是这一串编码的 SHA-256。编码只在本 crate
//! 的当前版本内解释，**不承诺跨版本稳定**：编码一改，全部旧学习提案按设计过期。

mod commitment;
mod core;
mod error;
mod semantic;
mod version;

pub use commitment::RuleCommitment;
pub use core::{CoreFingerprint, ProtectedCore, ProtectedCoreParts, seal};
pub use error::ProtectedError;
pub use semantic::ProtectedSemantic;
pub use version::{RuleDigest, RuleVersion};
