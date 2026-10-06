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
//! **本文件两向都不承担「路由机制」与「注册表被碰过几次」这两条守卫**（协调者裁定
//! 2026-10-06；评审守卫判据 2026-10-06）：**一条断言是守卫，当且仅当在它实际所处的位置上，
//! 存在一个被测代码的变异使它会红**。
//!
//! - **路由机制**（「先逐个问遍所有适配器、再判未命中」）：第一向的注册表里**一个适配器都
//!   没有**，两版问的都是零个，那条变异在这里退化成等价变异体；
//! - **注册表被碰过几次**：由**用例自身控制流**决定——两向的调用方在 `authorize` 被拒
//!   （第二向）或在**空**注册表上未命中（第一向）之后都无调用可发，`register_tool` 也不碰
//!   适配器，没有任何 `authorize` / `invoke_tool` 的变异能移动那个计数，写出来也是恒真。
//!
//! 两条性质的守卫都是 Task 4 `tests/invoke_tool.rs` 的 `an_unregistered_tool_is_not_called_at_all`
//! ——那边登记了一个服务于**另一个 id** 的 `RecordingTool`，并断言 `recorder.touches() == 0`。
//! 本文件不复制它：同一件事的第二个落点是本仓反对的形状。
//!
//! **`db()` / `profile()` 照 Task 4 `tests/invoke_tool.rs` 同形另写一份**：本仓既有做法是
//! 每个测试目标各有一份，不做跨目标的共享夹具。

mod common;

use common::FakeTool;
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

    // 后一半：未命中报的是 `Unregistered` 这一种 `Err`。**这一条同时排掉 `Provider`**
    // （两个变体互斥，`Unregistered` 成立即「不是 `Provider`」成立）——故不另写一条
    // `!matches!(Provider(_))`：它会被这一条严格蕴含、只在通过后才可达，**不可能独立变红**，
    // 按守卫判据不成立（评审 2026-10-06）。
    assert!(
        matches!(&err, ToolCallError::Unregistered { id } if id.as_str() == "t1"),
        "路由未命中应是 Unregistered {{ id: t1 }}，实际 {err:?}"
    );
}

/// **第二向**：`ProviderRegistry` 登记了适配器、`tool` 表**当时没有**这一行。
///
/// `authorize` 报 `UnknownTool`，且发生在 `invoke_tool` **之前**——没有授权就构造不出
/// 调用。「注册表在这条路径上没被碰过」这句**在此处没有断言**：计数由本用例自身的控制流
/// 决定（下面压根不发调用），没有实现变异能移动它，故按守卫判据不写（见文件头）。
#[test]
fn an_adapter_without_a_row_in_the_table_fails_authorize_before_any_call() {
    let (_dir, db) = db();
    let mut registry = ProviderRegistry::new();
    // 服务哪个 id 由**登记**给出，不看适配器自己声明什么（`FakeTool` 声明的是 `echo`）——
    // 登记是路由的权威（设计 §3.1 末段）。本用例只要求「注册表里有个适配器」，不碰它的任何方法。
    registry
        .register_tool(vec![ToolId::new("t1")], Arc::new(FakeTool::echo()))
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
    //
    // **这一句对本用例的断言是惰性的**（只有 `.unwrap()` 会跑，且位于全部断言之后）：它的
    // 存在只为让上面那条变异是字面意义的「挪」而非「凭空插入」。属 plan 定下的产物，留此
    // 并据实标注（评审 2026-10-06）。
    save_tool(&tx, &profile("t1")).unwrap();
    tx.commit().unwrap();
}
