//! 注册期双向覆盖的用例（设计 §3.2.1 第 2 条、§9）。
//!
//! 每条用例**只触发一条核对**：构造时避开会同时触发另一条（或 Task 3 的两条）的情形，
//! 故注册期核对的**先后次序在用例上不可观察**——次序由实现定死，不为它写用例。

use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use continuum_capability::{CapabilityKind, EmailAction, FsAction};
use continuum_connector::{ConnectorError, ConnectorImpl, ConnectorRegistry, OpBinding};
use continuum_core::connector::{ConnectorDescriptor, ConnectorId, ConnectorOp};
use continuum_core::ProviderError;
use continuum_secrets::SecretMaterial;
use serde_json::Value;

/// 夹具：可声明任意描述符与任意绑定，并记录 `invoke_with` 的调用次数。
struct FakeConnector {
    id: &'static str,
    declared: Vec<&'static str>,
    bindings: Vec<(&'static str, CapabilityKind)>,
    /// `invoke_with` 的调用次数。**Task 4 起才用得上**（入口的逐次调用路径），本 task 先建。
    invocations: AtomicUsize,
}

impl FakeConnector {
    fn new(
        id: &'static str,
        declared: &[&'static str],
        bindings: &[(&'static str, CapabilityKind)],
    ) -> Self {
        Self {
            id,
            declared: declared.to_vec(),
            bindings: bindings.to_vec(),
            invocations: AtomicUsize::new(0),
        }
    }
}

fn op(name: &str) -> ConnectorOp {
    ConnectorOp::new(name).expect("用例里的操作串合法（非空且含 `.`）")
}

fn descriptor_of(id: &str, operations: &[&str]) -> ConnectorDescriptor {
    ConnectorDescriptor::new(
        ConnectorId::new(id),
        operations.iter().map(|name| op(name)).collect(),
    )
    .expect("用例里的声明集非空")
}

#[async_trait]
impl ConnectorImpl for FakeConnector {
    fn descriptor(&self) -> ConnectorDescriptor {
        descriptor_of(self.id, &self.declared)
    }

    fn bindings(&self) -> Vec<OpBinding> {
        self.bindings
            .iter()
            .map(|(name, kind)| OpBinding::new(op(name), *kind))
            .collect()
    }

    async fn invoke_with(
        &self,
        _op: &ConnectorOp,
        _input: Value,
        _material: &SecretMaterial,
    ) -> Result<Value, ProviderError> {
        self.invocations.fetch_add(1, Ordering::SeqCst);
        Ok(Value::Null)
    }
}

fn register(fake: FakeConnector) -> Result<(), ConnectorError> {
    ConnectorRegistry::new().register(Box::new(fake))
}

/// 声明集里有一条没绑 → `UnboundOperation`，且 `op` 是**没绑的那一条**（逐字比）。
#[test]
fn an_operation_that_is_declared_but_not_bound_is_rejected() {
    let fake = FakeConnector::new(
        "Email",
        &["Email.send", "Email.draft"],
        &[("Email.send", CapabilityKind::Email(EmailAction::Send))],
    );

    match register(fake) {
        Err(ConnectorError::UnboundOperation { connector, op }) => {
            assert_eq!(connector.as_str(), "Email", "connector 应是本连接器的 id");
            assert_eq!(op.as_str(), "Email.draft", "op 应是没绑的那一条");
        }
        other => panic!("期望 UnboundOperation{{ op: Email.draft }}，实得 {other:?}"),
    }
}

/// 绑定集里有一条没声明 → `UndeclaredBoundOperation`（双向覆盖的另一侧）。
#[test]
fn an_operation_that_is_bound_but_not_declared_is_rejected() {
    let fake = FakeConnector::new(
        "Email",
        &["Email.send"],
        &[
            ("Email.send", CapabilityKind::Email(EmailAction::Send)),
            ("Email.other", CapabilityKind::Filesystem(FsAction::Read)),
        ],
    );

    match register(fake) {
        Err(ConnectorError::UndeclaredBoundOperation { connector, op }) => {
            assert_eq!(connector.as_str(), "Email", "connector 应是本连接器的 id");
            assert_eq!(op.as_str(), "Email.other", "op 应是没声明的那一条");
        }
        other => panic!("期望 UndeclaredBoundOperation{{ op: Email.other }}，实得 {other:?}"),
    }
}

/// 对照臂：声明与绑定逐项对齐 → `Ok(())`。
#[test]
fn a_fully_bound_connector_registers() {
    let fake = FakeConnector::new(
        "Email",
        &["Email.send"],
        &[("Email.send", CapabilityKind::Email(EmailAction::Send))],
    );

    assert_eq!(register(fake), Ok(()), "声明与绑定逐项对齐时应注册成功");
}
