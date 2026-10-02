//! bubblewrap 机制：可执行文件定位与参数表构造。
//!
//! 与 `landlock` 模块的分工相同——本模块只做「构造」，施加由 bwrap 进程自己在
//! [`crate::Sandbox::spawn`] 里完成（bwrap 先建挂载命名空间再 exec 调用方的命令，
//! 故不存在 Landlock 那种「fork 之后、exec 之前」的窗口，也没有闭包内不得分配内存的约束）。
//!
//! **保留该后端的理由是文件系统这一项上的机制独立性，不是它的网络与 PID 能力。**
//! 内核没有 Landlock 时它走挂载命名空间仍然可用；本子项目的参数集不含
//! `--unshare-net` / `--unshare-pid`，那两项能力在本配置下不生效，故
//! [`crate::Sandbox::capabilities`] 对它们报告为假。

use crate::sandbox::BubblewrapSandbox;
use continuum_workspace::TaskWorkspace;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Command;

/// 在 `PATH` 中查找的可执行文件名。
const BWRAP_PROGRAM: &str = "bwrap";

/// 构造 `bwrap` 的完整参数表：加固参数在前，调用方的命令原样接在 `--` 之后。
///
/// 抽成纯函数是为了可单测：参数表本身能被逐项断言，不必每次真起进程。这一点在本 crate
/// 不是可选的谨慎——**加固参数少写一项即等于未隔离**，而少写一项时行为用例是否变红取决于
/// 少的是哪一项（见下表最后一列，逐项都实测过）。
///
/// 下表第三列是**逐项变异实测**的结果（删掉该项后跑全量 `--no-fail-fast` 得到的失败点）：
///
/// | 参数 | 作用 | 删掉它时变红的用例 |
/// | --- | --- | --- |
/// | `--ro-bind / /` | 把宿主根整个只读地挂进来 | `denies_writes_to_base_and_allows_writes_to_task` 的实验臂（写 Base 会成功）；另有三条用例与两条参数表单测 |
/// | `--bind <task> <task>` | 把 Task 根挂回可写 | 同名用例的对照臂（Task 被盖成只读，子进程什么也写不了）；另同上 |
/// | `--dev /dev` | 挂一份新 devtmpfs | 同名用例的 `/dev/null` 臂（写成宿主 `/dev` 的只读 bind，以 `Permission denied` 失败） |
/// | `--chdir <task>` | 子进程工作目录为 Task 根（设计第 4.2 节） | `the_child_working_directory_is_the_task_root` |
/// | `--proc /proc` | 挂一份新 procfs，使 `/proc` 属于本挂载命名空间 | **仅** `argv_pins_every_hardening_flag_in_order`——无行为用例，见表下 |
/// | `--` | 终止 bwrap 自己的选项解析 | **仅** `argv_pins_every_hardening_flag_in_order`——无行为用例，见表下 |
///
/// 后两项**只被参数表单测钉住，没有任何行为用例能钉**。两者原因不同，都不是「还没来得及写」：
///
/// - **`--`**：必要性是实测的——`bwrap … -weird` 报 `Unknown option -weird`（bwrap 把调用方
///   的命令当成了自己的选项）。但本 crate 的用例都以 `Command::new("sh")` 起子进程，argv
///   不以 `-` 开头，故删掉 `--` 时没有任何**行为**用例变红；变红的只有断言整表的
///   `argv_pins_every_hardening_flag_in_order`。保留它是为调用方给一个以 `-` 开头的程序名时
///   仍能落到「exec 失败」而不是「bwrap 选项解析失败」。
/// - **`--proc /proc`**：实测删掉它**换不来任何可判定的行为差异**——`--ro-bind / /` 已经
///   递归地把宿主 `/proc` 带进来了，两种形态下子进程都能读 `/proc`，那条命名空间断言在
///   两种形态下都成立（网络与 PID 命名空间本就与宿主相同）。两者仅有的差别是
///   `/proc/self/mounts` 的条目数（挂载布局的细节，随 bwrap 版本变），不足以为据。
///   故这一项按设计参数集保留，但**本 crate 不声称验证过它的行为**，只钉住它出现在表里。
///
/// **顺序是参数表的一部分，不是排版**：`--ro-bind / /` 必须排在 `--bind <task> <task>` 之前。
/// bwrap 按出现顺序执行挂载，后挂的覆盖先挂的；反过来的话 Task 根先被挂成可写、再被
/// `--ro-bind / /` 整个盖成只读，两条臂会一起变红。
///
/// 参数表**不含** `--unshare-net` / `--unshare-pid`：本子项目不使用这两项能力。这是
/// [`crate::SandboxCapabilities`] 对它们报告为假的直接原因，两者必须一致。
///
/// 调用方的 `env` 不在此处处理——它在 `spawn` 里经 `Command::get_envs` 逐个重放，
/// 因为那个动作需要一个可变的 `Command`。**stdio 是唯一无法保留的一项**：`Command`
/// 不提供 stdin / stdout / stderr 的读取接口（rustc 1.95.0 实测无 `get_stdout`）。
pub(crate) fn bwrap_argv(task: &TaskWorkspace, cmd: &Command) -> Vec<OsString> {
    let root = task.root();
    let mut argv: Vec<OsString> = Vec::with_capacity(cmd.get_args().len() + 14);

    argv.push("--ro-bind".into());
    argv.push("/".into());
    argv.push("/".into());

    argv.push("--bind".into());
    argv.push(root.into());
    argv.push(root.into());

    argv.push("--dev".into());
    argv.push("/dev".into());

    argv.push("--proc".into());
    argv.push("/proc".into());

    argv.push("--chdir".into());
    argv.push(root.into());

    argv.push("--".into());
    argv.push(cmd.get_program().into());
    argv.extend(cmd.get_args().map(OsString::from));

    argv
}

