//! 入口：**唯一一条通往 `Connector::invoke` 的路**（决定 B-2，设计 §5.3）。
//!
//! 本 task 只做**效应臂**：入口收 [`AuthorizedEffect`]，不收裸能力。两臂的
//! `ConnectorAuthorization` 由 Task 5 加上。**中间态是 fail-closed 的**：非效应型的操作
//! 在本 task 里**根本递不进来**（入口没有收裸能力的入口），故不存在「绕开强制点 (2)
//! 做副作用」的窗口。
//!
//! 入口做设计 §5.3 的**四步**核对，本 task 落第 **1、2、4** 步——第 3 步（绑定 kind
//! 落在 `for_effect` 的像里、却没出示效应臂 → `EffectAuthorizationRequired`）由 Task 5 加。
//! **四步都在调用实现之前**，故任一步拒，实现都没被调用（逐条照片见 `tests/invoke.rs`）。
//! 次序只决定同时犯两种错时报哪一种 `Err`，不决定放行与否（与 P3A `authorize` 的写法同形）。

use continuum_capability::AuthorizedEffect;
use continuum_core::connector::{ConnectorId, ConnectorOp};
use continuum_provider::Connector;
use serde_json::Value;

use crate::adapter::Adapter;
use crate::error::ConnectorError;
use crate::registry::{ConnectorRegistry, service_half};

impl ConnectorRegistry {
    /// **唯一一条通往 `Connector::invoke` 的入口**（决定 B-2）。
    ///
    /// 四步都在**调用实现之前**；四步全过才可能返回 `Ok`。次序只决定同时犯两种错时
    /// 报哪一种 `Err`，不决定放行与否（与 P3A `authorize` 的写法同形）。
    ///
    /// `now` 由调用方给：本项目的既有约定是「核不收时钟」（`Capability::is_valid_at`
    /// 的文档；`continuum-secrets` 的 `CredentialSource` 同）。**入口内的两次判定
    /// （`issue` 与 `material`）共用这同一个 `now`，入口自己不取第二个时钟**——两次各取
    /// 一次，「签发时未过期、取料时已过期」那一格就会被抹掉，而它正是 §103 与 §51
    /// 要看得见的那一格（设计 §5.1，裁决 B2）。
    ///
    /// # 限度（设计 §2.2）
    ///
    /// `Connector::invoke` 是公开方法，任何人都能直接调它。本入口能给的不是
    /// 「外部调不到 `invoke`」，而是「**本入口**要过一个只有核对通过才产生的值」
    /// （效应臂是 `AuthorizedEffect`）。若将来绕开，那是类型上表达得出来的选择，
    /// 不是「忘了接线」。
    pub async fn invoke(
        &self,
        authorization: &AuthorizedEffect,
        op: &ConnectorOp,
        input: Value,
        now: i64,
    ) -> Result<Value, ConnectorError> {
        // 第 1 步：按操作串的服务半边解析连接器。解析之所以唯一，靠的是注册期那条
        // 「服务半边 == 连接器 id」的核对（设计 §3.2 第 5 条）——没有它，一个连接器
        // 可以声明不属于自己的操作，而那条操作在这里永远解析不到。
        let id = ConnectorId::new(service_half(op));
        let connector = self
            .connector(&id)
            .ok_or_else(|| ConnectorError::UnknownConnector {
                connector: id.clone(),
            })?;

        // 第 2 步：`op` 必须在该连接器**声明的操作集**内，否则 `UndeclaredOperation`
        // ——§125 的「按操作细分」由此第一次有了强制。
        //
        // 读的是绑定表：注册期的双向覆盖（`register` 的第 2 条核对）保证「绑定的键集
        // == 声明的操作集」，故它与 `descriptor().operations()` 是同一个集合，且不在
        // 每次调用上再跑一遍连接器作者的 `descriptor()`。**第 4 步的绑定也取自这一处**：
        // 「这条操作声明过没有」只有一个来源；若一处读 `descriptor()`、一处读绑定表，
        // 一个在两次 `descriptor()` 之间换了声明集的实现就会让两处各说各话。
        let Some(bound) = self.bound_kind(&id, op) else {
            return Err(ConnectorError::UndeclaredOperation {
                connector: id,
                op: op.clone(),
            });
        };

        // 第 3 步（Task 5）：绑定 kind 落在 `for_effect` 的像里、却没出示效应臂
        // → `EffectAuthorizationRequired`。本 task 的入口只收效应臂，非效应臂递不进来。

        // 第 4 步：出示值给出的 kind 必须**等于**绑定的 kind。本条只管这一侧：
        // 反过来的那一侧（绑定 kind 在像里却没走效应臂）是第 3 步的事，两步不得并成
        // 一条（设计 §5.3）——并了之后，`<正确的 kind> + 错误的臂` 就能过，而那条路径
        // 恰恰是绕开强制点 (2) 做副作用，那是 fail-open 的一侧。
        let presented = authorization.capability().kind();
        if presented != bound {
            return Err(ConnectorError::AuthorizationMismatch {
                op: op.clone(),
                bound,
                presented,
            });
        }

        // 凭据路径（设计 §4.1）：核对全过、调用实现之前，按**出示的那枚能力**逐次签发
        // 凭据并取料（强制点 (3)）。**逐次签发而不是构造时一次**：作用域随能力逐次不同
        // （同一次运行里可以有 `git.push:origin/main` 与 `git.push:origin/release` 两枚）。
        //
        // B **不重铸能力、也不复核时效**：能力原样交给 `issue`；「能力还有效吗」只有
        // `Capability::is_valid_at` 一个产生点，`issue` 已把它转成
        // `SecretsError::Capability`，这里只**不吞**那个 `Err`（设计 §4.3）。
        let credential = self.secrets().issue(authorization.capability(), now)?;
        let material = self.secrets().material(&credential, now)?;

        // 逐次存活的适配器：材料只在调用期间存活，且走**带外参数**、绝不进 `input`
        // （设计 §4.5）。适配器的注释里写着轮换的可观察范围与它的限度。
        let adapter = Adapter::new(connector, &material);
        adapter
            .invoke(op, input)
            .await
            .map_err(ConnectorError::Provider)
    }
}
