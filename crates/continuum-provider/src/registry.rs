//! 进程内注册表（设计 §3.1）与两个错误类型（设计 §3.5）。
//!
//! 本文件只做**路由**：记下「哪个 id 由哪个适配器服务」。它不含任何适配器实现，
//! 也不引用任何实现类型——持有的只有 `Arc<dyn …>`（设计 §4.1）。

use crate::model::ModelProvider;
use crate::tool::{AuthorizedToolInvocation, ToolProvider};
use continuum_capability::AuthorizedTool;
use continuum_core::model::ModelId;
use continuum_core::tool::{ToolDescriptor, ToolId, ToolResult};
use continuum_core::ProviderError;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// 登记与查找的失败。**与 `ProviderError` 判据不同、不可合并**（设计 §3.5）：
/// `ProviderError` 描述**一次 provider 调用**的失败（可能瞬时，可重试）；
/// 本类型描述**路由 / 装配**的失败（配置缺陷，重试不会变）。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RegistryError {
    /// 模型侧按 id 发现时未命中（设计 §3.1）。**工具侧不走这里**——工具侧的未命中是
    /// [`ToolCallError::Unregistered`]。
    #[error("模型 {} 未登记", id.as_str())]
    NotFound { id: ModelId },
    /// 同一个 id 被登记第二次（模型侧与工具侧共用）。
    ///
    /// **与 `crates/continuum-operator/src/registry.rs:10-11` 的 `OperatorError::Duplicate`
    /// 同的只是「重复登记有一个具名变体」这一取向，不是字段形状**：那里两个字段都是强类型
    /// （`OperatorId` / `OperatorVersion`），而本注册表有**两个 id 类型**（`ModelId` / `ToolId`），
    /// 收窄到某一种就得为另一种再造一个变体。
    ///
    /// **字段形状规范未给判据**（裁决 C5 明写归遗留）：本计划取「以文本承载 id，
    /// 是哪一张表由调用点可知（`register_model` / `register_tool`），故不另立字段」，
    /// 并把它作为**计划的取法**申报在计划 `## 遗留`（三）。
    #[error("id {id} 已登记")]
    Duplicate { id: String },
}

