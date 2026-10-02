//! Landlock 机制：ABI 探测、能力推导与规则集构造。
//!
//! 本模块只做「构造」。规则集的**施加**在 `pre_exec` 闭包里，见
//! [`crate::Sandbox::spawn`]：闭包运行在 fork 之后、exec 之前，其中不得分配内存，
//! 故规则集必须在此之前构造完毕。

use crate::sandbox::{LandlockSandbox, SandboxCapabilities};
use landlock::{
    Access, AccessFs, PathBeneath, PathFd, Ruleset, RulesetAttr, RulesetCreated,
    RulesetCreatedAttr, RulesetError, ABI,
};
use std::path::{Path, PathBuf};

/// 子进程只读放行的路径。
///
/// 不足以加载动态库时子进程根本起不来——那会让**对照臂**也失败，而不是让实验臂
/// 通过，故这里漏项的后果是可发现的。`/lib`、`/lib64`、`/bin`、`/sbin` 在多数发行版
/// 上是指向 `/usr` 的符号链接，`PathFd::new` 以 `O_PATH` 打开并跟随符号链接，
/// 故留着它们不会出错，而某些发行版（如 NixOS）确实把它们放在别处。
const READ_ONLY_PATHS: &[&str] = &["/usr", "/lib", "/lib64", "/etc", "/bin", "/sbin"];

/// 放行全部访问权限的设备文件。
///
/// 这些是常见设备，程序（含 shell 与 libc）会打开它们。放行的是**文件**权限，
/// 且仅限这些具名设备本身，不扩及其所在目录。
const READ_WRITE_DEVICE_PATHS: &[&str] = &[
    "/dev/null",
    "/dev/zero",
    "/dev/full",
    "/dev/random",
    "/dev/urandom",
    "/dev/tty",
];

/// `LANDLOCK_CREATE_RULESET_VERSION`：`landlock_create_ruleset` 的 `flags` 取值，
/// 表示「不建规则集，只回报内核的 Landlock ABI 版本」。
///
/// 该常量在 `landlock` crate 的 uapi 模块里，而那个模块不公开，故此处按 uapi 头
/// 固定值重复一次。
const LANDLOCK_CREATE_RULESET_VERSION: libc::c_uint = 1;

/// 查询本机内核的 Landlock ABI 版本；不可用时返回 `0`。
///
/// 这是设计第 4.3 节规定的父进程侧探测入口，也是本 crate 唯一能判定 Landlock 能力的
/// 手段：`landlock` crate 的 `Ruleset::create()` 在不支持的内核上返回的是**假的 `Ok`**
/// （一个没有 fd 的 `RulesetCreated`）而非错误，`RulesetStatus` 又只能在被限制的进程内
/// 取得，而子进程是调用方给定的任意命令、无法回传。
///
/// 用 `libc` 而非裸 syscall：本调用与 `landlock` crate 自己的 uapi 层是同一个
/// `syscall(SYS_landlock_create_ruleset, …)`，用同一个 crate 走同一条路能保证行为一致；
/// 裸 syscall 要么写 `asm!`（须自行处理各架构的寄存器约定与 `SYS_*` 编号，是纯粹的正确性
/// 风险），要么自己 `extern "C"` 声明 `syscall` 并硬编码常量，等于重抄一份 libc 而更差。
/// 且 `libc` 0.2 已作为 `landlock` 的依赖在锁文件里，登记它不引入新的 crate。
fn probe_abi() -> i32 {
    // SAFETY: 参数与内核约定一致——`attr` 为 NULL、`size` 为 0 时内核不读写任何内存；
    // 带 VERSION 标志时该调用不创建对象、不产生副作用，只返回一个整数。返回值 < 0 表示
    // 失败（EOPNOTSUPP / ENOSYS），此时不读 errno 而直接当作「不可用」。
    let ret = unsafe {
        libc::syscall(
            libc::SYS_landlock_create_ruleset,
            std::ptr::null::<libc::c_void>(),
            0usize,
            LANDLOCK_CREATE_RULESET_VERSION,
        )
    };
    if ret < 0 { 0 } else { ret as i32 }
}

