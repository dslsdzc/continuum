//! 工具侧**唯一**受门禁的调用入口：`ProviderRegistry::invoke_tool`（Task 4；设计 §7.1）。
//!
//! 四条用例各钉一条性质：
//! - `a_registered_tool_is_invoked_and_its_result_returned`：门禁内的正常路径（设计 §11），
//!   顺带是「输入被原样带到适配器」这条性质的照片（echo 适配器的 `output == 输入`）——
//!   Task 3 不再有构造该请求的集成用例，那条性质在这里观测（设计 §7.5）；
//! - `an_unregistered_tool_is_not_called_at_all`：未登记的 id **只**走路由，一个适配器都不碰；
//! - `the_adapter_failure_is_reported_as_provider`：适配器自己失败 → `Provider`，**不是** `Unregistered`；
//! - `the_adapter_is_handed_the_authorization_that_was_passed_in`：适配器收到的授权就是传进去的那一枚。
//!
//! **`db()` 照 `continuum-capability/tests/authorize.rs` 同形另写一份**：本仓既有做法是
//! 每个测试目标各有一份，不做跨目标的共享夹具（同 crate 的 `tests/persist.rs` 亦是如此）。
//!
//! **三条结果里的「工具跑起来了但自身失败 → `Ok(is_error: true)`」不在本文件**：那是 C 加在
//! 适配器上的**约定**，§316 里没有出处、也没有强制（设计 §7.1）；它的照片在 Task 7，
//! 且只钉夹具、不钉真实适配器。`invoke_tool` 的实现**不替适配器做这个分流**——
//! `Ok` / `Err` 一律按适配器给的转出去，本文件用 `RecorderOutcome::Fail` 钉的就是后一半。

mod common;

use common::{FakeTool, RecorderOutcome, RecordingTool};
use continuum_capability::{
    AuthorizedTool, CapabilityKind, Tool, ToolId, ToolProfile, Trust, authorize,
    p3_capability_migrations, save_tool,
};
use continuum_core::ProviderError;
use continuum_persist::{Db, Migration, builtin_migrations};
use continuum_provider::{ProviderRegistry, ToolCallError};
use serde_json::{Value, json};
use std::sync::Arc;

/// 起一个临时库：内置迁移（`audit_log` 表来自那里）＋ capability 的 `tool` 表迁移。
fn db() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("invoke.db");
    let mut migrations: Vec<Migration> = builtin_migrations();
    migrations.extend(p3_capability_migrations());
    let db = Db::open_with(&path, migrations).unwrap();
    db.migrate().unwrap();
    (dir, db)
}

/// 一条不需要任何能力的登记项：`required_capabilities` 为空，
/// 于是空出示集就能过 `authorize`——本文件的用例都不关心能力比对。
fn profile(id: &str) -> ToolProfile {
    ToolProfile::new(
        Tool::new(
            ToolId::new(id),
            "1".to_owned(),
            json!({"type": "object"}),
            json!({"type": "object"}),
            Vec::<CapabilityKind>::new(),
            None,
            true,
        ),
        None,
        None,
        Trust,
    )
}

/// 把工具登记进 `tool` 表，再经 `authorize` 取一枚**真的** `AuthorizedTool`——
/// 这是全仓唯一的取得路径，本文件也只用这一条（设计 §3.4）。
fn authorized(id: &str) -> (tempfile::TempDir, AuthorizedTool) {
    let (dir, db) = db();
    let tx = db.begin().unwrap();
    save_tool(&tx, &profile(id)).unwrap();
    let authorized = authorize(&tx, &ToolId::new(id), &[], 1_000).unwrap();
    tx.commit().unwrap();
    (dir, authorized)
}

/// 门禁内正常路径：登记了适配器 → 授权证明 + 输入 → 适配器跑通，结果原样交出。
///
/// `FakeTool` 的 `invoke` 回显输入，故 `output == 输入` 这一条**同时**是「输入被原样
/// 带到适配器、原样读出去」的照片（设计 §7.5）。
#[tokio::test]
async fn a_registered_tool_is_invoked_and_its_result_returned() {
    let (_dir, auth) = authorized("echo");
    let mut registry = ProviderRegistry::new();
    registry
        .register_tool(vec![ToolId::new("echo")], Arc::new(FakeTool))
        .unwrap();

    let result = registry
        .invoke_tool(&auth, json!({"x": 1}))
        .await
        .expect("已登记且授权在手，调用应当跑通");

    assert_eq!(result.output, json!({"x": 1}), "输入应原样到达适配器");
    assert!(!result.is_error, "跑通的调用不是工具级失败");
}

