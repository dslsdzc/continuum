//! Sandbox 抽象：两种内核层隔离机制的统一入口。

use crate::error::SandboxError;
use crate::landlock;
use continuum_workspace::TaskWorkspace;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
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
/// 字段私有且无公开构造函数，故**唯一来源是 [`Sandbox::landlock`]**——crate 外拿不到
/// 句柄，也就无从绕过 [`Sandbox::capabilities`] 的判定另行断言一组能力。
///
/// 承载两项状态：内核 ABI 探测结果，与放行路径集合。**规则集本身不在此处**：
/// `RulesetCreated::restrict_self(mut self)` 消费规则集，它按构造即用，不可能存下来；
/// 且它的构造要分配内存，只能发生在 `pre_exec` 之前（见 [`Sandbox::spawn`]）。
pub struct LandlockSandbox {
    /// 构造期探测到的内核 ABI 版本；`0` 表示不可用（见 `landlock` 模块）。
    pub(crate) abi: i32,
    /// 只读放行的路径，构造时已滤除不存在的项。
    pub(crate) read_only_paths: Vec<PathBuf>,
    /// 放行全部访问权限的设备文件，构造时已滤除不存在的项。
    pub(crate) read_write_paths: Vec<PathBuf>,
}

/// bubblewrap 机制。
///
/// 本 task 中它不携带状态；**设计上**句柄经 [`Sandbox::bubblewrap`] 取得，该唯一性
/// 同 [`LandlockSandbox`] 一样要到 Task 7 加上私有字段后才成立。Task 7 起承载
/// bwrap 可执行文件的定位结果与固定的加固参数。
pub struct BubblewrapSandbox {
    // Task 7 起承载该机制的状态：bwrap 可执行文件的定位结果与固定的加固参数。
}

/// 内核层隔离的统一入口。
///
/// 两种机制在这一层不可区分：调用方只经 [`Sandbox::spawn`] 启动子进程、
/// 经 [`Sandbox::capabilities`] 读取「按当前内核的 ABI 探测结果，该机制将生效」的
/// 隔离项，不感知机制差异。
pub enum Sandbox {
    /// Landlock：进程级规则集，经 `restrict_self` 生效，并随 `exec` 被子孙继承。
    Landlock(LandlockSandbox),
    /// bubblewrap：以挂载命名空间限制文件系统。其网络与 PID 命名空间能力本子项目不使用
    /// （Task 7 的参数集不含 `--unshare-net` / `--unshare-pid`），故届时
    /// [`Sandbox::capabilities`] 对这两项报告为假——该报告的口径是「按当前内核的 ABI
    /// 探测结果，该机制将生效」，不含机制「本来能做什么」。
    Bubblewrap(BubblewrapSandbox),
}

