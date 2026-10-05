//! 注册入口：绑定与声明的双向覆盖（设计 §3.2.1）。

use std::collections::{HashMap, HashSet};

use async_trait::async_trait;
use continuum_capability::CapabilityKind;
use continuum_core::connector::{ConnectorDescriptor, ConnectorId, ConnectorOp};
use continuum_core::ProviderError;
use continuum_secrets::SecretMaterial;
use serde_json::Value;

use crate::binding::OpBinding;
use crate::error::ConnectorError;

/// 连接器作者实现的是本 trait（§4.5 有它为什么不是 §124 那个）。`Send + Sync` 与 §124 的
/// `Connector` 同要求，因为适配器要把实现放进一个满足 `Connector` 的类型里。
#[async_trait]
pub trait ConnectorImpl: Send + Sync {
    /// **复用 §124 的 `ConnectorDescriptor`，不另建第二个描述类型**（设计 §3.2.1）。
    fn descriptor(&self) -> ConnectorDescriptor;

    /// 每个已声明操作恰一条。**这就是绑定的产生点**，唯一产生点是
    /// [`ConnectorRegistry::register`]。
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

/// **注册期全部核对的唯一产生点**（设计 §3.2.1）。
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
#[derive(Default)]
pub struct ConnectorRegistry {
    /// 已注册的连接器本体，按 id 索引。入口（Task 4）从这里取实现。
    connectors: HashMap<ConnectorId, Box<dyn ConnectorImpl>>,
    /// 已注册的连接器各自的绑定，按 id 与操作索引。**`ConnectorOp` 只有 `Hash` / `Eq`、
    /// 没有 `Ord`**（`crates/continuum-core/src/connector.rs:26`），故用 `HashMap`
    /// 而不是 `BTreeMap`。
    bindings: HashMap<ConnectorId, HashMap<ConnectorOp, CapabilityKind>>,
}

impl ConnectorRegistry {
    pub fn new() -> Self {
        Self::default()
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
