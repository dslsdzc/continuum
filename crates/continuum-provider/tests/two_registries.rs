//! 两个登记点不一致的两向（设计 §3.3 代价三）。
//!
//! 「工具存在两个登记点」：capability 的 `tool` 表（**治理**：要哪些能力、什么 effect_class）
//! 与 [`ProviderRegistry`]（**路由**：谁去跑）。两个方向的错法都可达、各有可观察形态，
//! 故本文件**逐向一条**——只写一向等于「守卫只钉一侧」，而缺的那侧正是 fail-open 的那侧
//! （第一向就是：没有适配器却被放行到调用）。两条是**同一条事实的两向**，必须同时存在。
//!
//! - `a_tool_in_the_table_without_an_adapter_passes_authorize_then_fails_to_route`（第一向）：
//!   表里有行、注册表里没有适配器 → `authorize` **通过**而路由**未命中**、报 `Unregistered`；
//! - `an_adapter_without_a_row_in_the_table_fails_authorize_before_any_call`（第二向）：
//!   注册表里有适配器、表里当时没有行 → `authorize` 在**任何调用之前**报 `UnknownTool`。
//!
//! **第一向不另配 `RecordingTool`**（协调者裁定，2026-10-06）：它是一条「序列 + 结果」的用例，
//! 钉的是「`authorize` 通过**而**路由未命中」这条序列，以及「未命中报的是 `Unregistered`
//! 这一种 `Err`」这个结果，**不承担路由机制的变异守卫**。路由机制（「先逐个问遍所有适配器、
//! 再判未命中」）那条性质的照片是 Task 4 `tests/invoke_tool.rs` 的
//! `an_unregistered_tool_is_not_called_at_all`（登记一个服务于**另一个 id** 的 `RecordingTool`
//! 并断言 `recorder.touches() == 0`）——本向的注册表里**一个适配器都没有**，两版问的都是零个，
//! 那条变异在这里退化成等价变异体。同一件事的第二个落点是本仓反对的形状，故不在此复制。
//! **第二向用 `RecordingTool` 是另一码事**：它观察的是「注册表在这一路径上被碰过几次」
//! 这条**不同**的性质。
//!
//! **`db()` / `profile()` 照 Task 4 `tests/invoke_tool.rs` 同形另写一份**：本仓既有做法是
//! 每个测试目标各有一份，不做跨目标的共享夹具。

mod common;

use common::{RecorderOutcome, RecordingTool};
use continuum_capability::{
    CapabilityError, Tool, ToolId, ToolProfile, Trust, authorize, p3_capability_migrations, save_tool,
};
use continuum_persist::{Db, Migration, builtin_migrations};
use continuum_provider::{ProviderRegistry, ToolCallError};
use serde_json::json;
use std::sync::Arc;

/// 起一个临时库：内置迁移（`audit_log` 表来自那里）＋ capability 的 `tool` 表迁移。
fn db() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("two_registries.db");
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
            Vec::<continuum_capability::CapabilityKind>::new(),
            None,
            true,
        ),
        None,
        None,
        Trust,
    )
}

/// **第一向（fail-open 的那侧）**：`tool` 表登记了、`ProviderRegistry` **没**登记。
///
/// 两半都要断言：只断后一半会让「`authorize` 也能挡住它」这条假说法活下来——`authorize`
/// 只看治理侧的 `tool` 表，看不到路由侧有没有人去跑，故它对这一向**必然放行**；
/// 挡在调用门外的是路由的未命中，报 `Unregistered`（配置缺陷，**不是** `Provider`）。
#[tokio::test]
async fn a_tool_in_the_table_without_an_adapter_passes_authorize_then_fails_to_route() {
    let (_dir, db) = db();
    let tx = db.begin().unwrap();
    save_tool(&tx, &profile("t1")).unwrap();
    // 前一半：治理侧登记在手，authorize 放行——本向的错法正从这里开始（fail-open）。
    let auth = authorize(&tx, &ToolId::new("t1"), &[], 1_000)
        .expect("capability 侧登记了，authorize 应当放行");
    tx.commit().unwrap();

    // 注册表**空**：一个适配器都没有（本向刻意如此，理由见文件头）。
    let registry = ProviderRegistry::new();
    let err = registry
        .invoke_tool(&auth, json!({"x": 1}))
        .await
        .expect_err("注册表里没有服务这个 id 的适配器，路由必须未命中");

    // 后一半：未命中报的是 `Unregistered` 这一种 `Err`。
    assert!(
        matches!(&err, ToolCallError::Unregistered { id } if id.as_str() == "t1"),
        "路由未命中应是 Unregistered {{ id: t1 }}，实际 {err:?}"
    );
    assert!(
        !matches!(err, ToolCallError::Provider(_)),
        "配置缺陷不得报成 provider 失败，实际 {err:?}"
    );
}

/// **第二向**：`ProviderRegistry` 登记了适配器、`tool` 表**当时没有**这一行。
///
/// `authorize` 报 `UnknownTool`，且发生在 `invoke_tool` **之前**——没有授权就构造不出
/// 调用，故这一路径上注册表一次都没被碰过（下面用 `Recorder` 把这句话的主语显出来）。
#[test]
fn an_adapter_without_a_row_in_the_table_fails_authorize_before_any_call() {
    let (_dir, db) = db();
    let (adapter, recorder) = RecordingTool::new(ToolId::new("t1"), RecorderOutcome::Echo);
    let mut registry = ProviderRegistry::new();
    registry
        .register_tool(vec![ToolId::new("t1")], Arc::new(adapter))
        .unwrap();

    let tx = db.begin().unwrap();
    let rejected = authorize(&tx, &ToolId::new("t1"), &[], 1_000);
    assert!(
        matches!(&rejected, Err(CapabilityError::UnknownTool { id }) if id.as_str() == "t1"),
        "表里没有这一行，authorize 应报 UnknownTool，实际 {rejected:?}"
    );

    // 把行**补在这之后**：上面那条 Err 是「查了表、表里当时没有」的结果。本 task Step 3 的
    // 第二条变异正是把这一句挪到 `authorize` 之前——表里就有了行，上面那条断言随即失效，
    // 这条变异钉的便是「断言确实在查表」。
    save_tool(&tx, &profile("t1")).unwrap();
    tx.commit().unwrap();

    // 没拿到授权，故这一路径上没有任何调用可发；门禁在路由之前，注册表一次都没被碰过。
    assert_eq!(
        recorder.touches(),
        0,
        "authorize 被拒时，注册表一次都不该被碰过"
    );
    assert_eq!(
        recorder.seen_tool_id(),
        None,
        "更不该有适配器收到过授权"
    );
}