/// 由 ABI 版本推导「该机制将生效」的隔离项。
///
/// 抽成纯函数是为了可单测：降级路径在本机内核（ABI 已足够高）上不可达，用环境变量
/// 之类的手段模拟 ABI 不足只会得到一条「测环境变量」的用例，而非测降级的用例。
///
/// 取值口径与 [`SandboxCapabilities`] 的约定一致——报告的是**本配置**将生效的隔离项，
/// 不是机制「本来能做什么」：
///
/// - `restricts_filesystem_writes`：ABI v1 起即支持文件系统访问类别，故 `abi >= 1` 为真；
///   `0`（内核未启用或不支持 Landlock）为假。
/// - `isolates_network`：恒为假。ABI v4 起内核**能**限制 TCP bind/connect，但本 crate 的
///   规则集不 handle 网络访问，故网络实际不受限——不得因为「内核支持」而报真。
/// - `isolates_pid`：恒为假。Landlock 是路径访问控制，不是命名空间机制，没有任何 ABI
///   版本能隔离 PID。
pub(crate) fn capabilities_for_abi(abi: i32) -> SandboxCapabilities {
    SandboxCapabilities {
        restricts_filesystem_writes: abi >= ABI::V1 as i32,
        isolates_network: false,
        isolates_pid: false,
        mechanism: "landlock",
    }
}

/// ABI 探测结果到「要限制的访问类别」的映射；内核不支持 Landlock 时为 `None`。
///
/// `None` 是**降级路径**（设计第 4.3 节）：不构造规则集、不施加隔离，也**不使启动失败**
/// ——本条按 [`crate::SandboxError`] 中「Landlock 的内核 ABI 不足不属 MechanismUnavailable，
/// 那走降级路径，由 SandboxCapabilities 如实反映」的既定口径实现。
///
/// 抽成纯函数是为了可单测：本机内核支持 Landlock，这条路径在行为用例里不可达，
/// 而「不使启动失败」是个绝对措辞，必须有对应用例。
///
/// 高于本 crate 已知上限的 ABI（例如内核算出 v10）由 `ABI::from` 钳到最高已知版本
/// ——本 crate 表达不出的访问类别也就无从 handle。
pub(crate) fn effective_abi(abi: i32) -> Option<ABI> {
    if abi >= ABI::V1 as i32 {
        Some(ABI::from(abi))
    } else {
        None
    }
}

/// 滤出实际存在的路径。
///
/// 不存在的路径交给 `path_beneath_rules` 只会被静默丢弃（它以 `filter_map` 吞掉打开
/// 失败的项），在构造期先滤一次，`LandlockSandbox` 里存的才是**真的会放行**的集合。
fn existing_paths(candidates: &[&str]) -> Vec<PathBuf> {
    candidates
        .iter()
        .map(PathBuf::from)
        .filter(|p| p.exists())
        .collect()
}

impl LandlockSandbox {
    /// 探测内核并固定放行路径集合。
    ///
    /// 探测在**构造期**做一次而非每次 spawn 做：`capabilities()` 与 `spawn()` 必须依据
    /// 同一个 ABI 判定，否则两者可能对不上（内核不会变，但读取的时机可以不同）。
    pub(crate) fn new() -> Self {
        LandlockSandbox {
            abi: probe_abi(),
            read_only_paths: existing_paths(READ_ONLY_PATHS),
            read_write_paths: existing_paths(READ_WRITE_DEVICE_PATHS),
        }
    }

    /// 内核 ABI 探测结果；`0` 表示不可用。
    pub(crate) fn abi(&self) -> i32 {
        self.abi
    }