/// 受门禁的 `invoke_tool` 与工具侧按 id 查的只读入口的失败（设计 §3.5）。
///
/// **为什么需要第二个错误类型**：`invoke_tool` 的失败有两个不同来源——「这个 id 没有适配器」
/// （路由，配置缺陷）与「适配器调用失败」（可能瞬时）；一个 `Result` 只能带一个错误类型，
/// 把两者压进 `ProviderError` 就是把「配置错了」报成「provider 挂了」——而那件事在本仓
/// 已经发生过一次（`crates/continuum-provider/tests/fake_provider.rs` 的 `FakeTool::describe_tool`
/// 把「无此工具」报成 `Unavailable`，设计 §5.2 记的正是这一处）。
///
/// `list_tools()` 是已登记适配器各自 `list_tools()` 的并集、**不按 id 查**，
/// 故 [`ToolCallError::Unregistered`] 在它那条路径上不可达（设计 §3.5 的订正段）。
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ToolCallError {
    /// 这个 id 没有登记的适配器。（工具 id 只有授权证明一个来源，故本变体只由路由产生。）
    #[error("工具 {} 没有登记的适配器", id.as_str())]
    Unregistered { id: ToolId },
    /// 适配器自己失败。
    #[error("适配器调用失败: {0}")]
    Provider(#[from] ProviderError),
}

/// 进程内注册表（设计 §3.1）。
///
/// 设计取**一个结构体持两张表**（模型侧与工具侧），不拆两个结构体（§3.4 的理由：装配点只有一个）。
/// 工具侧除了那张表**另持一份登记顺序的适配器列表**——`list_tools()` 是已登记适配器各自
/// `list_tools()` 的并集，故它要能遍历；**模型侧不持这份列表**（它的枚举入口已裁删，留一份
/// 没人遍历的列表就是同一种「建好没人用」，设计 §3.1 的订正段）。
///
/// 适配器是**代码**（`Arc<dyn …>`），不是数据——故它不进库，也不是一张表。
///
/// **暴露面两侧不对称，理由是强制点 (1) 只覆盖工具**（设计 §3.1）：模型侧 `model_for` 直接
/// 交出裸 `Arc<dyn ModelProvider>`；工具侧只开只读入口（`list_tools` / `describe_tool`）与
/// 唯一的调用入口 `invoke_tool`（后者 Task 4 起落地），**不交出适配器**——**没有 `tool_for`、
/// 也没有 `tool_providers()`**。后两者缺席的证据归 Task 6 的编译失败样例，**本处不对此作任何主张**
/// ——本处没有能证明「这两个名字编不过」的用例。
///
/// **本注册表没有「枚举有哪些模型」的入口，这是有意的**（设计 §3.1 的裁决，2026-10-06）：
/// ① 它零消费方；② 枚举出来的 `Arc<dyn ModelProvider>` **不带 id**，而模型调用路径是**按 id 解析**
/// 的（`model_for`），两者无从对齐；③ 「有哪些模型」的权威在子项目 D 的 Model Registry 表，
/// 不在本注册表——**本注册表只按 id 解析就够了**。
#[derive(Default)]
pub struct ProviderRegistry {
    /// `ModelId → 适配器`。这是**路由的唯一权威**：发现只看它，不去问适配器的 `list_models()`
    /// （设计 §3.3 代价一）。
    models: HashMap<ModelId, Arc<dyn ModelProvider>>,
    /// `ToolId → 适配器`。同样只做**路由**，不拿它替代适配器的 `list_tools()`（设计 §3.1 末段）。
    tools: HashMap<ToolId, Arc<dyn ToolProvider>>,
    /// 已登记的工具适配器，**按登记顺序**。`list_tools()` 要逐个问它们，故必须能遍历；
    /// 同一个适配器登记几次就在此出现几次（`list_tools()` 的并集语义会去掉重复的 id）。
    tool_adapters: Vec<Arc<dyn ToolProvider>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// 把**一组 id** 登记给同一个适配器（设计 §3.1：id 由登记方显式给出，注册表不去问适配器）。
    ///
    /// **先全查后全插（原子）**：组内任一 id 已占用即返回 [`RegistryError::Duplicate`]，
    /// **一个都不登记**。半登记的注册表会让「谁服务这个 id」这件事在两次启动之间漂移，
    /// 而漂移没有任何用例看得见。**设计未规定这一点**，本计划取原子并给出照片
    /// （`registering_a_group_of_ids_is_all_or_nothing`）。
    pub fn register_model(
        &mut self,
        ids: Vec<ModelId>,
        provider: Arc<dyn ModelProvider>,
    ) -> Result<(), RegistryError> {
        for id in &ids {
            if self.models.contains_key(id) {
                return Err(RegistryError::Duplicate {
                    id: id.as_str().to_owned(),
                });
            }
        }
        for id in ids {
            self.models.insert(id, Arc::clone(&provider));
        }
        Ok(())
    }

    /// 按 id 发现。未登记 → [`RegistryError::NotFound`]。
    pub fn model_for(&self, id: &ModelId) -> Result<Arc<dyn ModelProvider>, RegistryError> {
        self.models
            .get(id)
            .map(Arc::clone)
            .ok_or_else(|| RegistryError::NotFound { id: id.clone() })
    }

    /// 工具侧登记。与 [`ProviderRegistry::register_model`] 同形：一组 id、一个适配器、先全查后全插。
    ///
    /// **注意那是适配器登记，不是往 `tool` 表登记**——后者是 `continuum_capability::save_tool`，
    /// 属治理（要哪些能力、什么 effect_class）；本处只管路由（谁去跑）。设计 §3.3 代价三。
    ///
    /// **`ids` 为空时的语义**（设计未规定，本计划取此；与 `register_model` 的取法同形，
    /// 那里空列表同样是「登记表不动」）：两个循环都不做，`tools` 表里不留下任何一条路由，
    /// 但适配器**仍进 `tool_adapters`**——于是它在 `list_tools()` 的并集里照常出现（收进的是
    /// 它自己 `list_tools()` 声明的那些 id），而那些 id 没有任何路由指向它，
    /// 故 `describe_tool` 对它们报 [`ToolCallError::Unregistered`]。
    /// **本行为没有用例**：触发它需要一次零 id 的登记，而本仓现有调用点全是本模块的测试，
    /// 每一个都显式给出非空 id 列表；写在这里是为了让「空列表不是漏判」这件事有处可查，
    /// 不是为了主张它已被验过。
    pub fn register_tool(
        &mut self,
        ids: Vec<ToolId>,
        provider: Arc<dyn ToolProvider>,
    ) -> Result<(), RegistryError> {
        for id in &ids {
            if self.tools.contains_key(id) {
                return Err(RegistryError::Duplicate {
                    id: id.as_str().to_owned(),
                });
            }
        }
        for id in ids {
            self.tools.insert(id, Arc::clone(&provider));
        }
        self.tool_adapters.push(provider);
        Ok(())
    }

    /// **已登记适配器**各自 `list_tools()` 的并集，按登记顺序。
    ///
    /// 任一适配器失败即返回该 `Err`（**不吞、不跳过**）——静默跳过一个坏适配器，会让
    /// 「哪些工具存在」在下游少一块而无人看得见。
    ///
    /// **并集语义**：同一个 `ToolId` 被多个适配器声明时保留**首次出现**的那一条。
    /// **设计未给判据**，本计划取并集并附照片，见计划 `## 遗留`。
    ///
    /// **它不按 id 查，故本入口上 [`ToolCallError::Unregistered`] 臂不可达**（设计 §3.5 的订正段）。
    pub async fn list_tools(&self) -> Result<Vec<ToolDescriptor>, ToolCallError> {
        let mut listed: Vec<ToolDescriptor> = Vec::new();
        for adapter in &self.tool_adapters {
            for descriptor in adapter.list_tools().await? {
                if !listed.iter().any(|seen| seen.id == descriptor.id) {
                    listed.push(descriptor);
                }
            }
        }
        Ok(listed)
    }

    /// 先按**登记**路由到适配器，再转出它的描述；id 未登记 → [`ToolCallError::Unregistered`]。
    ///
    /// **它不看 `list_tools()`**：登记是路由的权威、适配器的 `list_*` 是描述的权威，
    /// 两者可以不一致（设计 §3.1 末段）。
    pub async fn describe_tool(&self, id: &ToolId) -> Result<ToolDescriptor, ToolCallError> {
        let adapter = self
            .tools
            .get(id)
            .ok_or_else(|| ToolCallError::Unregistered { id: id.clone() })?;
        Ok(adapter.describe_tool(id).await?)
    }

    /// **工具侧唯一的调用入口**（设计 §7.1）。解析与调用是同一步。
    ///
    /// 工具 id **只有这一个来源**：`authorized.tool_id()`。本函数**不另收 `ToolId`**
    /// （旧类型 `ToolInvocation` 已裁删，见 §7.5、§12 第 22 条），故「出示的 id 与被授权的
    /// id 不是一个」这一种可能不存在。
    ///
    /// **未登记的 id 不构成本次调用失败**，它是**配置缺陷**（[`ToolCallError::Unregistered`]）
    /// ——调用方**不得**把它当作可重试（设计 §5.1）。
    ///
    /// **它交出的是结果，不是适配器**：注册表**不提供** `tool_for`、也**不提供**
    /// `tool_providers()` 枚举（设计 §7.1）。那两条入口的缺席由 Task 6 的编译失败样例钉，
    /// **本处不对此作任何主张**——本处没有能证明「这两个名字编不过」的用例。
    ///
    /// **顺序**：先按 id 路由（未命中即返 `Unregistered`，**适配器一次都没被碰过**——
    /// 路由的权威是 `tools` 表，不是去逐个问适配器的 `list_tools()`，与
    /// [`ProviderRegistry::describe_tool`] 同一条口径），构造请求，再调适配器。
    /// [`AuthorizedToolInvocation`] **在本函数内部构造**——调用方只传 `&AuthorizedTool`
    /// 与 `input`，不构造也不命名那个类型。该类型的构造入口是 `pub(crate)`（裁决 C2），
    /// 故**本函数是全仓唯一的构造点**。
    ///
    /// **`Ok` / `Err` 一律按适配器给的转出去**：工具跑起来了但自身失败该报成
    /// `Ok(ToolResult { is_error: true, .. })` 是 **C 加在适配器上的约定**，§316 里没有出处、
    /// 也没有强制（设计 §7.1）；本函数**不替适配器做这个分流**——那样做等于把适配器
    /// 自己的判断覆盖掉，而本层没有依据判断它跑没跑成。
    pub async fn invoke_tool(
        &self,
        authorized: &AuthorizedTool,
        input: Value,
    ) -> Result<ToolResult, ToolCallError> {
        let id = authorized.tool_id();
        let adapter = self
            .tools
            .get(id)
            .ok_or_else(|| ToolCallError::Unregistered { id: id.clone() })?;
        let call = AuthorizedToolInvocation::new(authorized, input);
        Ok(adapter.invoke(call).await?)
    }
}
