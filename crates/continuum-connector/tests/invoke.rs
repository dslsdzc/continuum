//! 入口（效应臂）的用例：四步核对里的第 1、2、4 步、凭据的逐次签发与取料，
//! 以及**逐次调用的适配器**（设计 §5.3、§4.1、§4.5）。
//!
//! **本 task 只做效应臂**：入口收 `AuthorizedEffect`，不收裸能力。两臂的
//! `ConnectorAuthorization` 由 Task 5 加上（那一条计划的 Step 2 要改本文件的出示值构造）。
//!
//! 夹具是两个连接器注册进同一个注册表：服务半边必须与连接器自己的 id 相等
//! （Task 3 的核对），故两个操作分属两个服务时**不能塞进同一个连接器**。
//!
//! 这里**不写 `EffectType::PushBranch` 这种名**：`continuum-effect` 的依赖边按计划
//! 由 Task 5 登记（那时入口第 3 步的 `EffectAuthorizationRequired` 才需要那个类型名）。
//! 本 task 的用例只需一个满足 `AuthorizedEffect::new` 的效应值，从被出示的 kind 经
//! [`CapabilityKind::effect`] 推出即可——**那也正是入口核对用的同一条对应**。

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use continuum_capability::{
    AuthorizedEffect, CapabilityError, CapabilityKind, EmailAction, GitAction, PaymentAction,
    mint,
};
use continuum_connector::{ConnectorError, ConnectorImpl, ConnectorRegistry, OpBinding};
use continuum_core::connector::{ConnectorDescriptor, ConnectorId, ConnectorOp};
use continuum_core::ProviderError;
use continuum_secrets::{
    FileCredentialSource, SecretMaterial, SecretsError, SecretsRuntime,
};
use serde_json::{Value, json};

/// 所有用例共用的「现在」。凭据源里的到期时刻都取得比它远，免得源那一侧先把凭据截短。
const NOW: i64 = 1_000_000;

/// 假实现记录的两样东西：`invoke_with` 被调了几次、最近一次收到的材料字节。
///
/// 材料字节是「凭据取自哪一条作用域」在实现这一侧**唯一可观察**的东西——凭据本身
/// 不出入口（它只在入口内构造、随即析构）。
#[derive(Default)]
struct Record {
    invocations: AtomicUsize,
    material: Mutex<Option<Vec<u8>>>,
}

/// 假实现返回值的形状。
enum Reply {
    /// 返回一个固定值。
    Fixed(Value),
    /// 把**收到的 `input`** 原样回显——用来照「材料从不进 `input`」。
    EchoInput,
    /// 把材料字节主动写进返回值——**对照臂**：说明 B 不拦这一步（设计 §4.5）。
    LeakMaterial,
    /// 返回这个后端错误（内层变体逐字段断言用）。
    Fail(ProviderError),
}

struct FakeConnector {
    id: &'static str,
    declared: Vec<&'static str>,
    bindings: Vec<(&'static str, CapabilityKind)>,
    reply: Reply,
    record: Arc<Record>,
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
        input: Value,
        material: &SecretMaterial,
    ) -> Result<Value, ProviderError> {
        self.record.invocations.fetch_add(1, Ordering::SeqCst);
        *self.record.material.lock().unwrap() = Some(material.expose().to_vec());
        match &self.reply {
            Reply::Fixed(value) => Ok(value.clone()),
            Reply::EchoInput => Ok(input),
            Reply::LeakMaterial => Ok(json!({
                "leaked": String::from_utf8_lossy(material.expose()),
            })),
            Reply::Fail(error) => Err(error.clone()),
        }
    }
}

/// 用例夹具：一个注册表 + 它的两个假连接器各自的记录。
///
/// `GitHub` 那条绑的是 `Git(Push)`——**resource 是 `git` 不是 `github`，这不是错**
/// （设计 §11 第 12 条）：§88 规定 `git.push` 是一枚能力，驱动为 `--effect push_branch`
/// 铸的正是它；连接器若不绑它，`AuthorizedEffect` 就配不上。
struct Fixture {
    /// 临时目录保活：凭据源文件在里面。
    _dir: tempfile::TempDir,
    registry: ConnectorRegistry,
    github: Arc<Record>,
    email: Arc<Record>,
}

