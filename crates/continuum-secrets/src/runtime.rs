//! 密钥运行时：按能力逐枚签发短命凭据（设计 §5.1、§5.2、§5.4）。
//!
//! §51：Agent 不直接获得所有密钥；每个节点只获得 minimum capability、short-lived
//! credential、scoped token。本模块是这条要求在本阶段的落点，也是本子项目的
//! **强制点 (3)**。
//!
//! 三条结构性约束（设计 §5.2）在本模块的落法：
//!
//! 1. **作用域由能力决定**——[`SecretsRuntime::issue`] 只收那枚能力本身，没有可以
//!    指定另一个作用域的位置；调用方拿窄能力去要宽凭据不是被检查掉，而是**要不到**
//!    （`tests/compile_fail/issue_has_no_scope_parameter.rs`）；
//! 2. **凭据短命且不越权**——凭据的到期时刻取「源声称的」与「能力的」二者中更早的
//!    （`issue` 里的 `min`，两个方向各有照片）；
//! 3. **没有「取全部」的入口**——只有按能力逐枚签发的
//!    [`SecretsRuntime::issue`]（`tests/compile_fail/there_is_no_way_to_ask_for_all_secrets.rs`）。

use std::sync::atomic::{AtomicU64, Ordering};

use continuum_capability::Capability;

use crate::error::SecretsError;
use crate::source::{CredentialSource, SecretMaterial};

/// 全局单调的代号发号器，两个用途共用：**运行时代号**（`new` 时取一次，此后不变）
/// 与**轮换代代号**（`new` 与 `rotate` 各取一次）。
///
/// **为什么是全局而不是每运行时的计数器**：代号是判等的唯一依据，若按运行各自从 0
/// 数起，同进程里 A 与 B 的运行时代号会一样、轮换代代号也会撞上，A 签出的凭据会被 B
/// 当成自己的，B 会按 **B 的源**给出同一个作用域的材料——一次混淆代理。全局发号让
/// 不同运行时之间、同一运行时的不同代之间都不可能撞号。
///
/// 照片：`tests/issue.rs` 的 `a_credential_from_another_runtime_is_rejected`
/// （A 的凭据在 B 上报 `ForeignCredential`，且有 A 自己可用的对照臂）。
static NEXT_TICKET: AtomicU64 = AtomicU64::new(0);

fn next_ticket() -> u64 {
    NEXT_TICKET.fetch_add(1, Ordering::Relaxed)
}

/// §103 列出的四类轮换触发事件。
///
/// 本枚举**逐项照录**这四类；[`RotationEvent::as_str`] 是无通配臂的穷尽 match，故
/// §103 之外再加一类会让它编译不过。四类各有一张照片：`tests/issue.rs` 的
/// `every_rotation_event_class_invalidates_old_credentials` 以 [`RotationEvent::ALL`]
/// 为迭代域逐项验证。
///
/// **残余缺口**（写下来而不是含糊过去）：新增变体会让 `as_str` 编译不过，但**不会**
/// 让 `ALL` 编译不过——「加了变体忘了加进 `ALL`」编译器抓不到。故
/// `the_four_rotation_event_classes_are_listed_by_name` 只能钉住 `ALL` 现有四项。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RotationEvent {
    /// Authority 被攻破。
    AuthorityCompromised,
    /// 永久故障转移。
    PermanentFailover,
    /// 设备撤销。
    DeviceRevoked,
    /// 凭据泄露。
    CredentialLeaked,
}

impl RotationEvent {
    /// §103 四类事件的名字。穷尽 match、**无通配臂**：加一类即编译不过。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AuthorityCompromised => "authority_compromised",
            Self::PermanentFailover => "permanent_failover",
            Self::DeviceRevoked => "device_revoked",
            Self::CredentialLeaked => "credential_leaked",
        }
    }

    /// 本阶段支持的全部轮换事件（§103 的四类）。
    pub const ALL: [RotationEvent; 4] = [
        RotationEvent::AuthorityCompromised,
        RotationEvent::PermanentFailover,
        RotationEvent::DeviceRevoked,
        RotationEvent::CredentialLeaked,
    ];
}

