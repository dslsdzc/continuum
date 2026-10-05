//! 注册期四条核对的用例（设计 §3.2.1、§9）：双向覆盖、一一、服务半边相符，
//! 外加设计 §3.4 的两张照片。
//!
//! 每条用例**只触发一条核对**：构造时避开会同时触发另一条的情形，
//! 故注册期核对的**先后次序在用例上不可观察**——次序由实现定死，不为它写用例。

use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use continuum_capability::{CapabilityKind, EmailAction, FsAction, GitAction, GithubAction};
use continuum_connector::{ConnectorError, ConnectorImpl, ConnectorRegistry, OpBinding};
use continuum_core::connector::{ConnectorDescriptor, ConnectorId, ConnectorOp};
use continuum_core::ProviderError;
use continuum_secrets::{EnvCredentialSource, SecretMaterial, SecretsRuntime};
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

/// 注册表要一枚凭据运行时才构造得出来（入口持有它，设计 §4.1）。注册期用例不碰凭据
/// 路径，故给一个构造期不读环境、不做 I/O 的环境变量源即可。
fn secrets() -> SecretsRuntime {
    SecretsRuntime::new(Box::new(EnvCredentialSource::new("CONTINUUM_TEST_", 60_000)))
}

fn register(fake: FakeConnector) -> Result<(), ConnectorError> {
    ConnectorRegistry::new(secrets()).register(Box::new(fake))
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

/// **「一一」那条核对的另一侧**：声明并绑定**两条操作、两枚不同 kind** → `Ok(())`。
///
/// 加上这条之前，全仓**没有任何注册成功的连接器声明 ≥2 条操作**，故
/// `two_operations_bound_to_the_same_kind_are_rejected` 只钉住「拒」的那一侧：
/// 把那处核对改成「≥2 条绑定即拒」时，那条拒的用例与本文件里只声明 1 条操作的
/// `a_fully_bound_connector_registers` **都仍绿**。**本用例钉的是「只在该拒时拒」**，
/// 与设计 §9 对服务半边要求的正例（`a_positive_arm_with_the_matching_service_half_registers`）同形。
#[test]
fn two_operations_bound_to_two_kinds_register() {
    let fake = FakeConnector::new(
        "GitHub",
        &["GitHub.read_repo", "GitHub.push_branch"],
        &[
            ("GitHub.read_repo", CapabilityKind::Git(GitAction::Read)),
            ("GitHub.push_branch", CapabilityKind::Git(GitAction::Push)),
        ],
    );

    assert_eq!(
        register(fake),
        Ok(()),
        "两条操作各绑一枚不同 kind、服务半边都对得上时应注册成功"
    );
}

/// 两条操作绑同一枚 kind → `DuplicateKindBinding`（设计 §3.2 第 3 条，「一一」）。
///
/// 「一一」的理由是 §125 的意图——「`GitHub.merge` 能单独不授」：两条操作绑同一枚
/// kind 会让授权其一即授权另一（设计 §3.3）。
#[test]
fn two_operations_bound_to_the_same_kind_are_rejected() {
    let fake = FakeConnector::new(
        "Email",
        &["Email.send", "Email.draft"],
        &[
            ("Email.send", CapabilityKind::Email(EmailAction::Send)),
            ("Email.draft", CapabilityKind::Email(EmailAction::Send)),
        ],
    );

    match register(fake) {
        Err(ConnectorError::DuplicateKindBinding { connector, kind }) => {
            assert_eq!(connector.as_str(), "Email", "connector 应是本连接器的 id");
            assert_eq!(
                kind,
                CapabilityKind::Email(EmailAction::Send),
                "kind 应是被两条操作共绑的那一枚"
            );
        }
        other => panic!("期望 DuplicateKindBinding，实得 {other:?}"),
    }
}

/// 操作的服务半边不是本连接器的 id → `OperationServiceMismatch`（设计 §3.2 第 5 条）。
///
/// 这类操作**永不可达**（入口第 1 步按服务半边解析，永远解析不到它），故挡在注册期。
#[test]
fn an_operation_whose_service_half_is_not_this_connector_is_rejected() {
    let fake = FakeConnector::new(
        "GitHub",
        &["Email.send"],
        &[("Email.send", CapabilityKind::Email(EmailAction::Send))],
    );

    match register(fake) {
        Err(ConnectorError::OperationServiceMismatch { connector, op }) => {
            assert_eq!(connector.as_str(), "GitHub", "connector 应是本连接器的 id");
            assert_eq!(op.as_str(), "Email.send", "op 应是服务半边对不上的那一条");
        }
        other => panic!("期望 OperationServiceMismatch{{ op: Email.send }}，实得 {other:?}"),
    }
}

/// 服务半边**逐字比较、不折叠大小写**（设计 §3.2.1 末段）。
///
/// 被否掉的替代正是**折叠大小写**：折叠只到「modulo ASCII 大小写」，会让 `github` 与
/// `GitHub` 被判为同一个服务。本用例的夹具就是把 id 写成 `GitHub`、操作服务半边写成
/// `github`——**这是本判据唯一的守卫**，夹具若退化（两串大小写一致）它就恒绿地骗人。
#[test]
fn the_service_half_is_compared_case_sensitively() {
    let fake = FakeConnector::new(
        "GitHub",
        &["github.push_branch"],
        &[("github.push_branch", CapabilityKind::Git(GitAction::Push))],
    );

    match register(fake) {
        Err(ConnectorError::OperationServiceMismatch { connector, op }) => {
            assert_eq!(connector.as_str(), "GitHub", "connector 应是本连接器的 id");
            assert_eq!(
                op.as_str(),
                "github.push_branch",
                "op 应是大小写不同的那一条"
            );
        }
        other => panic!(
            "期望 OperationServiceMismatch（逐字比较：github 与 GitHub 是两个服务），实得 {other:?}"
        ),
    }
}

/// 服务半边与 id 逐字相等 → `Ok(())`。**这条同时是设计 §3.4 照片 1 的一半**
/// （另一半「一次成功调用」由 Task 4 补）。
///
/// 注意这条缝：`GitHub.push_branch` 这个**操作**绑的 kind 是 `Git(Push)`——
/// **resource 是 `git` 不是 `github`**（设计 §11 第 12 条）。**这不是笔误，别把它
/// 「修」成 `Github`**：§88 规定 `git.push` 是一枚能力，驱动为 `--effect push_branch`
/// 铸的正是它，连接器若不绑它，`AuthorizedEffect` 就配不上。
#[test]
fn a_positive_arm_with_the_matching_service_half_registers() {
    let fake = FakeConnector::new(
        "GitHub",
        &["GitHub.push_branch"],
        &[("GitHub.push_branch", CapabilityKind::Git(GitAction::Push))],
    );

    assert_eq!(
        register(fake),
        Ok(()),
        "服务半边逐字相等且绑定一一时应注册成功"
    );
}

/// **设计 §3.4 照片 2——本设计初稿写反的那一侧**：绑错了 kind **照样注册成功**。
///
/// **规范没有「一条操作该绑哪一枚 kind」的判据**，故这里断言的是 **`Ok`**，不是 `Err`。
/// 注册期的四条核对没有一条查这个（设计 §3.4、§11 第 16 条）：作者把 `GitHub.merge`
/// 绑到 `Github(CreatePr)` 上，核对全过、注册成功，而没有任何东西会红。
/// **没有这张照片，本节就是在替规范声称一条它没有的判据。**
#[test]
fn a_mis_bound_operation_still_registers() {
    let fake = FakeConnector::new(
        "GitHub",
        &["GitHub.merge"],
        &[(
            "GitHub.merge",
            CapabilityKind::Github(GithubAction::CreatePr),
        )],
    );

    assert_eq!(
        register(fake),
        Ok(()),
        "「配得对不对」无人判：绑错了也注册成功（设计 §3.4 照片 2）"
    );
}