/// 一份只登记给定条目的**文件**凭据源（作用域逐字匹配）。`entries` 为空即一份空源。
fn file_runtime(dir: &Path, entries: &[(&str, &str, i64)]) -> SecretsRuntime {
    let path = dir.join("credentials");
    let mut text = String::new();
    for (scope, material, expiry) in entries {
        text.push_str(&format!("{scope}\t{material}\t{expiry}\n"));
    }
    std::fs::write(&path, text).expect("写临时凭据源");
    SecretsRuntime::new(Box::new(
        FileCredentialSource::open(path).expect("凭据源格式合法"),
    ))
}

fn fixture(entries: &[(&str, &str, i64)], github_reply: Reply) -> Fixture {
    let dir = tempfile::tempdir().expect("临时目录");
    let mut registry = ConnectorRegistry::new(file_runtime(dir.path(), entries));

    let github = Arc::new(Record::default());
    registry
        .register(Box::new(FakeConnector {
            id: "GitHub",
            declared: vec!["GitHub.push_branch"],
            bindings: vec![("GitHub.push_branch", CapabilityKind::Git(GitAction::Push))],
            reply: github_reply,
            record: Arc::clone(&github),
        }))
        .expect("GitHub 声明与绑定逐项对齐且服务半边相符");

    // Email 只用在拒绝面：它从不被成功调用，故返回值取一个不管用的固定值。
    let email = Arc::new(Record::default());
    registry
        .register(Box::new(FakeConnector {
            id: "Email",
            declared: vec!["Email.send"],
            bindings: vec![("Email.send", CapabilityKind::Email(EmailAction::Send))],
            reply: Reply::Fixed(Value::Null),
            record: Arc::clone(&email),
        }))
        .expect("Email 同上");

    Fixture {
        _dir: dir,
        registry,
        github,
        email,
    }
}

/// 把一枚能力包成「它的 kind 所对应的那条效应」的 `AuthorizedEffect`。
///
/// `AuthorizedEffect::new` 会核对 `capability.kind() == for_effect(effect)`，故能包进去的
/// kind 必在 `for_effect` 的像里；`.expect` 是对用例夹具的断言（用例只出示像里的 kind）。
fn authorized_effect(kind: CapabilityKind, scope: &str, expiry: i64) -> AuthorizedEffect {
    let capability = mint(kind, scope.to_owned(), expiry).expect("用例里的作用域恒非空");
    let effect = kind
        .effect()
        .expect("用例只出示落在 for_effect 像里的 kind");
    AuthorizedEffect::new(effect, capability).expect("kind 与效应按定义相符")
}

fn invocations(record: &Record) -> usize {
    record.invocations.load(Ordering::SeqCst)
}

// ── 第一组：拒绝面（第 1、2、4 步）────────────────────────────────────────

/// 第 1 步：请求一个没有注册的服务半边 → `UnknownConnector`，且两个实现都未被调用。
#[tokio::test]
async fn an_unregistered_connector_is_rejected() {
    let fx = fixture(&[], Reply::Fixed(Value::Null));
    let authorized = authorized_effect(CapabilityKind::Git(GitAction::Push), "repo/X", NOW + 100);

    match fx
        .registry
        .invoke(&authorized, &op("Slack.post"), json!({}), NOW)
        .await
    {
        Err(ConnectorError::UnknownConnector { connector }) => {
            assert_eq!(connector.as_str(), "Slack", "connector 应是服务半边");
        }
        other => panic!("期望 UnknownConnector{{ connector: Slack }}，实得 {other:?}"),
    }
    assert_eq!(invocations(&fx.github), 0, "拒时实现未被调用");
    assert_eq!(invocations(&fx.email), 0, "拒时实现未被调用");
}