/// 签出的凭据：**作用域 + 到期时刻 + 取得材料的句柄（运行时代号 + 轮换代代号）**
/// （设计 §5.2）。
///
/// **不含能力本身**：凭据一旦签出即与那枚能力解耦，故后续无法凭它反推或扩大权限。
/// 这一条是**形状陈述**，没有运行期照片——字段私有、无公开构造函数，crate 外既读不到
/// 也造不出（唯一构造点在 [`SecretsRuntime::issue`]）。
///
/// **凭据上读得到的东西**（不止作用域与到期时刻，别把下面这几样漏掉）：
/// [`Credential::scope`] 与 [`Credential::expiry`] 两个访问器；
/// [`Credential::is_valid_at`] 给出的到期结论；以及 `Debug` 打出的四个字段——除作用域与
/// 到期时刻外还有两个句柄（运行时代号与轮换代代号）。**读不到的是能力与材料**：前者不在
/// 这个类型里，后者留在源里、每次访问现取
/// （`the_material_is_fetched_at_every_access_and_not_kept_on_the_credential`）。
/// 效果可见的那一半的照片是 `tests/issue.rs` 的
/// `a_credential_carries_the_capabilitys_scope_and_expiry`（它断言的是两个访问器，
/// 不是「只读得到这两样」）。
///
/// **也不含材料**：材料留在源里，每次访问现取（`the_material_is_fetched_at_every_access_and_not_kept_on_the_credential`）。
#[derive(Debug, Clone)]
pub struct Credential {
    scope: String,
    expiry: i64,
    /// 签出它的**运行时**的代号：`new` 时取一次，`rotate` 不改。用来判「这张凭据
    /// 是不是本运行时签的」。
    runtime: u64,
    /// 取得材料的句柄：签发时的**轮换代**代号。轮换使它前进，旧凭据随即被**取代**
    /// （[`SecretsError::Superseded`]，§5.4）——注意这里说的是「取代」不是「失效」：
    /// 本 crate 里「失效」指到期（[`SecretsError::CredentialExpired`]），两者是不同的原因。
    generation: u64,
}

impl Credential {
    /// 本凭据的作用域。它**等于**所据能力的作用域——`issue` 的签名里没有第二个来源。
    pub fn scope(&self) -> &str {
        &self.scope
    }

    /// Unix 毫秒的失效时刻，不晚于所据能力的 `expiry`。
    pub fn expiry(&self) -> i64 {
        self.expiry
    }

    /// 凭据自己的到期判定。`expiry` 是**失效时刻**：`now == expiry` 即已失效。
    ///
    /// 这是本 crate 里**唯一**比较凭据到期时刻的地方。它**不是**
    /// [`continuum_capability::Capability::is_valid_at`] 的重写：判的是凭据自己的到期
    /// 时刻（可能因源声称更短而早于能力的），凭据不含能力故也无从用那枚能力的判据。
    /// 能力的有效性判定在本 crate 里只有一处：`issue` 里对 `Capability::is_valid_at`
    /// 的调用。
    pub fn is_valid_at(&self, now: i64) -> Result<(), SecretsError> {
        if now < self.expiry {
            Ok(())
        } else {
            Err(SecretsError::CredentialExpired {
                expiry: self.expiry,
                now,
            })
        }
    }
}

/// 密钥运行时：持有凭据源、本运行时的代号与当前轮换代，按能力逐枚签发。
pub struct SecretsRuntime {
    source: Box<dyn CredentialSource>,
    /// 本运行时的代号：`new` 时取一次，**`rotate` 不改**。凭据记下它，用来判
    /// 「这张凭据是不是本运行时签的」。
    runtime: u64,
    /// 当前轮换代代号：`new` 与 `rotate` 各领一个全局号（见 [`NEXT_TICKET`]）。
    generation: u64,
    last_rotation: Option<RotationEvent>,
}

impl SecretsRuntime {
    pub fn new(source: Box<dyn CredentialSource>) -> Self {
        Self {
            source,
            // 运行时代号与轮换代代号各领一个全局号，都不是 0：两者若按运行各自从 0
            // 数起，两个运行时的凭据会互相通用（见 NEXT_TICKET）。
            runtime: next_ticket(),
            generation: next_ticket(),
            last_rotation: None,
        }
    }

