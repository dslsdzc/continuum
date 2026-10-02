//! Sandbox 抽象：两种内核层隔离机制的统一入口。

use crate::error::SandboxError;
use continuum_workspace::TaskWorkspace;
use std::process::{Child, Command};

/// 隔离项报告。
///
/// **每一项都是「按当前内核，该机制将生效」的隔离项，而非机制自称的能力。** 这是本类型
/// 唯一的取值约定：Landlock 的 ABI 不支持某类访问时，对应字段必须是 `false`。
/// 调用方与测试据此判断隔离的实际强度——一个如实报告 `false` 的报告是可用状态，
/// 一个谎报 `true` 的不是。
///
/// 该值来自**父进程侧**的内核 ABI 探测（设计第 4.3 节），不是子进程内的实际施加结果：
/// `restrict_self` 只能在被限制的进程内调用，而子进程是调用方给定的任意命令，无法回传。
/// 「本次确实生效」由设计第 13 节的两臂用例在行为上验证。
///
/// 残余缺口：`restrict_self` 因 ABI 之外的原因失败时，子进程直接以 spawn 错误终止，
/// 本报告不会因此为假——方向是响的。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SandboxCapabilities {
    /// 文件系统写入被限制在 Task Workspace 内。
    pub restricts_filesystem_writes: bool,
    /// 网络命名空间被隔离。
    pub isolates_network: bool,
    /// PID 命名空间被隔离。
    pub isolates_pid: bool,
    /// 本报告所属的机制名，用于报告与断言。
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
    /// bubblewrap：以挂载命名空间限制文件系统。其网络与 PID 命名空间能力本子项目不使用
    /// （Task 7 的参数集不含 `--unshare-net` / `--unshare-pid`），故届时
    /// [`Sandbox::capabilities`] 对这两项报告为假——该报告的口径是「实际生效」。
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
    /// **签名是刻意的**：workspace 参数是 `&TaskWorkspace` 而非 `&Path`，故「把 Base
    /// Workspace 当作 workspace 传进来」在类型上不可表达。**不要把这条读成「Base 路径
    /// 不可能进入子进程参数」**——`cmd` 的 argv 与环境变量完全由调用方给定，
    /// `cmd.arg(format!("…{}", base.root().display()))` 一行即是反例。设计第 4.2 节要求
    /// 「argv 与环境变量由调用方给定且不包含 Base 路径」，那是**调用方义务**，
    /// 由第 13 节的用例（检查实际 spawn 的 argv / env / cwd）验证；即便参数里真出现
    /// Base 路径，写入仍被内核层拒绝。
    ///
    /// `cmd` **按值**收：所有权交给本层，因为本层未必照原样启动它。Landlock 只需在同一
    /// 条命令上挂 `pre_exec`，bubblewrap 则要据此重建一条 `bwrap …` 命令。后者附带一条
    /// 必须写明的限制：`std::process::Command` 不提供 stdin / stdout / stderr 的读取接口
    /// （rustc 1.95.0 实测无 `get_stdout`），故重建命令时**无法保留**调用方设置的 stdio。
    /// 需要管道的调用方不要依赖 bubblewrap 分支保留它。
    ///
    /// 设计第 4.2 节要求子进程的工作目录是 Task 根，该设置已在本函数内完成。
    ///
    /// # 当前状态
    ///
    /// 两种机制均未实现（Task 6、Task 7），故本函数在设好工作目录后即 panic，
    /// **不提供任何隔离**。这是刻意的：一个「照常启动但不施加隔离」的实现会让调用方
    /// 以为子进程已被约束，与 [`SandboxCapabilities`]「如实报告隔离项」这一约定
    /// 无法同时成立。
    pub fn spawn(&self, task: &TaskWorkspace, mut cmd: Command) -> Result<Child, SandboxError> {
        cmd.current_dir(task.root());
        match self {
            Sandbox::Landlock(_) => todo!("Task 6：在 pre_exec 中施加 Landlock 规则集"),
            Sandbox::Bubblewrap(_) => todo!("Task 7：据此重建 bwrap 命令后启动"),
        }
    }

    /// 按当前内核的 ABI 探测结果，本机制将生效的隔离项。
    ///
    /// 取值约定见 [`SandboxCapabilities`]：返回的是「按这个内核，该机制**将**生效」的
    /// 隔离项，不是机制自称的能力，也不是子进程内的实际施加结果。
    ///
    /// # 当前状态
    ///
    /// 两种机制都尚未实现（Task 6、Task 7），探测入口亦未建立，故各项为 `false`：
    /// 此刻没有任何机制会生效，这是唯一正确的报告。Task 6 与 Task 7 落地后由各自的
    /// 实现据实返回。
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
