//! 注册入口：[`ConnectorRegistry::register`]——**注册期全部四条核对的唯一产生点**
//! （设计 §3.2.1）。四条是：声明集 ↔ 绑定集的双向覆盖（两侧）、一一
//! （`DuplicateKindBinding`）、操作的服务半边相符（`OperationServiceMismatch`）。
//!
//! 注册表也持有**凭据运行时**（设计 §4.1：驱动装配好传进来），入口
//! （[`crate::ConnectorRegistry::invoke`]，在 `src/entry.rs`）从它逐次签发凭据。

use std::collections::{HashMap, HashSet};

use async_trait::async_trait;
use continuum_capability::CapabilityKind;
use continuum_core::connector::{ConnectorDescriptor, ConnectorId, ConnectorOp};
use continuum_core::ProviderError;
use continuum_secrets::{SecretMaterial, SecretsRuntime};
use serde_json::Value;

use crate::binding::OpBinding;
use crate::error::ConnectorError;

/// 连接器作者实现的是本 trait（§4.5 有它为什么不是 §124 那个）。`Send + Sync` 与 §124 的
/// `Connector` 同要求，因为适配器要把实现放进一个满足 `Connector` 的类型里。
#[async_trait]
pub trait ConnectorImpl: Send + Sync {
    /// **复用 §124 的 `ConnectorDescriptor`，不另建第二个描述类型**（设计 §3.2.1）。
    fn descriptor(&self) -> ConnectorDescriptor;

    /// **声明集与绑定集的键集相等**——这是四条核对实际保证的。**这就是绑定的产生点**，
    /// 唯一产生点是 [`ConnectorRegistry::register`]。
    ///
    /// **（订正，原话照留）**：这里原写着「每个已声明操作恰一条」——那句话**超出了四条
    /// 核对实际保证的**。四条核对保证的是「声明集 ≡ 绑定集的**键集**」（双向覆盖的两侧、
    /// 一一、服务半边相符），**不保证**「同一个 op 只有一条绑定」：同一个 op 给两条
    /// **不同 kind** 的绑定时四条全过，随后 `register` 收进 `HashMap` 时**后者胜且静默**
    /// （仍返回 `Ok`），入口取到哪一条取决于 `Vec` 的顺序。设计 §3.2.1 的四条核对与
    /// §6.1 的十个错误变体里都没有对应判据，故本 crate 不新增核对、不造变体
    /// （`docs/superpowers/p3bcdf-followups.md` §八.23）。
    fn bindings(&self) -> Vec<OpBinding>;

    /// 比 §124 的 `invoke` 多一个材料入参（决定 B-4）：调用实现之前，入口按出示的
    /// 能力**逐次签发**凭据并取料，材料经此入参到达实现，§124 的 `invoke` 签名不动。
    async fn invoke_with(
        &self,
        op: &ConnectorOp,
        input: Value,
        material: &SecretMaterial,
    ) -> Result<Value, ProviderError>;
}

/// 操作串的**服务半边**：第一个 `.` 之前的子串（设计 §3.2.1 末段）。
///
/// 入口第 1 步（Task 4）也按它解析连接器，注册期的服务半边核对是那次解析唯一性的来源。
/// **但那条唯一性论证还依赖 `ConnectorId` 在表里唯一**——同一个 id 二次注册会静默覆盖
/// （`docs/superpowers/p3bcdf-followups.md` §八.20），那时服务半边只能解析到最后一条。
///
/// 比较是**逐字比较、不折叠大小写**——被否掉的替代是折叠大小写：折叠只到
/// 「modulo ASCII 大小写」，会让 `github` 与 `GitHub` 被判为同一个服务。
///
/// `split_once` 一定取得到 `.`：`ConnectorOp` 的字段私有、构造与反序列化都经
/// `ConnectorOp::new`，而它拒绝不含 `.` 的串（`crates/continuum-core/src/connector.rs:31-37`）。
pub(crate) fn service_half(op: &ConnectorOp) -> &str {
    op.as_str()
        .split_once('.')
        .expect("ConnectorOp::new 保证串里至少有一个 `.`")
        .0
}

