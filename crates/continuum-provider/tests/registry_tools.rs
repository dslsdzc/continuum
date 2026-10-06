//! 工具侧的登记与只读入口（设计 §3.1、§3.3 代价三、§3.5）。
//!
//! 本文件用的夹具：`common::FakeTool`（声明 `echo`，`describe_tool` 对声明外的 id 报
//! `Unavailable`），以及两个只为本文件服务的最小适配器——`OneTool`（可指定 id 与描述）与
//! `FailingList`（`list_tools` 恒失败）。

mod common;

use async_trait::async_trait;
use common::FakeTool;
use continuum_core::model::CallId;
use continuum_core::tool::{ToolDescriptor, ToolId, ToolResult};
use continuum_core::ProviderError;
use continuum_provider::tool::{AuthorizedToolInvocation, ToolProvider};
use continuum_provider::{ProviderRegistry, RegistryError, ToolCallError};
use serde_json::json;
use std::sync::Arc;

/// 只声明一个工具的最小适配器。`describe_tool` 与 `FakeTool` 同形：声明之外报 `Unavailable`。
struct OneTool {
    id: &'static str,
    description: &'static str,
}

impl OneTool {
    fn descriptor(&self) -> ToolDescriptor {
        ToolDescriptor {
            id: ToolId::new(self.id),
            description: self.description.into(),
            input_schema: json!({"type": "object"}),
        }
    }
}

#[async_trait]
impl ToolProvider for OneTool {
    async fn list_tools(&self) -> Result<Vec<ToolDescriptor>, ProviderError> {
        Ok(vec![self.descriptor()])
    }

    async fn describe_tool(&self, id: &ToolId) -> Result<ToolDescriptor, ProviderError> {
        if &self.descriptor().id == id {
            Ok(self.descriptor())
        } else {
            Err(ProviderError::Unavailable(id.as_str().to_owned()))
        }
    }

    async fn invoke(
        &self,
        call: AuthorizedToolInvocation<'_>,
    ) -> Result<ToolResult, ProviderError> {
        Ok(ToolResult {
            output: call.input().clone(),
            is_error: false,
        })
    }

    async fn cancel(&self, _call: &CallId) -> Result<(), ProviderError> {
        Ok(())
    }
}

/// `list_tools` 恒失败的适配器：用来钉「同一个适配器的失败不被吞、不被跳过」。
///
/// 它的 `describe_tool` 仍可用（返一个合法描述），故「跳过坏适配器」这条变异若把失败路径丢掉，
/// 只会让 `list_tools` 少一条而不会让别的用例红——这正是本适配器存在的理由。
struct FailingList;

#[async_trait]
impl ToolProvider for FailingList {
    async fn list_tools(&self) -> Result<Vec<ToolDescriptor>, ProviderError> {
        Err(ProviderError::Transport("适配器不可达".into()))
    }

    async fn describe_tool(&self, id: &ToolId) -> Result<ToolDescriptor, ProviderError> {
        Ok(ToolDescriptor {
            id: id.clone(),
            description: "坏适配器也能给描述".into(),
            input_schema: json!({}),
        })
    }

    async fn invoke(
        &self,
        call: AuthorizedToolInvocation<'_>,
    ) -> Result<ToolResult, ProviderError> {
        Ok(ToolResult {
            output: call.input().clone(),
            is_error: false,
        })
    }

    async fn cancel(&self, _call: &CallId) -> Result<(), ProviderError> {
        Ok(())
    }
}

fn tool(id: &str) -> ToolId {
    ToolId::new(id)
}

fn one(id: &'static str, description: &'static str) -> Arc<dyn ToolProvider> {
    Arc::new(OneTool { id, description })
}

#[tokio::test]
async fn a_registered_tool_is_described_by_id() {
    let mut registry = ProviderRegistry::new();
    registry
        .register_tool(vec![tool("echo")], Arc::new(FakeTool))
        .expect("首次登记应成功");

    let described = registry
        .describe_tool(&tool("echo"))
        .await
        .expect("已登记的 id 应命中所属适配器");
    let listed = registry.list_tools().await.expect("注册表应能列出工具");

    assert_eq!(
        described.id,
        tool("echo"),
        "描述里的 id 应就是被查的那个"
    );
    assert_eq!(
        described.id,
        listed[0].id,
        "`describe_tool` 与 `list_tools` 对同一个 id 应给同一条"
    );
    assert_eq!(listed[0].description, "回显输入", "描述取自适配器");
}

