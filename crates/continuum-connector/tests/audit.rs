//! 审计的**否定**照片与它的对照臂（设计 §6.2、决定 B-5）。
//!
//! 与 `tests/invoke.rs` **分开**：本文件要一个真数据库，其余用例不要。
//!
//! **这条照片的强度要写准**：B 的入口**没有任何数据库句柄**（`ConnectorRegistry` 不持有
//! `Tx`、`Db`），故行数不变是**结构性的**，不是「本可以用另一条路径写而这次没写」。
//! 它的价值在于把「B 不重记 §313 的两项」这条决定 B-5 钉成一个可执行判据。

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use continuum_capability::{
    AuthorizedEffect, Capability, CapabilityKind, GitAction, Tool, ToolId, ToolProfile, Trust,
    authorize, mint, p3_capability_migrations, save_tool,
};
use continuum_connector::{ConnectorAuthorization, ConnectorImpl, ConnectorRegistry, OpBinding};
use continuum_core::connector::{ConnectorDescriptor, ConnectorId, ConnectorOp};
use continuum_core::ProviderError;
use continuum_persist::{Db, Migration, Tx, Value, builtin_migrations};
use continuum_secrets::{FileCredentialSource, SecretMaterial, SecretsRuntime};
use serde_json::{Value as JsonValue, json};

const NOW: i64 = 1_000_000;

fn op(name: &str) -> ConnectorOp {
    ConnectorOp::new(name).expect("用例里的操作串合法（非空且含 `.`）")
}

// ── 库这一侧 ────────────────────────────────────────────────────────────

/// 起一个临时库：内置迁移（`audit_log` 表来自那里）＋调用方另给的迁移。
fn db(extra: Vec<Migration>) -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("audit.db");
    let mut migrations = builtin_migrations();
    migrations.extend(extra);
    let db = Db::open_with(&path, migrations).unwrap();
    db.migrate().unwrap();
    (dir, db)
}

fn int_of(v: &Value) -> i64 {
    match v {
        Value::Int(i) => *i,
        other => panic!("列应为整数，实际 {other:?}"),
    }
}

fn audit_count(tx: &Tx<'_>) -> i64 {
    let rows = tx.query("SELECT COUNT(*) FROM audit_log", &[]).unwrap();
    int_of(&rows[0][0])
}

/// 一条只关心 `required_capabilities` 的登记项（照 `continuum-capability` 的
/// `tests/authorize.rs` 的 `profile()` 写）。
fn profile(id: &str, required: Vec<CapabilityKind>) -> ToolProfile {
    ToolProfile::new(
        Tool::new(
            ToolId::new(id),
            "1".to_owned(),
            json!({}),
            json!({}),
            required,
            None,
            true,
        ),
        None,
        None,
        Trust,
    )
}

// ── 连接器这一侧 ────────────────────────────────────────────────────────

struct NoopConnector {
    invocations: Arc<AtomicUsize>,
}

#[async_trait]
impl ConnectorImpl for NoopConnector {
    fn descriptor(&self) -> ConnectorDescriptor {
        ConnectorDescriptor::new(ConnectorId::new("GitHub"), vec![op("GitHub.push_branch")])
            .expect("声明集非空")
    }

    fn bindings(&self) -> Vec<OpBinding> {
        vec![OpBinding::new(
            op("GitHub.push_branch"),
            CapabilityKind::Git(GitAction::Push),
        )]
    }

    async fn invoke_with(
        &self,
        _op: &ConnectorOp,
        _input: JsonValue,
        _material: &SecretMaterial,
    ) -> Result<JsonValue, ProviderError> {
        self.invocations.fetch_add(1, Ordering::SeqCst);
        Ok(JsonValue::Null)
    }
}

