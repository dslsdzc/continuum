//! 驱动侧的密钥运行时装配（设计 §4.1：**驱动装配好传进来**——不在连接器里自建，
//! 连接器自带凭据源会让「凭据从哪来」在装配处看不见；设计 §11 第 15 条把这条边
//! 的所有者写死为 B）。
//!
//! # 配置来源是本实现自定的
//!
//! 设计只写了「驱动装配」，**没写凭据源的位置从哪来**。本实现自定如下，不读成规范要求：
//!
//! - 设了环境变量 `CONTINUUM_CREDENTIALS` → 文件凭据源，值是文件的路径；
//! - 未设 → 环境变量凭据源，作用域前缀 `CONTINUUM_SECRET_`。
//!
//! 这两处取值（前缀与 TTL）同样出自本实现：设计没给判据，P3A 的实现也没留默认值。
//! 记入 B 计划的「遗留」第 7 条。
//!
//! # 读不出来即失败（fail-closed），不吞掉它
//!
//! 设了 `CONTINUUM_CREDENTIALS` 而那个文件读不出来（不存在、权限、格式错）时，
//! [`assemble`] 把 [`SecretsError`] 原样上抛，调用方据此拒绝启动。**代价**是驱动在
//! 配错一个变量时完全起不来；**不吞它的理由**是另一种代价更大：吞掉它等于把「凭据源
//! 配错了」这件事拖到第一次真的要签发凭据时才现形——那时已经在一个正在跑的副作用
//! 路径上，而不是在一个可以立刻重打命令的启动点上。这也正是「配置错误要拒绝、
//! 不要静默退化」那条（`cli.rs` 的重复选项、`save_policy` 的 `OR REPLACE` 同一条）。
//!
//! # 装配出来的运行时今天没有消费者
//!
//! 如实写明：本轮**没有**。它的唯一消费者是 B 的连接器入口（`continuum-connector`
//! 里），而 `runtime → connector` 那条边按接缝裁决 §六.3 不接。**本值与 §11 第 13b 条
//! 那条「B 的入口在生产路径上没有调用方」同源**：接上时它作为
//! `ConnectorRegistry::new(runtime)` 的入参（设计 §4.1）。故本步**不假装它被用到了**
//! ——`main.rs` 里那个绑定活到 `main` 返回，仅此而已。

use continuum_secrets::{
    CredentialSource, EnvCredentialSource, FileCredentialSource, SecretsError, SecretsRuntime,
};

/// 凭据源位置的所在：文件源读它的路径，未设则退到环境变量源。
///
/// **设计未规定，本实现自定**（见模块文档）。
pub(crate) const CREDENTIALS_ENV: &str = "CONTINUUM_CREDENTIALS";

/// 环境变量源的作用域前缀。P3A 的用例与文档用的是这个前缀
/// （`continuum-secrets/tests/source_env.rs`），本实现沿用。
///
/// **设计未规定，本实现自定。**
const ENV_PREFIX: &str = "CONTINUUM_SECRET_";

/// 环境变量源声称的有效期长度（毫秒）。取 5 分钟：凭据短命是本层的目的（§51 的
/// short-lived credential），但设计没有给数，故这个数是**本实现的取值**，
/// 不是规范要求。
///
/// 它只是源**声称**的上界：`issue` 取它与能力 `expiry` 里更早的那一个
/// （`continuum-secrets/src/runtime.rs`），故宽于此值不会放宽任何凭据。
const DEFAULT_TTL_MS: i64 = 5 * 60 * 1000;

/// 装配密钥运行时。
///
/// 设了 [`CREDENTIALS_ENV`] 就用文件源（读不出来即 `Err`，**fail-closed**——
/// 理由见模块文档）；未设则用环境变量源。
pub(crate) fn assemble() -> Result<SecretsRuntime, SecretsError> {
    let source: Box<dyn CredentialSource> = match std::env::var_os(CREDENTIALS_ENV) {
        Some(path) => Box::new(FileCredentialSource::open(path)?),
        None => Box::new(EnvCredentialSource::new(ENV_PREFIX, DEFAULT_TTL_MS)),
    };
    Ok(SecretsRuntime::new(source))
}