    /// **唯一取得凭据的路径**（设计 §5.1）。凭据的作用域不超出所给的能力。
    ///
    /// 签名只收能力本身，故「拿窄能力去要宽凭据」**要不到**，而不是被检查掉——
    /// 调用方没有可以指定另一个 scope 的位置。作用域由能力决定这一条的照片：
    /// `tests/issue.rs` 的 `a_credential_carries_the_capabilitys_scope_and_expiry`。
    ///
    /// 能力的有效性判定**不在这里重写**：调 [`Capability::is_valid_at`] 并把它的
    /// [`continuum_capability::CapabilityError`] 转出（`SecretsError::Capability`）。
    ///
    /// `now` 由调用方给（本项目的既有约定：核不收时钟）。
    pub fn issue(&self, cap: &Capability, now: i64) -> Result<Credential, SecretsError> {
        cap.is_valid_at(now)?;

        let claimed = self.source.claimed_expiry(cap.scope(), now)?;
        // 两个方向都截：源声称更长 → 能力的；源声称更短 → 源的。
        // 照片：`a_longer_source_validity_is_truncated_...` 与
        // `a_shorter_source_validity_is_kept_...`（含文件源与环境变量源两套）。
        let expiry = claimed.min(cap.expiry());

        Ok(Credential {
            scope: cap.scope().to_owned(),
            expiry,
            runtime: self.runtime,
            generation: self.generation,
        })
    }

    /// 按 §103 的一类事件轮换：当前代号前进，旧凭据随即失效，新凭据照常签发。
    ///
    /// 事件本身不改签发的**行为**（四类的处置在本阶段相同），但它被记下，供
    /// [`SecretsRuntime::last_rotation`] 读出——「按事件使旧凭据失效并重发」这条要求
    /// 里，「按事件」这几个字至少要是可观察的，否则调用方传哪一类都一样，
    /// 参数就是装饰。
    pub fn rotate(&mut self, event: RotationEvent) {
        // 轮换代也取全局号：与 `new` 共用同一个发号器，故同一运行时内代号单调前进。
        // （`runtime` 不动：轮换不改变「这张凭据是不是本运行时签的」。）
        self.generation = next_ticket();
        self.last_rotation = Some(event);
    }

    /// 最近一次轮换是哪一类事件；从未轮换过则为 `None`。
    pub fn last_rotation(&self) -> Option<RotationEvent> {
        self.last_rotation
    }

    /// 取凭据的材料。这里判**三件**事，判的全是凭据这一侧：
    ///
    /// 1. 这张凭据是**本运行时**签出的（否则 [`SecretsError::ForeignCredential`]）；
    /// 2. 签出它的轮换代仍是当前代——即未被轮换取代（否则
    ///    [`SecretsError::Superseded`]，§5.4）；
    /// 3. `now` 未到凭据的到期时刻（否则 [`SecretsError::CredentialExpired`]）。
    ///
    /// 之后才把凭据的作用域交给源取材料——作用域是否被源覆盖由源自报
    /// （[`SecretsError::ScopeNotCovered`]）。材料此刻才从源取（设计 §5.2：凭据上
    /// 只有句柄）。
    ///
    /// **能力是否容许不在这里判，这里也看不到能力**：那一判在 [`SecretsRuntime::issue`]
    /// （`cap.is_valid_at(now)?`），签出后不再复核。故一次访问能否成功，与那枚能力
    /// 此刻是否还有效无关。一次访问失败有**三个互不相同**的原因，别把它们并成一句：
    ///
    /// - **撤销**：本阶段唯一能让一枚尚未到期的凭据被本运行时拒绝的是**轮换**（§5.4），
    ///   报 [`SecretsError::Superseded`]；
    /// - **不供料**：源可能不再覆盖该作用域（文件源 `reload` 后该条目被删、环境变量源的
    ///   变量被取消或置空），报 [`SecretsError::ScopeNotCovered`] / [`SecretsError::EmptyMaterial`]
    ///   ——那是源不给料了，不是撤销；
    /// - **到期**：凭据自己的到期时刻已过，报 [`SecretsError::CredentialExpired`]，第三回事。
    ///
    /// **也不要把这里读成「材料只有这里出得去」**：材料的另一处出口是
    /// [`CredentialSource::fetch`] 本身——它是公开 trait 方法，`FileCredentialSource::open`
    /// 之类的公开构造点也公开，故 `FileCredentialSource::open(path)?.fetch("repo/X", 0)?`
    /// 能直接拿到材料，不经过上面任何一条判定。本运行时是**强制执行点**，
    /// 不是本 crate 里唯一的材料出口。
    pub fn material(
        &self,
        credential: &Credential,
        now: i64,
    ) -> Result<SecretMaterial, SecretsError> {
        if credential.runtime != self.runtime {
            return Err(SecretsError::ForeignCredential {
                runtime: credential.runtime,
                current: self.runtime,
            });
        }
        if credential.generation != self.generation {
            return Err(SecretsError::Superseded {
                current: self.generation,
            });
        }
        credential.is_valid_at(now)?;
        self.source.fetch(credential.scope(), now)
    }
}
