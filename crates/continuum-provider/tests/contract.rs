//! 工具级失败与 provider 级失败的分流（**约定**，非规范条文；Task 7）。
//!
//! **本文件钉的是夹具，不是真实适配器。** 「工具级失败走
//! `Ok(ToolResult { is_error: true, .. })`、provider 级失败走 `Err`」这条分流的出处是
//! **本子项目 C 写在适配器契约上的约定**，§316 里没有出处、也没有任何强制（设计 §7.1）——
//! 真实适配器完全可以对工具级失败返 `Err(Protocol)`。故能证明的是**两件事**：
//! **`FakeTool` 服从这条约定**，**且 `invoke_tool` 把它的 `Ok` / `Err` 原样转出去**。
//! 报告与注释都不得写成「适配器都这么干」。真实适配器上这条分流**拍不到**
//! （设计 §9 把它列在「拍不到的」里）。
//!
//! 被测的仍是 [`ProviderRegistry::invoke_tool`]（Task 4）：它**不替适配器做分流**，
//! `Ok` / `Err` 一律按适配器给的转出去（`src/registry.rs` 的对应文档段）。本文件两条用例
//! 各自钉住转出去的那一支。
//!
//! **两条用例不是等价变异体**：把适配器 `invoke` 的实现在**同一条输入**上从
//! `Ok(ToolResult { is_error: true, .. })` 改成 `Err(Protocol)`（或反过来），两条用例给出的
//! 结果**不同**（`Ok` vs `Err`），故各自都举得出让它**单独**变红的、仍能编译的变异体。
//!
//! **两条用例各只写一条断言。** 计划里「**不是 `Err`**」/「**不是 `Ok(is_error: true)`**」
//! 两处括注是**期望的措辞**，不是第二条断言（协调者裁定 2026-10-06）：`Ok(..)` 与 `Err(..)`
//! 是**对立的两支**，`matches!(Ok(..))` 成立即蕴含「不是 `Err`」，反之亦然——写成
//! `assert!` 就是蕴含型，**不可能独立变红**。故用 `matches!` 把要钉的那一支与它的载荷
//! （`is_error` 的真假即载荷）**一并断在同一条**里。这条口径与 Task 5 Step 1 的
//! 「统一判据」段同源。
//!
//! **`db()` / `profile()` / `authorized()` 照 Task 4 `tests/invoke_tool.rs` 同形另写一份**：
//! 本仓既有做法是每个测试目标各有一份，不做跨目标的共享夹具（同 crate 的
//! `tests/persist.rs` 亦是如此）。

mod common;

use common::FakeTool;
use continuum_capability::{
    AuthorizedTool, CapabilityKind, Tool, ToolId, ToolProfile, Trust, authorize,
    p3_capability_migrations, save_tool,
};
use continuum_core::ProviderError;
use continuum_persist::{Db, Migration, builtin_migrations};
use continuum_provider::{ProviderRegistry, ToolCallError};
use serde_json::json;
use std::sync::Arc;

/// 起一个临时库：内置迁移（`audit_log` 表来自那里）＋ capability 的 `tool` 表迁移。
fn db() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("contract.db");
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

/// 工具级失败（夹具报 `Ok(ToolResult { is_error: true, .. })`）→ 门禁内**得 `Ok` 且
/// `is_error` 为真**，**不是 `Err`**。
///
/// 后半句是上面这条期望的措辞、**不另写 `assert!`**（见文件头）：两变体互斥，它被本条严格
/// 蕴含、只在通过后可达，**举不出**让它单独红的变异体。载荷 `is_error` 断在**同一条**里。
#[tokio::test]
async fn a_tool_level_failure_is_ok_with_is_error_true() {
    let (_dir, auth) = authorized("echo");
    let mut registry = ProviderRegistry::new();
    registry
        .register_tool(
            vec![ToolId::new("echo")],
            Arc::new(FakeTool::failing_at_tool_level()),
        )
        .expect("首次登记应成功");

    let result = registry.invoke_tool(&auth, json!({"x": 1})).await;

    assert!(
        matches!(&result, Ok(r) if r.is_error),
        "工具级失败应是 Ok(ToolResult {{ is_error: true, .. }})，实际 {result:?}"
    );
}

/// provider 级失败（夹具报 `Err(ProviderError::Transport(..))`）→ 门禁内得
/// `Err(ToolCallError::Provider(_))`，**不是 `Ok(is_error: true)`**。
///
/// 后半句同样是措辞而非第二条断言（见文件头）。`ToolCallError::Provider` 那一臂与它内层
/// 适配器给出的那一个 `ProviderError` 臂**断在同一条**里——`invoke_tool` 不加工它。
#[tokio::test]
async fn a_provider_level_failure_is_err() {
    let (_dir, auth) = authorized("echo");
    let mut registry = ProviderRegistry::new();
    registry
        .register_tool(
            vec![ToolId::new("echo")],
            Arc::new(FakeTool::failing_at_provider_level()),
        )
        .expect("首次登记应成功");

    let result = registry.invoke_tool(&auth, json!({})).await;

    assert!(
        matches!(&result, Err(ToolCallError::Provider(ProviderError::Transport(_)))),
        "provider 级失败应是 Err(ToolCallError::Provider(..))，实际 {result:?}"
    );
}