/// 第 2 步：`Email` 只声明 `Email.send`，请求 `Email.draft` → `UndeclaredOperation`。
#[tokio::test]
async fn an_undeclared_operation_is_rejected() {
    let fx = fixture(&[], Reply::Fixed(Value::Null));
    // 出示的 kind **恰好等于** `Email.send` 的绑定，故第 4 步拦不住——红只可能红在第 2 步。
    let authorized = authorized_effect(CapabilityKind::Email(EmailAction::Send), "repo/X", NOW + 100);

    match fx
        .registry
        .invoke(&authorized, &op("Email.draft"), json!({}), NOW)
        .await
    {
        Err(ConnectorError::UndeclaredOperation { connector, op }) => {
            assert_eq!(connector.as_str(), "Email", "connector 应是被解析到的服务");
            assert_eq!(op.as_str(), "Email.draft", "op 应是没声明的那一条");
        }
        other => panic!("期望 UndeclaredOperation{{ op: Email.draft }}，实得 {other:?}"),
    }
    assert_eq!(invocations(&fx.github), 0, "拒时实现未被调用");
    assert_eq!(invocations(&fx.email), 0, "拒时实现未被调用");
}

/// 第 4 步：出示另一条**真在像里**的效应包出的 `AuthorizedEffect` → `AuthorizationMismatch`。
///
/// 出示值必须是一枚真包得进 `AuthorizedEffect` 的能力（`AuthorizedEffect::new` 核对
/// `capability.kind() == for_effect(effect)`），而 `for_effect` 的像里**没有**
/// `Filesystem(Read)`——故「出示一枚 `Filesystem(Read)` 的能力」那种写法构造不出来。
/// 本例取 `Charge` → `Payment(PaymentAction::Charge)`：在像里、且与 `Email(Send)` 不等。
#[tokio::test]
async fn a_presented_capability_of_another_kind_is_rejected() {
    let fx = fixture(&[], Reply::Fixed(Value::Null));
    let authorized = authorized_effect(
        CapabilityKind::Payment(PaymentAction::Charge),
        "repo/X",
        NOW + 100,
    );

    match fx
        .registry
        .invoke(&authorized, &op("Email.send"), json!({}), NOW)
        .await
    {
        Err(ConnectorError::AuthorizationMismatch {
            op,
            bound,
            presented,
        }) => {
            assert_eq!(op.as_str(), "Email.send", "op 应是被请求的那一条");
            assert_eq!(
                bound,
                CapabilityKind::Email(EmailAction::Send),
                "bound 应是该操作的绑定"
            );
            assert_eq!(
                presented,
                CapabilityKind::Payment(PaymentAction::Charge),
                "presented 应是出示能力给出的那一枚"
            );
        }
        other => panic!("期望 AuthorizationMismatch，实得 {other:?}"),
    }
    assert_eq!(invocations(&fx.github), 0, "拒时实现未被调用");
    assert_eq!(invocations(&fx.email), 0, "拒时实现未被调用");
}

// ── 第二组：成功路径与凭据 ──────────────────────────────────────────────

/// **对照臂**：四步全过 → 实现被调用**恰一次**，返回值就是实现给出的那个 `Value`。
///
/// 这条同时补上设计 §3.4 照片 1 的「一次成功调用」那一半（Task 3 只落了注册那一半）。
#[tokio::test]
async fn a_push_branch_operation_reaches_the_implementation() {
    let reply = json!({"ok": true, "ref": "refs/heads/main"});
    let fx = fixture(
        &[("repo/X", "SCOPE-X", NOW + 10_000)],
        Reply::Fixed(reply.clone()),
    );
    let authorized = authorized_effect(CapabilityKind::Git(GitAction::Push), "repo/X", NOW + 1_000);

    let returned = fx
        .registry
        .invoke(
            &authorized,
            &op("GitHub.push_branch"),
            json!({"ref": "refs/heads/main"}),
            NOW,
        )
        .await
        .expect("四步全过且凭据取得到材料");

    assert_eq!(returned, reply, "返回值就是实现给出的那个 Value");
    assert_eq!(invocations(&fx.github), 1, "实现被调用恰一次");
    assert_eq!(invocations(&fx.email), 0, "另一个连接器未被牵动");
}