impl Sandbox {
    /// Landlock 机制句柄。
    ///
    /// 构造期探测一次内核 ABI 并固定放行路径集合，故 [`Sandbox::capabilities`] 与
    /// [`Sandbox::spawn`] 依据的是同一个判定。
    pub fn landlock() -> Self {
        Sandbox::Landlock(LandlockSandbox::new())
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
    /// # 隔离在何处施加
    ///
    /// Landlock 分支在 `cmd` 上挂 `pre_exec`：闭包运行在 fork 之后、exec 之前，
    /// 在其中 `restrict_self`。**规则集本身在本函数内（即父进程侧）构造**——闭包里
    /// 不得分配内存，而 `handle_access` / `PathFd::new` / `add_rules` 都会分配；
    /// 闭包里只剩 `restrict_self`（其内是 `prctl(PR_SET_NO_NEW_PRIVS)` 与
    /// `landlock_restrict_self` 两个 syscall，成功路径无堆分配）。
    ///
    /// 施加于闭包所在的那条线程即覆盖整个子进程：按 `CommandExt::pre_exec` 的契约，
    /// 闭包在 **fork 之后**的子进程里运行，而 fork 出的子进程只有一条线程，
    /// 故不需要 `LANDLOCK_RESTRICT_SELF_TSYNC`（那是给多线程进程用的）。
    ///
    /// **两种「内核 ABI 不足」处置相反**（设计第 4.3 节）：
    ///
    /// - **ABI < 1（Landlock 整个不存在）→ [`SandboxError::MechanismUnavailable`]，拒绝启动。**
    ///   此时照常启动等于让子进程在零文件系统隔离下运行，违反 `§256`；
    ///   「`capabilities()` 会报假」不足以弥补——调用方可以不查。这条分支在内核未启用
    ///   Landlock 时会真的走到，读本函数时不要把它当作死代码。该内核上仍有 bubblewrap
    ///   可用（它要的是用户命名空间而非 Landlock），故运行时装不会被卡死。
    /// - **ABI ≥ 1 但某个访问类别不被支持 → 降级该类别、启动照常**，由
    ///   [`Sandbox::capabilities`] 如实反映。这条不经过本函数，由 `landlock` crate 的
    ///   BestEffort 在处理访问类别时消化。
    ///
    /// 其余失败的分界：父进程侧构造规则集失败（例如 Task 根打不开）返回
    /// [`SandboxError::IsolationFailed`]；`pre_exec` 内的失败则被 std 报成普通的
    /// spawn `io::Error`，与 exec 失败同形，只能落进 [`SandboxError::SpawnFailed`]
    /// ——两者的分界见 [`SandboxError`] 的文档。
    ///
    /// # 当前状态
    ///
    /// Landlock 分支已实现。bubblewrap 分支未实现（Task 7），命中即 panic。
    pub fn spawn(&self, task: &TaskWorkspace, mut cmd: Command) -> Result<Child, SandboxError> {
        cmd.current_dir(task.root());
        match self {
            Sandbox::Landlock(sandbox) => {
                // ABI < 1：Landlock 整个不存在，拒绝启动（设计第 4.3 节）。
                let Some(abi) = landlock::effective_abi(sandbox.abi()) else {
                    return Err(SandboxError::MechanismUnavailable {
                        mechanism: "landlock",
                        reason: format!(
                            "内核 Landlock ABI 为 {}（< 1），该机制在本机不存在",
                            sandbox.abi()
                        ),
                    });
                };
                let ruleset = sandbox.build_ruleset(
                    abi,
                    task.root(),
                )
                .map_err(|reason| SandboxError::IsolationFailed {
                    mechanism: "landlock",
                    reason: format!("无法构建 Landlock 规则集：{reason}"),
                })?;
                // `restrict_self` 消费规则集，而闭包必须可多次调用（`FnMut`），
                // 故经 `Option::take` 交出唯一的一份。
                let mut slot = Some(ruleset);
                // SAFETY: `pre_exec` 的契约是闭包内只允许 async-signal-safe 操作。
                // 本闭包只做 `take`（无分配）、`restrict_self`（`prctl` 与
                // `landlock_restrict_self` 两个 syscall，成功路径无堆分配），以及
                // 失败时把 errno 转成 `io::Error`——用的是 `from_raw_os_error`，
                // 同样不分配。规则集已在 fork 之前构造好。
                unsafe {
                    cmd.pre_exec(move || match slot.take() {
                        Some(ruleset) => ruleset.restrict_self().map(|_| ()).map_err(into_io_error),
                        // 到不了：fork 出的子进程只有一条线程，`pre_exec` 只被调用一次。
                        None => Err(std::io::Error::from_raw_os_error(libc::EINVAL)),
                    });
                }
                cmd.spawn()
                    .map_err(|e| SandboxError::SpawnFailed { reason: e.to_string() })
            }
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
    /// Landlock 分支据构造期探测到的 ABI 如实返回，推导见 `landlock::capabilities_for_abi`。
    /// bubblewrap 分支尚未实现（Task 7），各项为 `false`：此刻它确实不会生效，
    /// 这是唯一正确的报告。
    pub fn capabilities(&self) -> SandboxCapabilities {
        match self {
            Sandbox::Landlock(sandbox) => landlock::capabilities_for_abi(sandbox.abi()),
            Sandbox::Bubblewrap(_) => SandboxCapabilities {
                restricts_filesystem_writes: false,
                isolates_network: false,
                isolates_pid: false,
                mechanism: "bubblewrap",
            },
        }
    }
}

/// 把施加规则集的失败折成 `io::Error`，且**不分配内存**。
///
/// 本函数在 `pre_exec` 闭包内被调用——fork 之后、exec 之前不得分配。故只取 syscall 的
/// `errno` 走 [`std::io::Error::from_raw_os_error`]，而不是格式化一条消息
/// （`io::Error::other` 会把消息装箱）。`restrict_self` 的失败只可能来自两个 syscall
/// （`prctl` 与 `landlock_restrict_self`），两者的 `source` 都是原样的 OS 错误，
/// 其析构不涉及堆；其余形态在本路径上不可达，真出现则折成 `EPERM`。
fn into_io_error(e: ::landlock::RulesetError) -> std::io::Error {
    let code = match &e {
        ::landlock::RulesetError::RestrictSelf(
            ::landlock::RestrictSelfError::RestrictSelfCall { source, .. }
            | ::landlock::RestrictSelfError::SetNoNewPrivsCall { source, .. },
        ) => source.raw_os_error().unwrap_or(libc::EPERM),
        _ => libc::EPERM,
    };
    std::io::Error::from_raw_os_error(code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use continuum_workspace::{BaseWorkspace, IntentId};

    /// ABI < 1 时 `spawn` 拒绝启动，而不是照常启动一个零隔离的子进程。
    ///
    /// 本机内核支持 Landlock，探测结果永远 >= 1，故这条守卫只能靠直接构造一个
    /// `abi: 0` 的句柄来验——字段是 `pub(crate)`，crate 内的测试够得着。
    #[test]
    fn spawn_refuses_when_landlock_is_unavailable() {
        let dir = tempfile::tempdir().unwrap();
        let base_path = dir.path().join("base");
        std::fs::create_dir_all(&base_path).unwrap();
        let base = BaseWorkspace::new(&base_path).unwrap();
        let task =
            TaskWorkspace::new_outside(&base, dir.path().join("task"), IntentId::new("i1"))
                .unwrap();

        let sandbox = Sandbox::Landlock(LandlockSandbox {
            abi: 0,
            read_only_paths: vec![],
            read_write_paths: vec![],
        });

        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg("echo bad > inside.txt");
        let result = sandbox.spawn(&task, cmd);

        // 断言到具体的错误变体：只断言「返回了 Err」不足以把这条守卫与
        // 「规则集构造失败」之类的形态分开。
        assert!(
            matches!(
                result,
                Err(SandboxError::MechanismUnavailable { mechanism: "landlock", .. })
            ),
            "ABI=0 时 spawn 应以 MechanismUnavailable 拒绝，实际 {result:?}"
        );
        // 拒绝发生在启动之前：Task 内不得留下任何由子进程写出的文件。
        assert!(!task.root().join("inside.txt").exists());
    }

    /// 只读放行路径**真的是只读**：读得通、写不进。
    ///
    /// 为什么必须单独有用例：`isolation.rs` 的实验臂（「写 Base 被拒」）是**过量决定**的
    /// ——Base 落在任何被授予的子树之外，Landlock 对它是**读写皆不可达**，故那条断言
    /// 既可能因为「写被拒」成立，也可能因为「Base 整个不可达」成立，两者分不开。
    /// 本条把只读路径换成一个**在被授予范围内**的目录，就能把两者拆开：
    /// 读成功证明它可达，写被拒证明「只读」二字名副其实。
    ///
    /// 少了它，把只读放行从 `from_read` 改成 `from_all`（等于给 `/usr`、`/etc` 开写权限）
    /// 整套测试仍全绿——本 crate 已实测过这一点（变异 X1）。
    #[test]
    fn read_only_paths_are_readable_but_not_writable() {
        let dir = tempfile::tempdir().unwrap();
        let ro = dir.path().join("ro");
        std::fs::create_dir_all(&ro).unwrap();
        std::fs::write(ro.join("r.txt"), b"seed").unwrap();

        let base_path = dir.path().join("base");
        std::fs::create_dir_all(&base_path).unwrap();
        let base = BaseWorkspace::new(&base_path).unwrap();
        let task =
            TaskWorkspace::new_outside(&base, dir.path().join("task"), IntentId::new("i1"))
                .unwrap();

        // 在 `LandlockSandbox::new()` 的放行集合上追加 `ro`——**不能**把它整个换掉：
        // 只读集合也是子进程能加载动态库的原因，换掉后 `sh` 根本起不来，
        // 三条断言会以「夹具坏了」的形式一起失败。
        let mut sandbox = LandlockSandbox::new();
        sandbox.read_only_paths.push(ro.clone());
        let sandbox = Sandbox::Landlock(sandbox);

        // 1. 读得通：`ro` 确实在放行范围内，不是整体不可达。
        let status = spawn_sh(&sandbox, &task, &format!("cat {}/r.txt", ro.display()));
        assert!(
            status.success(),
            "只读路径内的文件应可读，实际 {status:?}——读不通说明本用例没测到「只读」"
        );

        // 2. 写不进：这就是 X1 的杀手，也是「只读放行路径真的是只读」的直接证据。
        //    取 `from_read` 换成 `from_all` 时变红的正是这一条。
        let status = spawn_sh(&sandbox, &task, &format!("echo bad > {}/w.txt", ro.display()));
        assert!(
            !status.success(),
            "只读路径内不得可写，实际 {status:?}"
        );
        assert!(
            !ro.join("w.txt").exists(),
            "只读路径内不得出现该文件"
        );

        // 3. 对照臂：写权限仍然只开在 Task 根上，前两条不是因为沙箱一律拒写。
        let status = spawn_sh(
            &sandbox,
            &task,
            &format!("echo ok > {}/t.txt", task.root().display()),
        );
        assert!(
            status.success(),
            "对照臂：Task 内写入应成功，实际 {status:?}"
        );
    }

    /// 用 `Sandbox::spawn` 起 `sh -c <脚本>`，等待结束后返回退出状态。
    ///
    /// 与 `tests/isolation.rs` 里的同名辅助函数是两份：那是集成测试、这是 crate 内单测，
    /// 两者不能互相引用。刻意都留着而不是抽到公共模块——它是测试夹具，不是产品代码。
    fn spawn_sh(sandbox: &Sandbox, task: &TaskWorkspace, script: &str) -> std::process::ExitStatus {
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(script);
        sandbox
            .spawn(task, cmd)
            .unwrap_or_else(|e| panic!("sandbox.spawn 失败：{e}"))
            .wait()
            .expect("等待子进程失败")
    }
}