/// 未登记的 id：**只**走路由就断掉——返 `Unregistered`（配置缺陷），**不是** `Provider`。
///
/// 「一个适配器都没被碰过」这句断言在主语上：注册表里**有一个**服务于别的 id 的适配器，
/// 它一次都不该被问到。只数 `invoke` 不够——「先逐个问适配器再判未命中」那种实现碰的是
/// `list_tools`，故 `RecordingTool` 把 `list_tools` / `describe_tool` / `invoke` 三者
/// 的调用都记进 `touches()`（`cancel` 不计——没有用例会走到它）。
#[tokio::test]
async fn an_unregistered_tool_is_not_called_at_all() {
    let (_dir, auth) = authorized("t1");
    let (adapter, recorder) = RecordingTool::new(ToolId::new("echo"), RecorderOutcome::Echo);
    let mut registry = ProviderRegistry::new();
    registry
        .register_tool(vec![ToolId::new("echo")], Arc::new(adapter))
        .unwrap();

    let err = registry
        .invoke_tool(&auth, json!({"x": 1}))
        .await
        .expect_err("没有适配器服务这个 id");

    assert!(
        matches!(&err, ToolCallError::Unregistered { id } if id.as_str() == "t1"),
        "未命中应是 Unregistered {{ id: t1 }}，实际 {err:?}"
    );
    assert!(
        !matches!(err, ToolCallError::Provider(_)),
        "配置缺陷不得报成 provider 失败，实际 {err:?}"
    );
    assert_eq!(recorder.touches(), 0, "路由未命中时一个适配器都不该被碰");
    assert_eq!(recorder.seen_tool_id(), None, "更不该有适配器收到过授权");
}

/// 适配器自己失败（传输失败）→ `Provider`，**不是** `Unregistered`。
///
/// 反向的那一侧同样钉住：这一条要是被报成 `Unregistered`，调用方会把它当成配置缺陷
/// 而不重试（设计 §5.1），而它其实是一次可能瞬时的失败。
#[tokio::test]
async fn the_adapter_failure_is_reported_as_provider() {
    let (_dir, auth) = authorized("t1");
    let (adapter, recorder) = RecordingTool::new(ToolId::new("t1"), RecorderOutcome::Fail);
    let mut registry = ProviderRegistry::new();
    registry
        .register_tool(vec![ToolId::new("t1")], Arc::new(adapter))
        .unwrap();

    let err = registry
        .invoke_tool(&auth, json!({}))
        .await
        .expect_err("适配器返 Err");

    assert!(
        matches!(
            &err,
            ToolCallError::Provider(ProviderError::Transport(m)) if m == "适配器自己没跑成"
        ),
        "适配器的失败应原样经 Provider 透出，实际 {err:?}"
    );
    assert!(
        !matches!(err, ToolCallError::Unregistered { .. }),
        "已登记的 id 不得被报成 Unregistered，实际 {err:?}"
    );
    assert_eq!(
        recorder.seen_tool_id(),
        Some(ToolId::new("t1")),
        "证明这一条走的确实是适配器那条路，不是路由提前断掉"
    );
}

/// 适配器收到的授权**就是**传进去的那一枚——路由用的 id 与被下传的授权是同一个来源。
///
/// 结果里没有任何回执字段可比对（设计 §7.1 的限度：类型层管住「id 从哪来」，
/// 管不住「适配器照着它做」），故只能让适配器把收到的 `call.authorization().tool_id()`
/// 抄出来，与 `auth.tool_id()` 比。
#[tokio::test]
async fn the_adapter_is_handed_the_authorization_that_was_passed_in() {
    let (_dir, auth) = authorized("t1");
    let (adapter, recorder) = RecordingTool::new(ToolId::new("t1"), RecorderOutcome::Echo);
    let mut registry = ProviderRegistry::new();
    registry
        .register_tool(vec![ToolId::new("t1")], Arc::new(adapter))
        .unwrap();

    let result: Value = registry
        .invoke_tool(&auth, json!({"payload": true}))
        .await
        .expect("已登记且授权在手")
        .output;
    assert_eq!(result, json!({"payload": true}));

    assert_eq!(
        recorder.seen_tool_id().as_ref(),
        Some(auth.tool_id()),
        "适配器读到的授权 id 应与传进去的那一枚相同"
    );
}
