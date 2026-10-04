//! 强制点 (1)：**唯一能把工具交给调用方的路径**（设计 §3.4）。
//!
//! 做成「非法状态不可表达」而非「调用方记得先校验」（与 P2 的 `WritablePath` 同形）：
//! [`AuthorizedTool`] 字段私有、无公开构造函数，**拿不到它就调不了工具**——
//! 「忘了校验」这一条路径在类型上不存在。crate 外无法构造它这一点的照片是
//! `tests/compile_fail/authorized_tool_cannot_be_built.rs` 的编译失败样例（设计 §8）。
//!
//! 取**自由函数** [`authorize`] 而非某个 `ToolRegistry` 类型的方法（设计 §3.4）：
//! 它要的东西全部从参数来——`Tx` 用于读登记项与写审计，没有需要挂在 `self` 上的状态，
//! 与 `continuum-workspace` 的 `approve_integration` 同形。「Tool Registry」在本子项目
//! 指 `tool` 表加 [`crate::persist`] 的两个读写函数，不是一个结构体。

use continuum_core::tool::ToolId;
use continuum_events::AuditKind;
use continuum_persist::{Tx, Value};

use crate::capability::Capability;
use crate::error::CapabilityError;
use crate::persist::load_tool;

/// 一次**已获准**的工具调用：工具 id，与调用方当场出示、且已通过校验的那些能力。
///
/// 字段私有、无公开构造函数（构造通道的编译失败样例见模块文档）。访问器只开两个，
/// 都是消费方要用的：这是哪个工具（[`AuthorizedTool::tool_id`]），以及准了哪些能力
/// （[`AuthorizedTool::granted`]）。**刻意不给 `#[allow(dead_code)]`**：字段由这两个
/// 访问器读，故不需要。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizedTool {
    tool_id: ToolId,
    granted: Vec<Capability>,
}

impl AuthorizedTool {
    /// 这次授权针对的工具。
    pub fn tool_id(&self) -> &ToolId {
        &self.tool_id
    }

    /// 已获准的能力：**按出示顺序照录**，含重复项。
    ///
    /// 不去重、不排序：工具声明的是一张 kind 表（[`crate::Tool::required_capabilities`]），
    /// 对作用域不作约束，故同 kind 出示两枚（作用域不同）时两枚都算获准；在此处替调用方
    /// 挑一枚或去重，等于凭空发明一条「哪一枚算数」的规则，而那件事本层没有依据决定。
    pub fn granted(&self) -> &[Capability] {
        &self.granted
    }
}