/// 凭据的作用域来自**出示的那枚能力**：源里只登记 `repo/X`，出示能力的作用域是 `repo/X`
/// → 实现收到的材料取自 `repo/X`（即源里那一条的字节）。
#[tokio::test]
async fn the_credential_carries_the_scope_of_the_presented_capability() {
    let fx = fixture(
        &[("repo/X", "SCOPE-X", NOW + 10_000)],
        Reply::Fixed(Value::Null),
    );
    let authorized = authorized_effect(CapabilityKind::Git(GitAction::Push), "repo/X", NOW + 1_000);

    fx.registry
        .invoke(&authorized, &op("GitHub.push_branch"), json!({}), NOW)
        .await
        .expect("源覆盖 repo/X，取得到材料");

    let received = fx.github.material.lock().unwrap().clone();
    assert_eq!(
        received.as_deref(),
        Some(&b"SCOPE-X"[..]),
        "材料应是源里 repo/X 那一条的字节"
    );
}

/// **对照臂**：把出示能力的作用域改成源不覆盖的 `repo/Y` → `Credentials(ScopeNotCovered)`。
///
/// 断言的是**这一种**内层变体（纪律 3），不是笼统的 `Err`。
#[tokio::test]
async fn a_scope_the_source_does_not_cover_is_rejected() {
    let fx = fixture(
        &[("repo/X", "SCOPE-X", NOW + 10_000)],
        Reply::Fixed(Value::Null),
    );
    let authorized = authorized_effect(CapabilityKind::Git(GitAction::Push), "repo/Y", NOW + 1_000);

    match fx
        .registry
        .invoke(&authorized, &op("GitHub.push_branch"), json!({}), NOW)
        .await
    {
        Err(ConnectorError::Credentials(SecretsError::ScopeNotCovered { scope, .. })) => {
            assert_eq!(scope, "repo/Y", "报的应是那个没被覆盖的作用域");
        }
        other => panic!("期望 Credentials(ScopeNotCovered)，实得 {other:?}"),
    }
    assert_eq!(invocations(&fx.github), 0, "凭据取不到时实现未被调用");
}

/// 已失效的能力到不了实现：`issue` 走 `Capability::is_valid_at` 并把它转出来。
///
/// `AuthorizedEffect::new` **不读时钟**，故一枚已过期的能力照样包得进去——时钟比较
/// 发生在入口的凭据路径上（设计 §4.3）。
#[tokio::test]
async fn an_expired_capability_cannot_reach_the_implementation() {
    let fx = fixture(
        &[("repo/X", "SCOPE-X", NOW + 10_000)],
        Reply::Fixed(Value::Null),
    );
    // `expiry == NOW`：`now < expiry` 不成立，即已失效（`expiry` 是失效时刻）。
    let authorized = authorized_effect(CapabilityKind::Git(GitAction::Push), "repo/X", NOW);

    match fx
        .registry
        .invoke(&authorized, &op("GitHub.push_branch"), json!({}), NOW)
        .await
    {
        Err(ConnectorError::Credentials(SecretsError::Capability(CapabilityError::Expired {
            expiry,
            now,
        }))) => {
            assert_eq!(expiry, NOW, "报的应是能力自己的到期时刻");
            assert_eq!(now, NOW, "报的应是入口收到的那个 now");
        }
        other => panic!("期望 Credentials(Capability(Expired))，实得 {other:?}"),
    }
    assert_eq!(invocations(&fx.github), 0, "能力已失效时实现未被调用");
}