/// 在 `PATH` 的值中定位 `bwrap`；找不到返回 `None`。
///
/// 收的是 `PATH` 的**值**而不是从环境里自己读，为的是可单测：`set_var` 在 edition 2024
/// 是 `unsafe` 且进程级，测试并发修改 `PATH` 会波及其他用例起 `sh` 的动作。取值由调用方
/// [`BubblewrapSandbox::new`] 在构造期做一次。
///
/// 判据是「普通文件且带执行位」。**不用 `Command::new("bwrap").output()` 试跑**：那会在
/// 判定「机制是否可用」这件本应无副作用的事上启动进程，而设计第 4.3 节要求该判定发生在
/// 启动任何进程之前。执行位是当前进程视角的 mode 位，与 `access(X_OK)` 在 euid 与
/// 属主一致时同结论；属主不一致时本函数偏严（报不可用），方向是拒绝启动。
pub(crate) fn locate_bwrap(path_var: Option<&OsStr>) -> Option<PathBuf> {
    let value = path_var?;
    std::env::split_paths(value)
        .map(|dir| dir.join(BWRAP_PROGRAM))
        .find(|candidate| is_executable_file(candidate))
}

/// 是普通文件且对当前进程带执行位。
///
/// 符号链接由 `metadata`（而非 `symlink_metadata`）跟随——`PATH` 里的 `bwrap` 是指向真身的
/// 符号链接是常见形态。
fn is_executable_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

impl BubblewrapSandbox {
    /// 在构造期定位一次 `bwrap` 并固定结果。
    ///
    /// 探测在**构造期**做一次而非每次 spawn 做：`capabilities()` 与 `spawn()` 必须依据
    /// 同一个判定，否则两者可能对不上。这与 `LandlockSandbox::new` 的 ABI 探测是同一个
    /// 理由。
    pub(crate) fn new() -> Self {
        BubblewrapSandbox {
            bwrap: locate_bwrap(std::env::var_os("PATH").as_deref()),
        }
    }

