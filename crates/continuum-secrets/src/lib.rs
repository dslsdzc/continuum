//! 密钥运行时（设计第 5 节；《工程》§4.3 的「密钥运行时 ← Capability 执行点」）。
//!
//! 本 crate 是 P3 子项目 A 的**强制点 (3)**：按能力逐枚签发短命、限定作用域的凭据
//! （§51：Agent 不直接获得所有密钥；每个节点只获得 minimum capability、short-lived
//! credential、scoped token）。
//!
//! 分三层：
//! - [`runtime`]：唯一取得凭据的路径 [`SecretsRuntime::issue`]、凭据 [`Credential`]
//!   与材料的强制执行点 [`SecretsRuntime::material`]，以及 §103 的轮换事件
//!   [`RotationEvent`]；
//! - [`source`]：凭据源 [`CredentialSource`] 的两个真实现——文件
//!   [`FileCredentialSource`] 与环境变量 [`EnvCredentialSource`]（设计 §5.3），
//!   以及材料类型 [`SecretMaterial`]；
//! - [`error`]：本 crate 的错误类型。
//!
//! # 三条结构性约束（设计 §5.2）
//!
//! 1. 作用域由能力决定，调用方无从放宽——[`SecretsRuntime::issue`] 只收那枚能力本身；
//! 2. 凭据短命且不越权——凭据的到期不晚于所据能力的 `expiry`；
//! 3. 没有「取全部」的入口——只有按能力逐枚签发。
//!
//! 前两条与第三条的不可表达性各由 `tests/compile_fail/` 的样例钉住。
//!
//! # 凭据不落库
//!
//! [`Capability`](continuum_capability::Capability) 与 [`Credential`] 都**不落库**
//! （设计 §2.3、§7），本 crate 也不依赖任何持久化 crate。
//!
//! 材料有两处出口，**不要把其中一处读成唯一**：强制执行点是
//! [`SecretsRuntime::material`]——它判的全是**凭据这一侧**的三件事（凭据是否属于本
//! 运行时、是否已被轮换取代、是否到期），它**看不到也不复核能力**（能力是否容许在
//! [`SecretsRuntime::issue`] 就定了）；另一处是凭据源自己的 [`CredentialSource::fetch`]
//! ——公开的 trait 方法，源也是公开可构造的，故它取材料时**不经过**任何判定。
//!
//! # 与 §100 的据实偏离
//!
//! 无硬件后端，本阶段用文件 + 环境变量；§100 是 SHOULD，替换点是长期阶段的
//! TPM / Secure Enclave（详见 [`source`] 的模块文档）。

pub mod error;
pub mod runtime;
pub mod source;

pub use error::SecretsError;
pub use runtime::{Credential, RotationEvent, SecretsRuntime};
pub use source::{
    CredentialSource, EnvCredentialSource, FileCredentialSource, SecretMaterial, env_var_name,
};
