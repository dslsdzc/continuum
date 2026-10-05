//! 逐次调用的适配器（决定 B-4，设计 §4.5）。
//!
//! §124 的 `Connector::invoke(&self, op, input)` **没有放凭据的位置**
//! （`crates/continuum-provider/src/connector.rs:16`）。本适配器是那条缺口的落法：
//! 它持 [`ConnectorImpl`] 与 [`SecretMaterial`] 两个引用，实现 §124 的 `Connector`，
//! 把 `invoke` 的参数转发给 `ConnectorImpl::invoke_with` 并附上材料——**§124 的签名
//! 一个字不改**。
//!
//! **它只在入口里构造，不开放为公开 API**（`pub(crate)`）：没有别的消费者，而公开它
//! 等于给材料多一个入口。
//!
//! # 轮换的可观察范围与它的限度（设计 §4.4、§11 第 7 条）
//!
//! [`material`](continuum_secrets::SecretsRuntime::material) 判的三件事——这张凭据是
//! **本运行时**签出的、签出它的轮换代**未被取代**、**未到期**——**只在取料那一刻**发生。
//! 材料一旦取出并交给实现，**后续的 `rotate` 不影响这次调用**：本阶段没有「用后即焚」
//! 或「调用中途复查」的机构。故「轮换使旧凭据失效」在本子项目的可观察范围是
//! **「下一次取料」**，不是「正在进行中的那一次调用」。
//!
//! **这一段没有照片，也写不出照片**：一次调用之内**没有可以插入 `rotate` 的位置**
//! （它要 `&mut SecretsRuntime`，在入口之外），故「材料已交出后 `rotate` 再发生」这个
//! 状态在入口上构造不出来（设计 §6.1 的 B3 段、§9 的「轮换」行、「遗留」第 5、18 条）。
//! 按本项目纪律 2，此处**明写它为什么没有照片**，不只在报告里说。

use async_trait::async_trait;
use continuum_core::connector::{ConnectorDescriptor, ConnectorOp};
use continuum_core::ProviderError;
use continuum_provider::Connector;
use continuum_secrets::SecretMaterial;
use serde_json::Value;

use crate::registry::ConnectorImpl;

/// 一次调用期间存活的适配器：材料**带外**到达实现，绝不进 `input`。
///
/// `input` 是 `serde_json::Value`，实现可以把它原样回显到返回值，而返回值会流向
/// Artifact / Journal / 审计——那正是 §51 要拦的（设计 §4.5 的否决理由）。要拦回来就得
/// 在出口扫材料，那是 §1.2 点名的最易失效位置，本设计明确否决。
pub(crate) struct Adapter<'a> {
    inner: &'a dyn ConnectorImpl,
    material: &'a SecretMaterial,
}

impl<'a> Adapter<'a> {
    pub(crate) fn new(inner: &'a dyn ConnectorImpl, material: &'a SecretMaterial) -> Self {
        Self { inner, material }
    }
}

#[async_trait]
impl Connector for Adapter<'_> {
    fn descriptor(&self) -> ConnectorDescriptor {
        self.inner.descriptor()
    }

    async fn invoke(&self, op: &ConnectorOp, input: Value) -> Result<Value, ProviderError> {
        // 材料走**带外参数**，不并入 `input`——本行就是上面那条否决理由的落点。
        self.inner.invoke_with(op, input, self.material).await
    }
}
