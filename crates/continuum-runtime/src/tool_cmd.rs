//! `tool` 子命令的实现（F 设计 §3、§6.4）。
//!
//! 本模块只做三件事，**不含路径逻辑**——路径本身是 lib 的
//! [`run_tool_call`]（即 `continuum_runtime::tool_call::run_tool_call`）：
//!
//! 1. **装配**：开库（经 `task_cmd` 的 `open_db`，两条子命令共用同一份迁移集合）与构造
//!    一个 [`ProviderRegistry`]；
//! 2. **调 lib**：`run_tool_call(&db, &registry, args)`；
//! 3. **失败映射**：`Err(e)` 由 `main` 的分派臂打印并返回 `ExitCode::FAILURE`（与 `task`
//!    那一臂同形，见 `main.rs`）。
//!
//! **`ToolResult.output` 不在这里打印**：它在 lib 的步骤 7 已经打出——两条路径不能有
//! 两个输出点。
//!
//! # 装配点今天是空的（**不是遗留物**）
//!
//! 下面这一行构造出的注册表里**一个适配器都没有**：今天的驱动没有任何工具适配器实现
//! 可登记（设计 §10.2）。故一次真实调用的步骤 6 会以
//! `ToolCallError::Unregistered` 失败——**它不是 C（`continuum-provider`）的交付缺口**：
//! 注册表的登记入口已交付，而按「外围 crate 中立」的规则，C 不可能提供 `impl
//! ToolProvider` 的实现类型。缺的是**一个可登记的适配器实现**，而装配者＝驱动自己，
//! 收件人是驱动自己／将来的适配器子项目。**它不挡强制点 (1)**：`authorize` 排在那一跳
//! 之前，故授权那一侧今天有真的生产调用方。
//!
//! 也**不写成注释掉的样例代码**：登记入口的实参表以 C 的契约为准，本处不写死一个可能与
//! 它漂移的形状（设计 §10.2 明写不预先发明那个签名）。
//!
//! [`ProviderRegistry`]: continuum_provider::ProviderRegistry

use crate::task_cmd::open_db;
use continuum_provider::ProviderRegistry;
use continuum_runtime::TaskError;
use continuum_runtime::cli::ToolArgs;
use continuum_runtime::tool_call::run_tool_call;

/// 运行 `tool` 子命令。
///
/// 失败原样交给调用方（`main` 的分派臂打印并给出退出码）——本函数不自己打印任何东西。
pub fn run(args: &ToolArgs) -> Result<(), TaskError> {
    let db = open_db(&args.db)?;

    // 装配点：组合根把适配器登记进来（设计 §10.2）。今天的登记表是空的，见模块文档
    // 「装配点今天是空的」——这一行是**接缝本身**，不是「建好没人用」的管道。
    let registry = ProviderRegistry::new();

    run_tool_call(&db, &registry, args)
}
