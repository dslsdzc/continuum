//! 连接器边界上的错误类型（设计 §6.1）。
//!
//! **没有产生方的变体不建**：注册期四条各自在
//! [`crate::ConnectorRegistry::register`] 里有真实产生方；调用期四条（本文件的下四条）
//! 各自在入口的对应步上有真实产生方（`src/entry.rs`）。设计 §6.1 的表共十行，
//! 截至本 task **十行齐**——最后落地的 `EffectAuthorizationRequired` 由 Task 5
//! 在入口**第 3 步**加上。
//!
//! `Credentials` / `Provider` 是**转出型**的（`#[from]`）：入口不新造凭据侧或后端侧的
//! 失败，只把 [`SecretsError`] / [`ProviderError`] 原样带出去——「是哪一种失败」永远由
//! 内层变体给出。`Credentials` **不展开** [`SecretsError`] 的八个变体：再抄一层就会与
//! 它漂移（与 P3A `CapabilityError::Persist` 的取舍同形）。

use continuum_capability::CapabilityKind;
use continuum_core::connector::{ConnectorId, ConnectorOp};
use continuum_core::ProviderError;
use continuum_effect::EffectType;
use continuum_secrets::SecretsError;

/// 连接器注册与调用边界上的错误。
///
/// 每一处核对都以**变体**区分失败种类，而不是一句笼统的话——调用方要能判出
/// 是「声明了没绑」还是「绑了没声明」（这两侧互为反面，设计 §6.1 的取材：
/// 只建一侧不算钉住），是「没注册这个服务」还是「这条操作没声明」，是「出示了错
/// 种类的能力」还是「凭据拿不到」还是「后端挂了」。**失败路径的用例都要断言是哪一种**
/// （本项目纪律 3），故变体的粒度就是判据的粒度。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ConnectorError {
    /// 声明的操作里有一条没有对应的绑定——绑定不完备。
    #[error("连接器 {} 声明的操作 {} 没有绑定", connector.as_str(), op.as_str())]
    UnboundOperation {
        connector: ConnectorId,
        op: ConnectorOp,
    },
    /// 绑定里有一条对应的操作不在声明集内——双向覆盖的另一侧。
    #[error(
        "连接器 {} 绑定了未声明的操作 {}",
        connector.as_str(),
        op.as_str()
    )]
    UndeclaredBoundOperation {
        connector: ConnectorId,
        op: ConnectorOp,
    },
    /// 同一个连接器内，一枚 `CapabilityKind` 被两条操作绑定——「一一」被破坏
    /// （设计 §3.2 第 3 条）。
    ///
    /// 理由（设计 §3.3）：绑定的粒度存在，正是为了让 §125 的每条操作**能单独不授**
    /// （「`GitHub.merge` 能单独不授」）。两条操作绑同一枚 kind，则授权其一即授权另一，
    /// 粒度退回服务级——那正是 §125 禁止的形态。
    ///
    /// 照片：`tests/register.rs` 的 `two_operations_bound_to_the_same_kind_are_rejected`。
    #[error("连接器 {} 的两条操作绑定了同一枚能力 {}", connector.as_str(), kind.as_str())]
    DuplicateKindBinding {
        connector: ConnectorId,
        kind: CapabilityKind,
    },
    /// 操作串的服务半边不等于本连接器的 id（设计 §3.2 第 5 条）。
    ///
    /// `ConnectorOp` 只要求串里有一个 `.`，不保证操作属于哪个服务。服务半边对不上的
    /// 操作**永不可达**——入口第 1 步按服务半边解析连接器，永远解析不到它——故这条
    /// 核对把它挡在注册期。
    ///
    /// 照片：`tests/register.rs` 的
    /// `an_operation_whose_service_half_is_not_this_connector_is_rejected` 与
    /// `the_service_half_is_compared_case_sensitively`（后者钉「逐字比较、不折叠大小写」）。
    #[error(
        "连接器 {} 不能声明服务半边不是自己的操作 {}",
        connector.as_str(),
        op.as_str()
    )]
    OperationServiceMismatch {
        connector: ConnectorId,
        op: ConnectorOp,
    },

    /// 该服务没有任何已注册的连接器（入口第 1 步）。
    ///
    /// 连接器由操作串的服务半边（第一个 `.` 之前的子串）解析；解析不到即拒。
    ///
    /// 照片：`tests/invoke.rs` 的 `an_unregistered_connector_is_rejected`。
    #[error("没有注册过服务 {} 的连接器", connector.as_str())]
    UnknownConnector { connector: ConnectorId },

    /// 操作不在该连接器声明的操作集内（入口第 2 步）。
    ///
    /// §125 只规定「按操作细分」，是入口这**一条**核对让它第一次有了强制：
    /// 连接器没声明过的操作，走不到实现。
    ///
    /// 照片：`tests/invoke.rs` 的 `an_undeclared_operation_is_rejected`。
    #[error("连接器 {} 没有声明操作 {}", connector.as_str(), op.as_str())]
    UndeclaredOperation {
        connector: ConnectorId,
        op: ConnectorOp,
    },

    /// 出示的那枚能力的 kind 与绑定的 kind 不是同一枚（入口第 4 步）。
    ///
    /// **本条只管一个方向**：绑定 kind 不在 `for_effect` 的像里、却出示了效应臂时，
    /// 效应臂给出的 kind 必在像里（`AuthorizedEffect::new` 保证），两者必不相等，故由本步拦。
    /// 反过来的那个方向（绑定 kind 在像里、却出示非效应臂）是**第 3 步**的事——那是
    /// [`ConnectorError::EffectAuthorizationRequired`]。两步不得并成一条（设计 §5.3）。
    ///
    /// 照片：`tests/invoke.rs` 的 `a_presented_capability_of_another_kind_is_rejected`
    /// （三个字段逐字比）与 `the_reverse_mismatch_is_caught_by_the_fourth_step_not_the_third`
    /// （后者钉「反过来的那个方向由本步拦，而不是第 3 步」）。
    #[error(
        "操作 {} 绑定的是能力 {}，出示的是 {}",
        op.as_str(),
        bound.as_str(),
        presented.as_str()
    )]
    AuthorizationMismatch {
        op: ConnectorOp,
        bound: CapabilityKind,
        presented: CapabilityKind,
    },

    /// 绑定 kind 落在 `for_effect` 的像里（**这条操作有外部效应**）、却没走效应臂
    /// （入口第 3 步）。
    ///
    /// **这是 fail-open 的那一侧**：有外部效应却没走强制点 (2)（设计 §3.5、§5.3）。
    /// 与 [`ConnectorError::AuthorizationMismatch`] 是**两条不同的核对，不得并成一条**：
    /// 本条管「有没有走强制点 (2)」，第 4 步管「拿的是不是那一枚动作」。并成一条之后，
    /// `<正确的 kind> + 错误的臂` 就能过。
    ///
    /// `effect` 是**由绑定 kind 推出的**那条效应（[`CapabilityKind::effect`]），不是出示方
    /// 给的。**反过来的那个方向到不了本变体**：绑定 kind 不在像里时推不出效应，这个字段
    /// **没有值可填**，那条路径由第 4 步的 `AuthorizationMismatch` 拦（设计 §5.3 的两行表）。
    ///
    /// 照片：`tests/invoke.rs` 的 `an_effect_operation_presented_with_a_bare_capability_is_rejected`
    /// （`effect` 逐字比，且实现未被调用）。
    #[error("操作 {} 有外部效应（{}），必须出示效应授权", op.as_str(), effect.as_str())]
    EffectAuthorizationRequired {
        op: ConnectorOp,
        effect: EffectType,
    },

    /// 凭据这条路失败：**原样转出** [`SecretsError`]，不重编、不吞成一句笼统的话。
    ///
    /// 入口只在两处转出它：`issue`（能力已失效、源不覆盖该作用域、源读不出来）与
    /// `material`（凭据已被轮换取代、已到期）。后两个内层变体**经本入口不可达**——
    /// 一次调用之内没有可插入 `rotate` 的位置，凭据也是本次调用自己刚签出的
    /// （设计 §6.1 的 B3 段）；类型仍保留它们，理由同那一段。
    ///
    /// 照片：`tests/invoke.rs` 的 `an_expired_capability_cannot_reach_the_implementation`
    /// 与 `a_scope_the_source_does_not_cover_is_rejected`（内层变体逐字段断言）。
    #[error("凭据不可用: {0}")]
    Credentials(#[from] SecretsError),

    /// **后端错误**：§124 的接口本就以 [`ProviderError`] 报错，实现原样带出。
    ///
    /// **独立于 [`ConnectorError::Credentials`]**：后端不可用与凭据拿不到是两回事，
    /// 并成一个变体会让「凭据被拒」这条路径上的消息说一件不真的事（设计 §6.1）。
    ///
    /// 照片：`tests/invoke.rs` 的 `a_backend_error_comes_back_as_provider`（内层变体
    /// 逐字段断言）。
    #[error("连接器实现报错: {0}")]
    Provider(#[from] ProviderError),
}

