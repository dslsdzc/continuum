//! Sandbox 抽象：两种内核层隔离机制的统一入口。

use crate::error::SandboxError;
use continuum_workspace::TaskWorkspace;
use std::process::{Child, Command};

/// 实际生效的隔离项。
///
/// **每一项都是「已生效」而非「机制声称支持」。** 这是本类型唯一的取值约定：
/// 机制因内核 ABI 不足、可执行文件缺失或参数构造失败而未能施加某类隔离时，
/// 对应字段必须是 `false`。调用方与测试据此判断隔离的实际强度，
/// 一个如实报告 `false` 的沙箱是可用状态，一个谎报 `true` 的沙箱不是。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SandboxCapabilities {
    /// 文件系统写入被限制在 Task Workspace 内。
    pub restricts_filesystem_writes: bool,
    /// 网络命名空间被隔离。
    pub isolates_network: bool,
    /// PID 命名空间被隔离。
    pub isolates_pid: bool,
    /// 本能力报告所属的机制名，用于报告与断言。
    pub mechanism: &'static str,
}

/// Landlock 机制。
///
/// 本 task 中它不携带状态，句柄经 [`Sandbox::landlock`] 取得；Task 6 起承载内核
/// ABI 探测结果与只读放行路径集合。
pub struct LandlockSandbox {
    // Task 6 起承载该机制的状态：内核 ABI 探测结果与只读放行路径集合。
}

/// bubblewrap 机制。
///
/// 本 task 中它不携带状态，句柄经 [`Sandbox::bubblewrap`] 取得；Task 7 起承载
/// bwrap 可执行文件的定位结果与固定的加固参数。
pub struct BubblewrapSandbox {
    // Task 7 起承载该机制的状态：bwrap 可执行文件的定位结果与固定的加固参数。
}

/// 内核层隔离的统一入口。
///
/// 两种机制在这一层不可区分：调用方只经 [`Sandbox::spawn`] 启动子进程、
/// 经 [`Sandbox::capabilities`] 读取实际生效的隔离项，不感知机制差异。
pub enum Sandbox {
    /// Landlock：进程级规则集，随 `exec` 生效并被子孙继承。
    Landlock(LandlockSandbox),
    /// bubblewrap：挂载、网络与 PID 命名空间。
    Bubblewrap(BubblewrapSandbox),
}

impl Sandbox {
    /// Landlock 机制句柄。
    pub fn landlock() -> Self {
        Sandbox::Landlock(LandlockSandbox {})
    }

    /// bubblewrap 机制句柄。
    pub fn bubblewrap() -> Self {
        Sandbox::Bubblewrap(BubblewrapSandbox {})
    }

    /// 在 Task Workspace 内启动命令。
    ///
    /// **签名是刻意的**：参数是 `&TaskWorkspace` 而非 `&Path`，故「把 Base Workspace
    /// 交给子进程」在类型上不可表达——调用方手里没有可写根之外的东西可传。
    /// 设计第 4.2 节要求子进程的工作目录是 Task 根，该设置已在本函数内完成。
    ///
    /// # 当前状态
    ///
    /// 两种机制均未实现（Task 6、Task 7），故本函数在设好工作目录后即 panic，
    /// **不提供任何隔离**。这是刻意的：一个「照常启动但不施加隔离」的实现会让调用方
    /// 以为子进程已被约束，与 [`SandboxCapabilities`]「如实报告实际生效的隔离项」
    /// 这一约定无法同时成立。
    pub fn spawn(&self, task: &TaskWorkspace, cmd: &mut Command) -> Result<Child, SandboxError> {
        cmd.current_dir(task.root());
        match self {
            Sandbox::Landlock(_) => todo!("Task 6：在 pre_exec 中构建并施加 Landlock 规则集"),
            Sandbox::Bubblewrap(_) => todo!("Task 7：改写为 bwrap 调用后启动"),
        }
    }

    /// 实际生效的隔离项。
    ///
    /// 返回值反映的是**已经施加**的隔离，不是机制的设计能力。因内核 ABI 不足而降级、
    /// 或因可执行文件缺失而未能施加时，对应字段为 `false`。
    ///
    /// # 当前状态
    ///
    /// 两种机制都尚未施加任何隔离（见 [`Sandbox::spawn`]），故各项为 `false`。
    /// 这不是占位值：在「实际生效」的取值约定下，它正是当前唯一正确的答案。
    /// Task 6 与 Task 7 落地后由各自的实现据实返回。
    pub fn capabilities(&self) -> SandboxCapabilities {
        match self {
            Sandbox::Landlock(_) => SandboxCapabilities {
                restricts_filesystem_writes: false,
                isolates_network: false,
                isolates_pid: false,
                mechanism: "landlock",
            },
            Sandbox::Bubblewrap(_) => SandboxCapabilities {
                restricts_filesystem_writes: false,
                isolates_network: false,
                isolates_pid: false,
                mechanism: "bubblewrap",
            },
        }
    }
}