    /// 定位到的 `bwrap` 可执行文件；`None` 表示本机不存在该机制。
    pub(crate) fn bwrap(&self) -> Option<&Path> {
        self.bwrap.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use continuum_workspace::{BaseWorkspace, IntentId};

    /// 参数表逐项断言：加固参数一个不少、顺序正确、调用方命令原样接在 `--` 之后。
    ///
    /// 用整表相等而不是「包含某几项」：包含式断言对**重复**与**顺序**都无感，
    /// 而本表的顺序是有语义的（见 [`bwrap_argv`] 文档）。
    #[test]
    fn argv_pins_every_hardening_flag_in_order() {
        let dir = tempfile::tempdir().unwrap();
        let base_path = dir.path().join("base");
        std::fs::create_dir_all(&base_path).unwrap();
        let base = BaseWorkspace::new(&base_path).unwrap();
        let task =
            TaskWorkspace::new_outside(&base, dir.path().join("task"), IntentId::new("i1"))
                .unwrap();

        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg("echo hi");

        let root = task.root();
        let expected: Vec<OsString> = vec![
            "--ro-bind".into(),
            "/".into(),
            "/".into(),
            "--bind".into(),
            root.into(),
            root.into(),
            "--dev".into(),
            "/dev".into(),
            "--proc".into(),
            "/proc".into(),
            "--chdir".into(),
            root.into(),
            "--".into(),
            "sh".into(),
            "-c".into(),
            "echo hi".into(),
        ];

        assert_eq!(
            bwrap_argv(&task, &cmd),
            expected,
            "bwrap 参数表与本用例钉住的形状不符"
        );
    }

    /// 只读根挂载必须排在 Task 可写挂载之前——bwrap 后挂的覆盖先挂的，
    /// 反过来 Task 根会被盖成只读，两条臂一起变红。
    ///
    /// 单独一条是因为它是**顺序**约束，整表相等的用例变红时看不出红在顺序还是红在内容。
    #[test]
    fn the_read_only_root_bind_precedes_the_task_bind() {
        let dir = tempfile::tempdir().unwrap();
        let base_path = dir.path().join("base");
        std::fs::create_dir_all(&base_path).unwrap();
        let base = BaseWorkspace::new(&base_path).unwrap();
        let task =
            TaskWorkspace::new_outside(&base, dir.path().join("task"), IntentId::new("i1"))
                .unwrap();

        let argv = bwrap_argv(&task, &Command::new("true"));
        let ro = argv
            .iter()
            .position(|a| a == "--ro-bind")
            .expect("参数表里没有 --ro-bind：宿主根都没挂进来，限制不到任何路径");
        let rw = argv
            .iter()
            .position(|a| a == "--bind")
            .expect("参数表里没有 --bind：Task 根会被只读根挂载盖住，子进程什么也写不了");
        assert!(
            ro < rw,
            "--ro-bind 必须排在 --bind 之前，实际位置 {ro} vs {rw}"
        );
    }

    /// 参数表不含网络与 PID 命名空间开关——能力报告对这两项报假的依据就在这里。
    ///
    /// 与 `Sandbox::capabilities` 的 bubblewrap 分支是同一件事的两侧：那边报假、这边不传，
    /// 任一侧单独改动都会被本用例或那条断言之一抓住。
    #[test]
    fn argv_does_not_unshare_network_or_pid_namespaces() {
        let dir = tempfile::tempdir().unwrap();
        let base_path = dir.path().join("base");
        std::fs::create_dir_all(&base_path).unwrap();
        let base = BaseWorkspace::new(&base_path).unwrap();
        let task =
            TaskWorkspace::new_outside(&base, dir.path().join("task"), IntentId::new("i1"))
                .unwrap();

        let argv = bwrap_argv(&task, &Command::new("true"));
        for forbidden in ["--unshare-net", "--unshare-pid", "--unshare-all"] {
            assert!(
                !argv.iter().any(|a| a == forbidden),
                "参数表出现了 {forbidden}：能力报告说这两项不隔离，参数表却隔离了，两者对不上"
            );
        }
    }

    /// `PATH` 里没有 `bwrap` 时定位失败——这是 `MechanismUnavailable` 的判据。
    #[test]
    fn locate_bwrap_reports_absence() {
        let dir = tempfile::tempdir().unwrap();
        let empty = dir.path().join("empty");
        std::fs::create_dir_all(&empty).unwrap();

        // 目录存在但没有 bwrap
        assert_eq!(locate_bwrap(Some(empty.as_os_str())), None);
        // `PATH` 整个不存在（环境里没有该变量）
        assert_eq!(locate_bwrap(None), None);
        // 同名但不可执行
        let plain = dir.path().join("plain");
        std::fs::create_dir_all(&plain).unwrap();
        std::fs::write(plain.join("bwrap"), b"not a program").unwrap();
        assert_eq!(
            locate_bwrap(Some(plain.as_os_str())),
            None,
            "无执行位的同名文件不得被当作 bwrap"
        );
        // 同名且是目录
        let dirnamed = dir.path().join("dirnamed");
        std::fs::create_dir_all(dirnamed.join("bwrap")).unwrap();
        assert_eq!(
            locate_bwrap(Some(dirnamed.as_os_str())),
            None,
            "同名目录不得被当作 bwrap"
        );
    }

    /// 定位按 `PATH` 的顺序取第一个可执行者，前面的目录里没有才轮到后面的。
    #[test]
    fn locate_bwrap_takes_the_first_hit_in_path_order() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let first = dir.path().join("first");
        let second = dir.path().join("second");
        std::fs::create_dir_all(&first).unwrap();
        std::fs::create_dir_all(&second).unwrap();
        std::fs::write(second.join("bwrap"), b"x").unwrap();
        std::fs::set_permissions(second.join("bwrap"), std::fs::Permissions::from_mode(0o755))
            .unwrap();

        let path_var = std::env::join_paths([&first, &second]).unwrap();
        assert_eq!(
            locate_bwrap(Some(&path_var)),
            Some(second.join("bwrap")),
            "第一个目录里没有，应落到第二个目录"
        );

        std::fs::write(first.join("bwrap"), b"x").unwrap();
        std::fs::set_permissions(first.join("bwrap"), std::fs::Permissions::from_mode(0o755))
            .unwrap();
        assert_eq!(
            locate_bwrap(Some(&path_var)),
            Some(first.join("bwrap")),
            "两个目录都有时应取 PATH 中靠前的那个"
        );
    }
}