/// **唯一能把工具交给调用方的路径**（设计 §3.4）。
///
/// 取自由函数而非方法，理由见模块文档。
///
/// # 检查次序
///
/// 1. 读登记项：读失败 → [`CapabilityError::Persist`]；id 不在表里 →
///    [`CapabilityError::UnknownTool`]。两者都在任何能力比对**之前**——没有登记项就
///    没有声明表可比；
/// 2. 查**出示集**（按 `presented` 的顺序逐枚）：工具没声明的 →
///    [`CapabilityError::UndeclaredCapability`]；声明了但已失效 →
///    [`CapabilityError::Expired`]（复用 [`Capability::is_valid_at`]，**不重写**时钟
///    比较——那条逻辑只有一个产生点）。同一枚能力两项都犯时报前者：工具的声明是框架，
///    不在声明里的能力本就不该被出示；
/// 3. 查**声明集**（按 `required_capabilities` 的顺序逐枚）：出示集里没有同 kind 的
///    → [`CapabilityError::MissingCapability`]，多枚同时缺时报最靠前的那一枚。
///
/// **这一次序只决定报哪一种 `Err`，不决定放行与否**：放行判据是「出示集 ⊆ 声明集 且
/// 声明集 ⊆ 出示集」这两向的**合取**，两向都查完才可能返回 `Ok`，次序不参与该判断。
/// 次序可观察的差别只在同时犯两种错时报哪一种。先查出示集（子集）后查声明集（覆盖），
/// 因为出示集是调用方带来的、声明是工具固有的，先把带来的东西查干净更符合「拒在最早
/// 能拒的地方」。照片：`tests/authorize.rs` 的
/// `the_presented_set_is_checked_before_the_declared_set`——反序实现在那条用例上会红。
///
/// # 比对按 kind，作用域由能力自带
///
/// 工具声明的是 kind 表，没有作用域可比：一枚 `git.push:origin/main` 满足对
/// `Git(Push)` 的声明。作用域不被本层放宽或收窄，它随已获准的能力原样进入
/// [`AuthorizedTool`] 与审计。**本函数只做「工具收不收这一类动作」的核对，不判断
/// 作用域够不够**——那是执行点与凭据签发的活（设计 §4、§5）。
///
/// # 审计
///
/// **成功恰写一条** [`AuditKind::CapabilityGrants`]（该变体自 P0 起预留，此处第一次
/// 有产生方）：`occurred_at` 取调用方给的 `now`（本层不取时钟，与
/// [`Capability::is_valid_at`] 同一条约定），payload 是含工具 id 与每枚已获准能力的
/// kind / scope 的 JSON 对象。**拒绝一条都不写**，由调用方处置（设计 §3.4）——包括
/// 读登记项失败那条路径。两侧的照片都在 `tests/authorize.rs`。
///
/// # 与其他层的边界
///
/// [`crate::mint`] 铸不铸得出能力是**驱动**的判断（`mints` 六格表），本函数不复核；
/// 它只比对「已铸出的」与「工具声明的」。
pub fn authorize(
    tx: &Tx<'_>,
    tool_id: &ToolId,
    presented: &[Capability],
    now: i64,
) -> Result<AuthorizedTool, CapabilityError> {
    let profile = load_tool(tx, tool_id)?.ok_or_else(|| CapabilityError::UnknownTool {
        id: tool_id.clone(),
    })?;
    let declared = profile.tool().required_capabilities();

    // 第一遍：出示集必须是声明集的子集，且每枚都未失效。
    for capability in presented {
        if !declared.contains(&capability.kind()) {
            return Err(CapabilityError::UndeclaredCapability {
                kind: capability.kind(),
            });
        }
        // 时钟比较的唯一产生点：本处只转出它的结果。
        capability.is_valid_at(now)?;
    }

    // 第二遍：声明集必须被出示集覆盖。
    for kind in declared {
        if !presented.iter().any(|capability| capability.kind() == *kind) {
            return Err(CapabilityError::MissingCapability { kind: *kind });
        }
    }

    tx.append_audit(
        AuditKind::CapabilityGrants,
        now,
        Value::text(
            serde_json::to_string(&audit_payload(tool_id, presented))
                .expect("payload 由 json! 构造，必可序列化"),
        ),
    )?;

    Ok(AuthorizedTool {
        tool_id: tool_id.clone(),
        granted: presented.to_vec(),
    })
}

/// 成功授权的审计 payload：工具 id，加上每枚已获准能力的 kind 与 scope。
///
/// kind 取 [`crate::CapabilityKind::as_str`]——那是本 crate 落库编码唯一的产生点，
/// 审计与 `tool.required_capabilities` 列因此同源。写成 JSON 对象而非一行文本：
/// 字段要供后续对账取用，结构化比文本稳（与 `continuum-effect` 的审计同形）。
fn audit_payload(tool_id: &ToolId, granted: &[Capability]) -> serde_json::Value {
    serde_json::json!({
        "tool_id": tool_id.as_str(),
        "capabilities": granted
            .iter()
            .map(|capability| serde_json::json!({
                "kind": capability.kind().as_str(),
                "scope": capability.scope(),
            }))
            .collect::<Vec<_>>(),
    })
}