    /// 构造规则集：默认拒绝，放行只读路径与设备，写权限只开 Task 根。
    ///
    /// 必须在 `pre_exec` **之前**调用——闭包里不得分配内存，而 `handle_access`、
    /// `PathFd::new`、`add_rules` 都会分配。
    ///
    /// 返回 `Ok(None)` 表示降级：内核 ABI 为 0，本次不施加任何隔离（见 [`effective_abi`]）。
    /// 返回 `Err` 才是本层在父进程侧的失败。
    ///
    /// 失败以说明文字返回而非 `RulesetError`：`PathFdError` 到 `RulesetError` 的转换
    /// 在本 crate 里不存在（`path_beneath_rules` 干脆以 `filter_map` 吞掉打不开的路径，
    /// 从不产生 `Err`），逐段折成文字反而能把「哪一步失败」带出去。唯一消费方
    /// [`crate::Sandbox::spawn`] 本就要把它包进
    /// [`crate::SandboxError::IsolationFailed`] 的 `reason`。
    pub(crate) fn build_ruleset(
        &self,
        task_root: &Path,
    ) -> Result<Option<RulesetCreated>, String> {
        let Some(abi) = effective_abi(self.abi) else {
            return Ok(None);
        };
        let all = AccessFs::from_all(abi);
        let read = AccessFs::from_read(abi);

        let ruleset = Ruleset::default()
            .handle_access(all)
            .map_err(|e| format!("声明要限制的访问类别失败：{e}"))?
            .create()
            .map_err(|e| format!("创建规则集失败：{e}"))?;
        let ruleset = ruleset
            .add_rules(landlock::path_beneath_rules(&self.read_only_paths, read))
            .map_err(|e| format!("放行只读路径失败：{e}"))?;
        let ruleset = ruleset
            .add_rules(landlock::path_beneath_rules(&self.read_write_paths, all))
            .map_err(|e| format!("放行设备文件失败：{e}"))?;

        // Task 根**单独**构造而不走 `path_beneath_rules`：那条路会静默丢弃打不开的路径，
        // 而 Task 根打不开时必须报错——那意味着本次启动一个写权限都不放行，子进程在
        // 被隔离的名义下什么也做不了，属于本层在父进程侧的失败。
        let task_fd = PathFd::new(task_root)
            .map_err(|e| format!("打开 Task 根 {} 失败：{e}", task_root.display()))?;
        ruleset
            .add_rules([Ok::<_, RulesetError>(PathBeneath::new(task_fd, all))])
            .map(Some)
            .map_err(|e| format!("放行 Task 根失败：{e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 低 ABI 与高 ABI 各断言一次——这是降级路径唯一的测试缝隙。
    #[test]
    fn capabilities_follow_the_probed_abi() {
        // 内核未启用或不支持 Landlock：文件系统写入不受限。
        assert!(!capabilities_for_abi(ABI::Unsupported as i32).restricts_filesystem_writes);
        // v1 起即支持文件系统访问类别。
        assert!(capabilities_for_abi(ABI::V1 as i32).restricts_filesystem_writes);
        // 更高的 ABI 只增不减。
        assert!(capabilities_for_abi(ABI::V9 as i32).restricts_filesystem_writes);
        // 高于本 crate 已知上限的 ABI 同样为真。
        assert!(capabilities_for_abi(99).restricts_filesystem_writes);
    }

    /// 网络与 PID 与本配置无关，任何 ABI 下都为假——包括内核「本来能」限制网络的 v4 以上。
    #[test]
    fn network_and_pid_are_never_reported_as_isolated() {
        for abi in [0, 1, 4, 9, 99] {
            let caps = capabilities_for_abi(abi);
            assert!(!caps.isolates_network, "abi={abi}：本配置不 handle 网络");
            assert!(!caps.isolates_pid, "abi={abi}：Landlock 不是命名空间机制");
            assert_eq!(caps.mechanism, "landlock");
        }
    }

    /// 降级路径：内核 ABI 为 0 时不构造规则集（因而不施加隔离），也不报错。
    ///
    /// 与 [`capabilities_for_abi`] 必须一致：报「不限制写入」的那个 ABI，
    /// 正是这里返回 `None`（不构造规则集）的那个 ABI。
    #[test]
    fn unsupported_abi_degrades_instead_of_failing() {
        assert_eq!(effective_abi(ABI::Unsupported as i32), None);
        assert_eq!(effective_abi(-1), None);
        assert!(!capabilities_for_abi(0).restricts_filesystem_writes);

        assert_eq!(effective_abi(ABI::V1 as i32), Some(ABI::V1));
        assert_eq!(effective_abi(ABI::V9 as i32), Some(ABI::V9));
        // 高于本 crate 已知上限的 ABI 钳到最高已知版本，而非视作不可用。
        assert_eq!(effective_abi(99), Some(ABI::V9));
    }

    /// 夹具自检：只读放行集合里的路径若全都不存在，规则集等于什么都没放行，
    /// 对照臂会以「子进程起不来」的形式失败，而失败位置指向夹具而非实现。
    /// 本机至少应有 `/usr`，否则环境不满足本 crate 的前提。
    #[test]
    fn read_only_paths_are_non_empty_on_this_machine() {
        assert!(
            !existing_paths(READ_ONLY_PATHS).is_empty(),
            "本机没有任何只读放行路径存在，沙箱子进程无法加载动态库"
        );
    }
}