/// 一个只登记 `GitHub.push_branch` 的注册表，凭据源只覆盖 `repo/X`。
fn connector_registry(dir: &Path) -> (ConnectorRegistry, Arc<AtomicUsize>) {
    let path = dir.join("credentials");
    std::fs::write(&path, format!("repo/X\tSCOPE-X\t{}\n", NOW + 10_000)).unwrap();
    let runtime = SecretsRuntime::new(Box::new(FileCredentialSource::open(path).unwrap()));

    let invocations = Arc::new(AtomicUsize::new(0));
    let mut registry = ConnectorRegistry::new(runtime);
    registry
        .register(Box::new(NoopConnector {
            invocations: Arc::clone(&invocations),
        }))
        .expect("声明与绑定逐项对齐且服务半边相符");
    (registry, invocations)
}

fn authorized_effect(kind: CapabilityKind, scope: &str, expiry: i64) -> AuthorizedEffect {
    let capability = mint(kind, scope.to_owned(), expiry).expect("用例里的作用域恒非空");
    let effect = kind.effect().expect("用例只出示像里的 kind");
    AuthorizedEffect::new(effect, capability).expect("kind 与效应按定义相符")
}

/// **否定照片**：一次**成功**的连接器调用前后，`audit_log` 行数不变（决定 B-5）。
#[tokio::test]
async fn a_successful_connector_call_does_not_write_an_audit_row() {
    let (_db_dir, db) = db(vec![]);
    let source_dir = tempfile::tempdir().unwrap();
    let (registry, invocations) = connector_registry(source_dir.path());
    let authorized = authorized_effect(CapabilityKind::Git(GitAction::Push), "repo/X", NOW + 1_000);

    let tx = db.begin().unwrap();
    let before = audit_count(&tx);

    let returned = registry
        .invoke(
            &ConnectorAuthorization::Effect(authorized),
            &op("GitHub.push_branch"),
            json!({}),
            NOW,
        )
        .await
        .expect("四步全过且凭据取得到材料");

    // 先确认这次调用**真的到了实现**：否则「行数不变」可能只是调用没发生（空转的绿）。
    assert_eq!(returned, JsonValue::Null, "实现给出的返回值");
    assert_eq!(
        invocations.load(Ordering::SeqCst),
        1,
        "这次调用必须真的到达实现，否则行数不变是空转"
    );

    let after = audit_count(&tx);
    assert_eq!(
        after, before,
        "一次成功的连接器调用不写审计行——B 不重记 §313 的两项（决定 B-5）"
    );
}

/// **对照臂**：走一次会写审计的既有路径（`authorize` 的成功分支，写一条
/// `AuditKind::CapabilityGrants`），断言行数**恰加一**。
///
/// 库要多带 `p3_capability_migrations()`（`tool` 表）＋一条 `save_tool` 登记项＋与它
/// 相配的出示能力——少了这些，`authorize` 会报 `UnknownTool`（不写审计行），
/// **对照臂就以「行数没变」的方式绿了**，而它恰恰是来证「这里本来写得进一行」的。
#[tokio::test]
async fn the_control_arm_writes_exactly_one_audit_row() {
    let (_db_dir, db) = db(p3_capability_migrations());
    let tx = db.begin().unwrap();
    save_tool(
        &tx,
        &profile("t1", vec![CapabilityKind::Git(GitAction::Push)]),
    )
    .unwrap();

    let before = audit_count(&tx);
    let presented: [Capability; 1] = [mint(
        CapabilityKind::Git(GitAction::Push),
        "repo/X".to_owned(),
        NOW + 10,
    )
    .expect("作用域非空")];

    let authorized = authorize(&tx, &ToolId::new("t1"), &presented, NOW)
        .expect("登记项声明了这枚能力且它未失效");
    assert_eq!(authorized.tool_id(), &ToolId::new("t1"));

    let after = audit_count(&tx);
    assert_eq!(
        after,
        before + 1,
        "authorize 的成功分支恰写一条审计行——这里本来写得进一行"
    );

    tx.commit().unwrap();
}