#[tokio::test]
async fn an_unregistered_tool_id_is_reported_as_unregistered() {
    let mut registry = ProviderRegistry::new();
    registry
        .register_tool(vec![tool("echo")], Arc::new(FakeTool))
        .expect("首次登记应成功");

    let err = registry
        .describe_tool(&tool("nope"))
        .await
        .expect_err("未登记的 id 不该命中");
    // 这一条同时排掉 `Provider`（两变体互斥）：不另写 `!matches!(Provider(_))`——它被本条
    // 严格蕴含、只在通过后可达，**举不出**一个让它单独红的变异体（评审判据 2026-10-06）。
    assert!(
        matches!(err, ToolCallError::Unregistered { .. }),
        "必须是 Unregistered 这一种 Err（路由层判据），实际 {err:?}"
    );
}

#[tokio::test]
async fn list_tools_is_the_union_of_every_registered_adapter() {
    let mut registry = ProviderRegistry::new();
    registry
        .register_tool(vec![tool("alpha")], one("alpha", "第一个适配器的工具"))
        .expect("首次登记应成功");
    registry
        .register_tool(vec![tool("beta")], one("beta", "第二个适配器的工具"))
        .expect("第二次登记应成功");

    let tools = registry.list_tools().await.expect("注册表应能列出工具");

    assert_eq!(tools.len(), 2, "两个适配器各出一个工具，并集应是两条");
    // 逐项有照片：顺序 = 登记顺序，且逐个断言 id——只抽一个代表即失守。
    assert_eq!(tools[0].id, tool("alpha"), "第 0 条应是先登记的那个");
    assert_eq!(tools[1].id, tool("beta"), "第 1 条应是后登记的那个");
    assert_eq!(tools[0].description, "第一个适配器的工具");
    assert_eq!(tools[1].description, "第二个适配器的工具");
}

#[tokio::test]
async fn list_tools_keeps_the_first_entry_when_two_adapters_declare_the_same_id() {
    let mut registry = ProviderRegistry::new();
    // **登记表的 id 自己不许重复**（`register_tool` 会拒），故第二个适配器登记在另一个 id 上、
    // 却在**它自己的 `list_tools()` 里**声明 `echo`——「登记 id」与「适配器声明的 id」是两个
    // 产生点（设计 §3.3 代价一），本用例正是在这一点上取并集。
    registry
        .register_tool(vec![tool("echo")], one("echo", "先登记的那个的描述"))
        .expect("首次登记应成功");
    registry
        .register_tool(vec![tool("beta")], one("echo", "后登记的那个的描述"))
        .expect("第二次登记应成功——登记的是另一个 id，重复的是适配器声明的 id");

    let tools = registry.list_tools().await.expect("注册表应能列出工具");

    assert_eq!(tools.len(), 1, "同一个 ToolId 只留一条");
    assert_eq!(tools[0].id, tool("echo"));
    // 断「是哪一条」，不只断「只有一条」——后登记的那条若覆盖了先登记的，本行即红。
    assert_eq!(
        tools[0].description, "先登记的那个的描述",
        "并集语义保留首次出现的那一条（先登记的适配器给出的描述）"
    );
}

#[tokio::test]
async fn registering_the_same_tool_id_twice_is_rejected_as_duplicate() {
    let mut registry = ProviderRegistry::new();
    registry
        .register_tool(vec![tool("echo")], one("echo", "先登记的那个的描述"))
        .expect("首次登记应成功");

    // 第二个适配器**登记**在 `echo`（故这一调真的撞上 `Duplicate`），但它**自己声明的是 `delta`**。
    // 这个错开是刻意的：若两个适配器都声明 `echo`，并集语义会把多出来的那一条吞掉，
    // 「被拒的登记没往适配器列表里塞东西」就无从观察——`list_tools()` 照样只有一条 `echo`。
    // 声明 `delta` 之后，那条被塞进去的适配器会在并集里现形（见下方两条 `!any` 断言）。
    let err = registry
        .register_tool(vec![tool("echo")], one("delta", "后登记的那个的描述"))
        .expect_err("同一个 id 登记第二次必须被拒");
    assert!(
        matches!(err, RegistryError::Duplicate { .. }),
        "必须是 Duplicate 这一种 Err，实际 {err:?}"
    );

    // 被拒的登记不许动原条目，也不许往适配器列表里塞一条——后者会让 `list_tools()` 多出一条。
    let still = registry
        .describe_tool(&tool("echo"))
        .await
        .expect("原条目应仍在");
    assert_eq!(
        still.description, "先登记的那个的描述",
        "路由必须还是第一次登记的那个适配器"
    );
    let listed = registry.list_tools().await.expect("注册表应能列出工具");
    assert_eq!(listed.len(), 1, "被拒的登记不该多出一条工具");
    // 这一条钉住**是哪一条**：id 与描述成对来自同一个 `ToolDescriptor`，故它同时排除掉
    // 「被拒适配器的那条（声明 `delta`／描述「后登记的那个的描述」）进来了」——「整体替换而非
    // 追加」的变异体会在这里先红。不另写 `!listed.iter().any(..)` 两条：`len() == 1` 加本条
    // 已把它们的变异空间完全包含，**举不出**让它们单独红的变异体（评审判据 2026-10-06）。
    assert_eq!(listed[0].description, "先登记的那个的描述");
}

