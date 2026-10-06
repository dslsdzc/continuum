//! 进程内注册表（设计 §3.1）与两个错误类型（设计 §3.5）。
//!
//! 本文件只做**路由**：记下「哪个 id 由哪个适配器服务」。它不含任何适配器实现，
//! 也不引用任何实现类型——持有的只有 `Arc<dyn …>`（设计 §4.1）。

use crate::model::ModelProvider;
use continuum_core::model::ModelId;
use continuum_core::tool::ToolId;
use continuum_core::ProviderError;
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
/// 工具侧那张表与它的入口由后续 task 加入；本处只有模型侧。
///
/// 适配器是**代码**（`Arc<dyn …>`），不是数据——故它不进库，也不是一张表。
///
/// **暴露面两侧不对称，理由是强制点 (1) 只覆盖工具**（设计 §3.1）：模型侧 `model_for` 直接
/// 交出裸 `Arc<dyn ModelProvider>`，另开 `model_providers()` 供枚举；工具侧只开只读入口与
/// 唯一的调用入口 `invoke_tool`，**不交出适配器**。
#[derive(Default)]
pub struct ProviderRegistry {
    /// `ModelId → 适配器`。这是**路由的唯一权威**：发现只看它，不去问适配器的 `list_models()`
    /// （设计 §3.3 代价一）。
    models: HashMap<ModelId, Arc<dyn ModelProvider>>,
    /// 登记顺序的模型适配器列表，供 `model_providers()` 枚举——**每次登记调用**贡献一项。
    model_adapters: Vec<Arc<dyn ModelProvider>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// 把**一组 id** 登记给同一个适配器（设计 §3.1：id 由登记方显式给出，注册表不去问适配器）。
    ///
    /// **先全查后全插（原子）**：组内任一 id 已占用即返回 [`RegistryError::Duplicate`]，
    /// **一个都不登记**，适配器列表也不加项。半登记的注册表会让「谁服务这个 id」这件事在两次
    /// 启动之间漂移，而漂移没有任何用例看得见。**设计未规定这一点**，本计划取原子并给出照片
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
        self.model_adapters.push(provider);
        Ok(())
    }

    /// 按 id 发现。未登记 → [`RegistryError::NotFound`]。
    pub fn model_for(&self, id: &ModelId) -> Result<Arc<dyn ModelProvider>, RegistryError> {
        self.models
            .get(id)
            .map(Arc::clone)
            .ok_or_else(|| RegistryError::NotFound { id: id.clone() })
    }

    /// 全部模型适配器，**按登记顺序**，每次登记调用贡献一项（设计 §3.1「另各持一份登记顺序的
    /// 适配器列表（供枚举）」）。返回 `Vec<Arc<…>>` 的克隆而非切片：调用方（子项目 G）
    /// 要在 `await` 期间持有它们。
    pub fn model_providers(&self) -> Vec<Arc<dyn ModelProvider>> {
        self.model_adapters.clone()
    }
}