/// **注册期全部核对的唯一产生点**（设计 §3.2.1），也是入口
/// [`ConnectorRegistry::invoke`]（`src/entry.rs`）读取已登记连接器与绑定的地方。
///
/// 四条核对：双向覆盖的两侧（第 2 条）、一一（第 3 条）、服务半边相符（第 5 条）。
///
/// # 它不核什么（写下来，免得被读成它核过）
///
/// 上述四条核对**没有一条**核对「这条操作该绑哪一枚 `CapabilityKind`」：规范没有那条判据，
/// 本 crate 也没有发明（设计 §3.4）。故作者把 `GitHub.merge` 绑到 `Github(CreatePr)` 上，
/// 核对全过、**注册成功**——照片见 `tests/register.rs` 的
/// `a_mis_bound_operation_still_registers`。这条限度是本 trait 与
/// `register` 的**口径**，不是本 crate 没做够。
pub struct ConnectorRegistry {
    /// 密钥运行时：入口按出示的那枚能力**逐次**签发凭据并取料（设计 §4.1、强制点 (3)）。
    ///
    /// **由驱动装配好传进来**（[`ConnectorRegistry::new`]）：`SecretsRuntime` 的构造
    /// 要读凭据源的位置与内容，那是**外部配置**，与驱动的装配处同一职责。连接器自带
    /// 凭据源会让「凭据从哪来」在装配处看不见（设计 §4.1 取的那种形状）。
    secrets: SecretsRuntime,
    /// 已注册的连接器本体，按 id 索引。入口（Task 4）从这里取实现。
    connectors: HashMap<ConnectorId, Box<dyn ConnectorImpl>>,
    /// 已注册的连接器各自的绑定，按 id 与操作索引。**`ConnectorOp` 只有 `Hash` / `Eq`、
    /// 没有 `Ord`**（`crates/continuum-core/src/connector.rs:26`），故用 `HashMap`
    /// 而不是 `BTreeMap`。
    ///
    /// 注册期的双向覆盖使**本表的键集恰等于该连接器声明的操作集**——入口第 2 步
    /// （「操作已声明」）与第 4 步（取绑定的 kind）因此共用这一处，不再读第二份声明集。
    bindings: HashMap<ConnectorId, HashMap<ConnectorOp, CapabilityKind>>,
}

impl ConnectorRegistry {
    /// **注册表没有无参构造**：凭据运行时是它的必需的构件，故构造它时一起收下。
    /// 这**不是**「调用方可以选装」——没有运行时，入口的凭据路径就无路可走。
    pub fn new(secrets: SecretsRuntime) -> Self {
        Self {
            secrets,
            connectors: HashMap::new(),
            bindings: HashMap::new(),
        }
    }

    /// 入口第 1 步用：按 id 取回已注册的实现。
    ///
    /// **不返回 `Option<&Box<…>>` 的里层**：`Box` 是实现细节，入口只要那个 trait 对象。
    pub(crate) fn connector(&self, id: &ConnectorId) -> Option<&dyn ConnectorImpl> {
        self.connectors.get(id).map(Box::as_ref)
    }

    /// 入口第 2、4 步共用：取某连接器的某操作绑定的 kind；没有绑定即该操作未声明
    /// （注册期的双向覆盖保证「有声明必有绑定」，故本表的键集就是声明集）。
    pub(crate) fn bound_kind(
        &self,
        id: &ConnectorId,
        op: &ConnectorOp,
    ) -> Option<CapabilityKind> {
        self.bindings.get(id)?.get(op).copied()
    }

    /// 入口的凭据路径用：签发与取料都经它（设计 §4.1）。
    pub(crate) fn secrets(&self) -> &SecretsRuntime {
        &self.secrets
    }

    /// 收下一个连接器：先从描述符取出声明集、从实现取出绑定集，核过四条核对后才登记。
    ///
    /// 核对失败即返回 `Err`，**不留下半登记状态**——四条核对都发生在写入之前。
    /// 次序只决定同时犯两种错时报哪一种 `Err`，不决定放行与否（四条全过才返回 `Ok`）。
    pub fn register(&mut self, connector: Box<dyn ConnectorImpl>) -> Result<(), ConnectorError> {
        let descriptor = connector.descriptor();
        let id = descriptor.id().clone();
        let declared = descriptor.operations().to_vec();
        let bindings = connector.bindings();

        // 核对（第 2 条之一）：声明的每一条都有绑定。
        for op in &declared {
            if !bindings.iter().any(|b| b.op() == op) {
                return Err(ConnectorError::UnboundOperation {
                    connector: id.clone(),
                    op: op.clone(),
                });
            }
        }

        // 核对（第 2 条之另一侧）：绑定的每一条都已声明。
        for binding in &bindings {
            if !declared.contains(binding.op()) {
                return Err(ConnectorError::UndeclaredBoundOperation {
                    connector: id.clone(),
                    op: binding.op().clone(),
                });
            }
        }

        // 核对（第 3 条）：「一一」——同一连接器内，一枚 kind 不得被两条操作绑定。
        let mut seen: HashSet<CapabilityKind> = HashSet::new();
        for binding in &bindings {
            if !seen.insert(binding.kind()) {
                return Err(ConnectorError::DuplicateKindBinding {
                    connector: id.clone(),
                    kind: binding.kind(),
                });
            }
        }

        // 核对（第 5 条）：每条操作的服务半边必须**逐字**等于本连接器的 id。
        for op in &declared {
            if service_half(op) != id.as_str() {
                return Err(ConnectorError::OperationServiceMismatch {
                    connector: id.clone(),
                    op: op.clone(),
                });
            }
        }

        let bound = bindings
            .into_iter()
            .map(|b| (b.op().clone(), b.kind()))
            .collect();
        self.bindings.insert(id.clone(), bound);
        self.connectors.insert(id, connector);
        Ok(())
    }
}