/// **对照臂**：到期前一毫秒照常到达（边界的那一格）。
#[tokio::test]
async fn a_capability_valid_one_millisecond_before_its_expiry_reaches_the_implementation() {
    let fx = fixture(
        &[("repo/X", "SCOPE-X", NOW + 10_000)],
        Reply::Fixed(Value::Null),
    );
    let authorized = authorized_effect(CapabilityKind::Git(GitAction::Push), "repo/X", NOW);

    fx.registry
        .invoke(&authorized, &op("GitHub.push_branch"), json!({}), NOW - 1)
        .await
        .expect("到期前一毫秒应照常到达");

    assert_eq!(invocations(&fx.github), 1, "边界那一格应到达实现");
}

/// 材料**不进 `input`**：实现把收到的 `input` 原样回显，返回值里也没有材料。
#[tokio::test]
async fn the_material_never_leaves_through_the_return_value() {
    let material = "SCOPE-X-SECRET";
    let fx = fixture(&[("repo/X", material, NOW + 10_000)], Reply::EchoInput);
    let authorized = authorized_effect(CapabilityKind::Git(GitAction::Push), "repo/X", NOW + 1_000);
    let input = json!({"ref": "refs/heads/main"});

    let returned = fx
        .registry
        .invoke(&authorized, &op("GitHub.push_branch"), input.clone(), NOW)
        .await
        .expect("四步全过且凭据取得到材料");

    assert_eq!(returned, input, "实现回显了它收到的 input");
    let rendered = returned.to_string();
    assert!(
        !rendered.contains(material),
        "材料从不进 input（材料走 invoke_with 的带外参数），故回显 input 的返回值里也没有它：{rendered}"
    );
}

/// **对照臂**：假实现**主动**把 `material.expose()` 写进返回值 → 材料**出现在返回值里**。
///
/// **B 不拦这一步**：设计 §4.5 明确否决了「在出口扫材料」那条手工校验层（§1.2 点名的
/// 最易失效位置）。**没有这条对照臂**，上面那条在「适配器把材料塞进了 `input` 而实现
/// 没回显」时也会绿。
#[tokio::test]
async fn the_control_arm_the_material_does_leave_when_the_implementation_leaks_it() {
    let material = "SCOPE-X-SECRET";
    let fx = fixture(&[("repo/X", material, NOW + 10_000)], Reply::LeakMaterial);
    let authorized = authorized_effect(CapabilityKind::Git(GitAction::Push), "repo/X", NOW + 1_000);

    let returned = fx
        .registry
        .invoke(&authorized, &op("GitHub.push_branch"), json!({}), NOW)
        .await
        .expect("四步全过且凭据取得到材料");

    let rendered = returned.to_string();
    assert!(
        rendered.contains(material),
        "实现主动取用材料时它确实出现在返回值里——B 不拦这一步：{rendered}"
    );
}

/// 后端错误原样带出来：内层变体**逐字段**断言（纪律 3），不只断言 `Err`。
///
/// `Provider` 独立于 `Credentials`：后端不可用与凭据拿不到是两回事（设计 §6.1）。
#[tokio::test]
async fn a_backend_error_comes_back_as_provider() {
    let fx = fixture(
        &[("repo/X", "SCOPE-X", NOW + 10_000)],
        Reply::Fail(ProviderError::Unavailable("后端挂了".to_owned())),
    );
    let authorized = authorized_effect(CapabilityKind::Git(GitAction::Push), "repo/X", NOW + 1_000);

    match fx
        .registry
        .invoke(&authorized, &op("GitHub.push_branch"), json!({}), NOW)
        .await
    {
        Err(ConnectorError::Provider(ProviderError::Unavailable(message))) => {
            assert_eq!(message, "后端挂了", "内层变体的字段应逐字保留");
        }
        other => panic!("期望 Provider(Unavailable)，实得 {other:?}"),
    }
    assert_eq!(invocations(&fx.github), 1, "后端错误发生在实现被调用之后");
}