#[tokio::test]
async fn registering_a_group_of_tool_ids_is_all_or_nothing() {
    let mut registry = ProviderRegistry::new();
    registry
        .register_tool(vec![tool("b")], one("b", "已占用的那个"))
        .expect("首次登记应成功");

    // 这一组要拒的是**登记表**里的 `b`；适配器自己声明的是 `a`，而 `a` 不在表里——
    // 于是「整组被拒之后适配器列表有没有被塞进一条」在 `list_tools()` 上是可观察的。
    let err = registry
        .register_tool(
            vec![tool("a"), tool("b")],
            one("a", "本该一起被拒的那个"),
        )
        .expect_err("组内有已占用的 id，整组必须被拒");
    assert!(
        matches!(err, RegistryError::Duplicate { .. }),
        "必须是 Duplicate 这一种 Err，实际 {err:?}"
    );

    let free = registry
        .describe_tool(&tool("a"))
        .await
        .expect_err("组内未被占用的 id 也不许进去——半登记的注册表无人看得见");
    assert!(
        matches!(free, ToolCallError::Unregistered { .. }),
        "a 未登记，应是 Unregistered 这一种 Err，实际 {free:?}"
    );
    let kept = registry
        .describe_tool(&tool("b"))
        .await
        .expect("b 仍是先登记的那个");
    assert_eq!(kept.description, "已占用的那个", "b 不该被这次被拒的登记改写");
    // 适配器列表侧同样不许被污染：被拒的适配器声明 `a`，它若被塞进列表，并集里就会多出 `a`。
    let listed = registry.list_tools().await.expect("注册表应能列出工具");
    assert_eq!(listed.len(), 1, "整组被拒不该往适配器列表里塞适配器");
    // 这两条一起排除掉「被塞进来的那条」：`len() == 1` 加本条（id 恰是 `b`）之后，元素级
    // `!listed.iter().any(|d| d.id == tool("a"))` 被完全包含，**举不出**让它单独红的变异体
    // （评审判据 2026-10-06）。
    assert_eq!(listed[0].id, tool("b"), "列表里只该有先登记的那个适配器");
}

#[tokio::test]
async fn list_tools_reports_an_adapter_failure_as_provider() {
    let mut registry = ProviderRegistry::new();
    registry
        .register_tool(vec![tool("echo")], Arc::new(FakeTool))
        .expect("首次登记应成功");
    registry
        .register_tool(vec![tool("broken")], Arc::new(FailingList))
        .expect("第二次登记应成功");

    let err = registry
        .list_tools()
        .await
        .expect_err("有一个适配器失败，整次列举就该失败——不许吞、不许跳过");
    // 这一条同时排掉 `Unregistered`（两变体互斥）：不另写 `!matches!(Unregistered)`——它被本条
    // 严格蕴含、只在通过后可达，**举不出**让它单独红的变异体（评审判据 2026-10-06）。
    assert!(
        matches!(err, ToolCallError::Provider(_)),
        "适配器自己的失败应包在 Provider 臂里，实际 {err:?}"
    );
}

#[tokio::test]
async fn describe_tool_routes_by_registration_not_by_the_adapters_list_tools() {
    let mut registry = ProviderRegistry::new();
    // `FakeTool::list_tools()` 只含 "echo"；登记是**按 id 显式**给出的，故两者可以不一致
    // （设计 §3.1 末段、§3.3 代价一）。登记到 "m" 之后，路由的权威是登记，不是适配器的清单。
    registry
        .register_tool(vec![tool("m")], Arc::new(FakeTool))
        .expect("登记应成功");

    let routed = registry
        .describe_tool(&tool("m"))
        .await
        .expect_err("适配器不认 m，故它给不出描述");
    // 反序实现（先查 `list_tools()` 再路由）在上一条断言即红：它给的是 `Unregistered`，
    // 与本条的 `Provider(Unavailable(_))` 互斥。故不另写 `!matches!(Unregistered)`——被本条
    // 严格蕴含、只在通过后可达，**举不出**让它单独红的变异体（评审判据 2026-10-06）。
    assert!(
        matches!(
            routed,
            ToolCallError::Provider(ProviderError::Unavailable(_))
        ),
        "应先按登记路由到适配器，再转出它自己的 Unavailable，实际 {routed:?}"
    );
}
