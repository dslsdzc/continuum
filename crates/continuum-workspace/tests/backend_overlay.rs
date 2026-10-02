//! OverlayFS 后端的往返测试。
//!
//! 无特权 OverlayFS 的挂载只在其所在的用户与挂载命名空间内可见，故本文件的用例要真正
//! 挂载，就必须先处在这样一个命名空间里。测试进程本身不在其中，因此每个要挂载的用例都
//! 把自身重新执行进 `unshare -Urm`，由子进程完成断言；父进程只核对子进程的退出码。
//!
//! 环境不具备（`unshare` 缺失或用户命名空间不可用）时，用例**显式跳过并在 stderr 上
//! 标记**，不静默通过——见 [`skip`]。
//!
//! 路径一律取 `canonicalize` 之后的值：临时目录可能落在符号链接之下，而实现比较的是
//! 规范路径，两侧不规范化会让失败点指向断言而不是实现。

use continuum_workspace::{
    BaseWorkspace, IntentId, TaskWorkspace, WorkspaceBackend, WorkspaceError, create_task_workspace,
    discard_task_workspace, in_user_namespace,
};
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Command;

/// 标识「本进程是被重新执行出来的子进程」，防止再次 re-exec 造成无限递归。
const CHILD_ENV: &str = "CONTINUUM_OVERLAY_CHILD";

/// overlay 根：由父进程指到一个只属于本用例的临时目录，子进程据此建立工作区。
const OVERLAY_ROOT_ENV: &str = "CONTINUUM_OVERLAY_ROOT";

/// 子进程要用的 Base 路径。只有「overlay 根落在 Base 之内」那条用例需要父进程预先建好
/// Base（两条路径必须同源），故经环境变量传递。
const TEST_BASE_ENV: &str = "CONTINUUM_OVERLAY_TEST_BASE";

fn is_child() -> bool {
    std::env::var_os(CHILD_ENV).is_some()
}

/// 环境不具备本用例所需条件时的**显式**跳过标记。
///
/// 不静默通过：跳过原因打到 stderr。cargo 在用例通过时不回显这段输出，用 `--nocapture`
/// 运行即可见——这是稳定工具链上既不与「全量全绿」冲突、又留下痕迹的处置。改判为
/// `panic!` 会把「环境不具备」误报成「实现有缺陷」，反而掩盖真正的问题。
fn skip(test: &str, reason: &str) {
    eprintln!("【跳过】{test}：{reason}。本用例未执行断言。");
}

/// 环境是否具备建立用户与挂载命名空间的能力。
///
/// 探测方式是把**本测试二进制**以 `--list` 重新执行进 `unshare -Urm`：能起来即说明
/// 命名空间可建立。用自身二进制而非 `/bin/true` 一类的替身，是因为要探的正是
/// 「本测试进程能否被这样执行」——替身探不出动态加载等本进程的实际依赖。
fn namespace_available(exe: &str) -> bool {
    Command::new("unshare")
        .args(["-Urm", exe, "--list"])
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

/// 把本用例重新执行进一个子进程，并核对子进程的退出码。
///
/// 返回 `true` 表示「当前就是子进程，继续执行断言」；`false` 表示「父进程已跑完子进程
/// 并核对过结果，或环境不具备已显式跳过」。`namespace` 为真时经 `unshare -Urm` 进入用户
/// 与挂载命名空间，为假时直接重新执行（用于只需隔离环境变量、不需挂载的用例）。
fn reexec(test_name: &str, envs: &[(&str, &OsStr)], namespace: bool) -> bool {
    if is_child() {
        return true;
    }
    let exe = std::env::current_exe().expect("无法取到测试可执行文件的路径");
    let exe = exe
        .to_str()
        .expect("测试可执行文件的路径不是合法 UTF-8")
        .to_owned();

    let mut command = if namespace {
        if !namespace_available(&exe) {
            skip(test_name, "unshare -Urm 无法建立用户与挂载命名空间");
            return false;
        }
        let mut command = Command::new("unshare");
        command.args(["-Urm", &exe, test_name, "--exact"]);
        command
    } else {
        let mut command = Command::new(&exe);
        command.args([test_name, "--exact"]);
        command
    };
    // 只跑这一条：子进程里的用例函数会因为 `CHILD_ENV` 直接进入断言，不再 re-exec。
    command.env(CHILD_ENV, "1");
    for (key, value) in envs {
        command.env(key, value);
    }

    let out = command
        .output()
        .unwrap_or_else(|e| panic!("无法重新执行 {exe}：{e}"));
    assert!(
        out.status.success(),
        "子进程 {test_name} 失败（退出码 {:?}）：\n--- stdout ---\n{}\n--- stderr ---\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    false
}

/// 需要用户与挂载命名空间的用例的入口。
///
/// 父进程把自身经 `unshare -Urm` 重新执行，并把 `CONTINUUM_OVERLAY_ROOT` 指到一个只属于
/// 本次用例的临时目录——绝不能落到默认的 `~/.local/share/continuum/overlays`，否则用例会
/// 污染用户的 home。
fn enter_namespace(test_name: &str, extra_envs: &[(&str, &OsStr)]) -> bool {
    if is_child() {
        return true;
    }
    let holder = tempfile::tempdir().unwrap();
    let root: OsString = holder.path().join("overlays").into_os_string();
    let mut envs: Vec<(&str, &OsStr)> = vec![(OVERLAY_ROOT_ENV, &root)];
    envs.extend_from_slice(extra_envs);
    let child = reexec(test_name, &envs, true);
    // 子进程已退出，本次用例的 overlay 存储随临时目录一并清掉
    drop(holder);
    child
}

fn overlay_root() -> PathBuf {
    PathBuf::from(
        std::env::var_os(OVERLAY_ROOT_ENV)
            .expect("子进程应当拿到 CONTINUUM_OVERLAY_ROOT；父进程见 enter_namespace"),
    )
}

/// 建一个带 lower 内容的 Base，返回（保活用的 `TempDir`，规范化的 Base 根）。
///
/// 只在子进程里调用。
fn base_with_lower() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    std::fs::write(root.join("f.txt"), b"lower").unwrap();
    std::fs::write(root.join("only_lower.txt"), "只在 lower\n".as_bytes()).unwrap();
    (dir, root)
}

/// `dir` 下的一级条目名（排序后）。目录不存在时返回空。
fn dir_entries(dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// `root` 之下全部条目的相对路径（排序后，含目录自身）。
///
/// 用于「某处不该出现某物」这类断言。只查一两个固定位置会漏掉泄漏藏在别处的情形：
/// 比如工作目录落在 Base 内时，别人的未提交工作出现在 `.ai/overlays/<id>/<intent>/upper/`
/// 之下，而不是 Task 根的顶层。目录不存在时返回空。
fn tree_paths(root: &Path) -> Vec<PathBuf> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            out.push(path.strip_prefix(root).unwrap_or(&path).to_path_buf());
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                walk(root, &path, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out.sort();
    out
}

/// 本进程的有效 uid 是否为 0（root 无需用户命名空间即可挂载）。
fn running_as_root() -> bool {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| {
            status.lines().find_map(|line| {
                let rest = line.strip_prefix("Uid:")?;
                rest.split_whitespace().nth(1)?.parse::<u32>().ok()
            })
        })
        == Some(0)
}

#[test]
fn overlay_backend_round_trips() {
    if !enter_namespace("overlay_backend_round_trips", &[]) {
        return;
    }
    // 本用例的前提就是「已在命名空间内」；这条断言同时钉住 in_user_namespace 的判定——
    // 少了它，一个恒返回 true 的实现也能让本用例通过。
    assert!(
        in_user_namespace(),
        "在 unshare -Urm 的子进程里 in_user_namespace 应为真"
    );

    let (_d, base_path) = base_with_lower();
    let base = BaseWorkspace::new(&base_path).unwrap();
    let (task, backend) = create_task_workspace(&base, &IntentId::new("i1")).unwrap();
    assert_eq!(backend, WorkspaceBackend::Overlay);

    // 工作目录在 Base 之外：Base 内不得出现 .ai/ 或任何 overlay 存储
    assert!(
        !task.root().starts_with(&base_path),
        "overlay 工作区不得位于 Base 内：{}",
        task.root().display()
    );
    assert!(!base_path.join(".ai").exists(), "Base 内不得出现 .ai");

    // 只在 Base 里的文件经覆盖层透传
    assert_eq!(
        std::fs::read(task.root().join("only_lower.txt")).unwrap(),
        "只在 lower\n".as_bytes()
    );

    // 写 Task 中的同一文件：覆盖 lower，Base 一字不动
    let writable = task.writable_root();
    writable.write("f.txt", b"upper").unwrap();
    assert_eq!(writable.read("f.txt").unwrap(), b"upper");
    assert_eq!(
        std::fs::read(base_path.join("f.txt")).unwrap(),
        b"lower",
        "Task 内的写入穿到了 Base"
    );

    // 新增一个只在 Task 里的文件：Base 中不出现
    writable.write("新文件.txt", "新".as_bytes()).unwrap();
    assert!(
        !base_path.join("新文件.txt").exists(),
        "Task 新增的文件出现在 Base 中"
    );

    discard_task_workspace(&base, &task, backend).unwrap();
}

#[test]
fn overlay_does_not_leak_a_sibling_intent() {
    if !enter_namespace("overlay_does_not_leak_a_sibling_intent", &[]) {
        return;
    }
    let (_d, base_path) = base_with_lower();
    let base = BaseWorkspace::new(&base_path).unwrap();
    let (t1, _) = create_task_workspace(&base, &IntentId::new("i1")).unwrap();
    let (t2, _) = create_task_workspace(&base, &IntentId::new("i2")).unwrap();

    t2.writable_root()
        .write("i2_secret.txt", "i2 的未提交工作".as_bytes())
        .unwrap();
    assert_eq!(
        std::fs::read(t2.root().join("i2_secret.txt")).unwrap(),
        "i2 的未提交工作".as_bytes()
    );

    // i1 看不到 i2 的未提交工作。若工作目录落在 Base 内的 `.ai/overlays/` 之下，
    // lower 为整个 Base 时 i2 的 upper 层就会出现在 i1 的 lower 里。
    //
    // 遍历整棵树而不是只查 Task 根下的固定路径：泄漏出来的副本位于
    // `.ai/overlays/<Base 标识>/i2/upper/` 之下，顶层并无 `i2_secret.txt`，
    // 只查顶层会让本断言在这种泄漏下依然通过。
    let leaked: Vec<PathBuf> = tree_paths(t1.root())
        .into_iter()
        .filter(|p| p.to_string_lossy().contains("i2_secret"))
        .collect();
    assert!(
        leaked.is_empty(),
        "i1 的工作区里出现了 i2 的未提交工作：{leaked:?}"
    );
    // 也看不到任何 overlay 元数据目录：工作目录在 Base 之外，lower 里没有它们
    assert!(!t1.root().join(".ai").exists(), "i1 的工作区里出现了 .ai/");
    assert!(!base_path.join(".ai").exists(), "Base 内出现了 .ai/");
    // 对照：i1 仍能正常读到自己该看到的内容，本用例不是因为「什么都读不到」而通过
    assert_eq!(
        std::fs::read(t1.root().join("only_lower.txt")).unwrap(),
        "只在 lower\n".as_bytes()
    );
}

#[test]
fn overlay_discard_removes_the_workspace() {
    if !enter_namespace("overlay_discard_removes_the_workspace", &[]) {
        return;
    }
    let (_d, base_path) = base_with_lower();
    let base = BaseWorkspace::new(&base_path).unwrap();
    let (task, backend) = create_task_workspace(&base, &IntentId::new("i1")).unwrap();
    task.writable_root().write("f.txt", b"upper").unwrap();
    let root = task.root().to_path_buf();
    let storage = overlay_root();

    discard_task_workspace(&base, &task, backend).unwrap();

    assert!(!root.exists(), "Task 根仍在：{}", root.display());
    // Base 不受影响：Task 侧改过的文件仍是 lower 的内容
    assert_eq!(std::fs::read(base_path.join("f.txt")).unwrap(), b"lower");
    assert!(!base_path.join(".ai").exists(), "Base 内出现了 .ai/");
    // overlay 存储里不留本 Intent 的目录（该 Base 已无别的 Intent，故整个目录链都该空）
    assert_eq!(
        dir_entries(&storage),
        Vec::<String>::new(),
        "overlay 存储里留下了残留"
    );
}

#[test]
fn overlay_create_refuses_to_reuse_an_existing_intent_directory() {
    if !enter_namespace("overlay_create_refuses_to_reuse_an_existing_intent_directory", &[]) {
        return;
    }
    let (_d, base_path) = base_with_lower();
    let base = BaseWorkspace::new(&base_path).unwrap();
    let (task, _) = create_task_workspace(&base, &IntentId::new("i1")).unwrap();
    task.writable_root().write("f.txt", b"upper").unwrap();

    // 同一 Intent 再建一次：既有目录里的 upper 层是上一次留下的未提交工作，
    // 静默接续会让本次 Intent 看到不属于它的改动，故宁可见地失败。
    let err = create_task_workspace(&base, &IntentId::new("i1")).unwrap_err();
    assert!(
        matches!(err, WorkspaceError::BackendUnavailable { .. }),
        "期望 BackendUnavailable，得到 {err:?}"
    );
    assert!(
        err.to_string().contains(&overlay_root().to_string_lossy().to_string()),
        "错误里未给出冲突的目录位置：{err}"
    );

    // 拒绝没有动到已经建好的那个工作区
    assert_eq!(
        std::fs::read(task.root().join("f.txt")).unwrap(),
        b"upper",
        "既有的工作区被失败的创建破坏了"
    );
}

#[test]
fn the_task_sees_the_bases_ai_directory_through_the_overlay() {
    if !enter_namespace("the_task_sees_the_bases_ai_directory_through_the_overlay", &[]) {
        return;
    }
    // 本用例钉住的是**文档注释里写明**的一条可见性：lowerdir 是整个 Base，
    // 故 Base 内的 `.ai/` 会出现在 Task 中，本后端不做排除。
    let (dir, base_path) = base_with_lower();
    let _ = dir;
    std::fs::create_dir_all(base_path.join(".ai")).unwrap();
    std::fs::write(base_path.join(".ai/元数据.txt"), "基座元数据".as_bytes()).unwrap();

    let base = BaseWorkspace::new(&base_path).unwrap();
    let (task, _) = create_task_workspace(&base, &IntentId::new("i1")).unwrap();

    assert_eq!(
        std::fs::read(task.root().join(".ai/元数据.txt")).unwrap(),
        "基座元数据".as_bytes(),
        "文档注释承诺了 `.ai/` 在 Task 中可见"
    );
}

#[test]
fn overlay_create_outside_a_namespace_reports_it() {
    // 断言在子进程里做：下面「自称在命名空间内」那一支会真的建立覆盖层，overlay 根必须
    // 经环境变量指到临时目录（见 `enter_namespace`），而环境变量是进程级的，在用例进程内
    // 改动会与并行用例相扰。子进程不 unshare——本用例要观察的正是「本进程不在其中」。
    if !is_child() {
        let holder = tempfile::tempdir().unwrap();
        let root: OsString = holder.path().join("overlays").into_os_string();
        let envs: [(&str, &OsStr); 1] = [(OVERLAY_ROOT_ENV, &root)];
        if !reexec("overlay_create_outside_a_namespace_reports_it", &envs, false) {
            return;
        }
    }

    let dir = tempfile::tempdir().unwrap();
    let base = BaseWorkspace::new(dir.path()).unwrap();

    if in_user_namespace() {
        // 「自称在用户命名空间内」不能只是一句空话，否则一个恒返回 true 的判定会让本用例
        // 在这一行悄悄走掉、断言一次也不执行——本用例要钉的「不在时拒绝」永不被观察。
        // 复核的办法是照常建一次：真在内时挂载能成，假在内时 mount 会因缺 CAP_SYS_ADMIN
        // 失败并报成 CommandFailed。于是「自称在内」与「确实挂得上」在这里对上。
        match create_task_workspace(&base, &IntentId::new("i1")) {
            Ok((task, backend)) => {
                discard_task_workspace(&base, &task, backend).unwrap();
            }
            Err(e) => panic!("in_user_namespace 自称本进程已在用户命名空间内，但建不出覆盖层：{e}"),
        }
        return;
    }

    if running_as_root() {
        skip(
            "overlay_create_outside_a_namespace_reports_it",
            "本进程以 root 运行，无需用户命名空间即可挂载，故不会走到该分支",
        );
        return;
    }

    // 不在命名空间且非 root：create 返回 NotInNamespace，而不是静默建出对子进程不可见的工作区
    let err = create_task_workspace(&base, &IntentId::new("i1")).unwrap_err();
    assert!(
        matches!(err, WorkspaceError::NotInNamespace),
        "实际 {err:?}"
    );
    // 拒绝发生在落盘之前：Base 内与 overlay 存储里都不该多出东西
    assert!(!dir.path().join(".ai").exists(), "被拒的创建在 Base 里建出了 .ai/");
    assert_eq!(
        dir_entries(&overlay_root()),
        Vec::<String>::new(),
        "被拒的创建在 overlay 存储里留下了残留"
    );
}

#[test]
fn overlay_rejects_an_intent_that_escapes_the_workdir() {
    // intent 的路径逃逸判定与后端无关，且排在 overlay 的命名空间判定之前，
    // 故本用例不需要命名空间。
    let dir = tempfile::tempdir().unwrap();
    let base = BaseWorkspace::new(dir.path()).unwrap();
    for bad in ["../../x", "a/b", "../逃逸", "", ".", ".."] {
        let err = create_task_workspace(&base, &IntentId::new(bad)).unwrap_err();
        assert!(
            matches!(err, WorkspaceError::InvalidIntent { .. }),
            "Intent 标识 {bad:?} 未被拒绝：{err:?}"
        );
    }
}

#[test]
fn overlay_refuses_an_overlay_root_that_lies_inside_the_base() {
    // 本条要求「工作目录必须在 Base 之外」。两条路径必须同源，故由父进程把 Base 与
    // 落在其内的 overlay 根一并建好，经环境交给子进程；判定排在命名空间判定之前，
    // 所以这里不需要 unshare——但仍是重新执行，以免进程内改环境变量与并行用例相扰。
    if !is_child() {
        let holder = tempfile::tempdir().unwrap();
        let base = holder.path().join("base");
        std::fs::create_dir(&base).unwrap();
        let storage: OsString = base.join(".ai/overlays").into_os_string();
        let base_env: OsString = base.into_os_string();
        let envs: [(&str, &OsStr); 2] = [
            (OVERLAY_ROOT_ENV, &storage),
            (TEST_BASE_ENV, &base_env),
        ];
        if !reexec(
            "overlay_refuses_an_overlay_root_that_lies_inside_the_base",
            &envs,
            false,
        ) {
            return;
        }
    }

    let base_path = PathBuf::from(
        std::env::var_os(TEST_BASE_ENV).expect("子进程应当拿到父进程建好的 Base 路径"),
    );
    let base = BaseWorkspace::new(&base_path).unwrap();
    let err = create_task_workspace(&base, &IntentId::new("i1")).unwrap_err();
    assert!(
        matches!(err, WorkspaceError::BackendUnavailable { .. }),
        "期望 BackendUnavailable，得到 {err:?}"
    );
    assert!(
        err.to_string().contains("Base"),
        "错误未说明工作目录与 Base 的关系：{err}"
    );
    // 判定在落盘之前：被拒的配置没有在 Base 里建出任何目录
    assert!(
        !base_path.join(".ai").exists(),
        "被拒的配置在 Base 里建出了 .ai/"
    );
}

#[test]
fn a_failed_umount_leaves_the_workspace_in_place() {
    // 注入卸载失败：PATH 前置一个必败的假 `umount`。手法与 `a_failed_mount_leaves_no_workspace_behind`
    // 相同。要钉的是 `discard` 的次序承诺——**umount 未成功就绝不往下走**。
    let fake = tempfile::tempdir().unwrap();
    let fake_umount = fake.path().join("umount");
    std::fs::write(
        &fake_umount,
        "#!/bin/sh\necho '注入的 umount 失败' >&2\nexit 1\n",
    )
    .unwrap();
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&fake_umount, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let path_env = format!(
        "{}:{}",
        fake.path().display(),
        std::env::var("PATH").unwrap_or_default()
    );
    if !enter_namespace(
        "a_failed_umount_leaves_the_workspace_in_place",
        &[("PATH", OsStr::new(&path_env))],
    ) {
        return;
    }

    let (_d, base_path) = base_with_lower();
    let base = BaseWorkspace::new(&base_path).unwrap();
    let (task, backend) = create_task_workspace(&base, &IntentId::new("i1")).unwrap();
    task.writable_root().write("f.txt", b"upper").unwrap();
    let intent_dir = task.root().parent().unwrap().to_path_buf();

    let err = discard_task_workspace(&base, &task, backend).unwrap_err();
    // 断言错误就是 **umount 的**失败。这一条是会咬人的：若 umount 的失败被忽略而继续
    // `remove_dir_all`，挂载点仍在，删会以 `EBUSY` 失败，报出来的是 `IoFailed` 而非
    // `CommandFailed`——「次序承诺被打破」正是从这个变体差异上读出来的，不是无关的其它错误。
    match &err {
        WorkspaceError::CommandFailed { code, stderr, .. } => {
            assert_eq!(*code, 1, "退出码应原样带回：{err}");
            assert!(
                stderr.contains("注入的 umount 失败"),
                "stderr 应原样带回，实际 {stderr:?}"
            );
        }
        other => panic!("期望 umount 的 CommandFailed，得到 {other:?}"),
    }

    // 卸载未成功，工作区必须原样留着：目录仍在，覆盖层仍挂着且可用
    assert!(intent_dir.exists(), "umount 失败却把 Intent 目录删了");
    assert_eq!(
        std::fs::read(task.root().join("f.txt")).unwrap(),
        b"upper",
        "覆盖层已被拆掉"
    );
    assert!(!base_path.join(".ai").exists(), "Base 内出现了 .ai/");
}

#[test]
fn overlay_discard_keeps_a_sibling_intents_directory() {
    if !enter_namespace("overlay_discard_keeps_a_sibling_intents_directory", &[]) {
        return;
    }
    let (_d, base_path) = base_with_lower();
    let base = BaseWorkspace::new(&base_path).unwrap();
    let (t1, backend) = create_task_workspace(&base, &IntentId::new("i1")).unwrap();

    // 同一 Base 下另一个 Intent 的目录。**故意用普通目录而非第二个真挂载的工作区**：
    // 真挂载时 `<Base 标识>` 目录里会含一个挂载点，`remove_dir_all` 递归到它就以 `EBUSY`
    // 提前退出，变异未必变红（取决于 readdir 顺序）。本用例要钉的是「非空的 `<Base 标识>`
    // 目录不被递归删除」，普通目录才能确定性地命中。
    let base_dir = t1.root().parent().unwrap().parent().unwrap().to_path_buf();
    let sibling = base_dir.join("i2");
    std::fs::create_dir_all(sibling.join("upper")).unwrap();
    std::fs::write(sibling.join("upper/证据.txt"), "别的 Intent 的未提交工作".as_bytes())
        .unwrap();

    discard_task_workspace(&base, &t1, backend).unwrap();

    assert!(!t1.root().exists(), "被放弃的 Intent 目录仍在");
    assert!(
        sibling.join("upper/证据.txt").is_file(),
        "别的 Intent 的目录被连带删除了"
    );
    assert_eq!(
        dir_entries(&overlay_root()).len(),
        1,
        "<Base 标识> 目录仍有别的 Intent，不该被移除"
    );
}

#[test]
fn overlay_discard_refuses_a_root_that_is_not_an_overlay_layout() {
    // 调用方把别的后端（worktree）建出的 Task 根、或自建的路径，配上 Overlay 后端交给
    // `discard` 时，必须在 umount **之前**报「不是本后端的布局」。少了这一步就会走到
    // umount：那里的答复是「umount 失败」，把误用说成外部命令故障；而若该路径下当真挂了
    // 别的东西，umount 会成功，随后整删一个不属于本后端的目录。
    //
    // 本用例不需要命名空间：形态校验排在 umount 之前，故在无挂载能力的环境里也走得到。
    let holder = tempfile::tempdir().unwrap();
    let base_path = holder.path().join("base");
    std::fs::create_dir(&base_path).unwrap();
    let task_root = holder.path().join("task");
    std::fs::create_dir_all(&task_root).unwrap();
    std::fs::write(task_root.join("用户的文件.txt"), "别动".as_bytes()).unwrap();

    let base = BaseWorkspace::new(&base_path).unwrap();
    let task = TaskWorkspace::new_outside(&base, &task_root, IntentId::new("i1")).unwrap();

    let err = discard_task_workspace(&base, &task, WorkspaceBackend::Overlay).unwrap_err();
    assert!(
        matches!(err, WorkspaceError::BackendUnavailable { .. }),
        "期望 BackendUnavailable，得到 {err:?}"
    );
    assert!(
        err.to_string().contains("overlay 布局"),
        "错误未说明这不是本后端的布局：{err}"
    );
    // 误用的路径一字未动
    assert!(
        task_root.join("用户的文件.txt").is_file(),
        "误用的路径被删了：{}",
        task_root.display()
    );
}

#[test]
fn a_failed_mount_leaves_no_workspace_behind() {
    // 注入挂载失败：PATH 前置一个必败的假 `mount`。这是本文件唯一能确定性地走到
    // 「目录已建、挂载失败」这条回收路径的办法。
    let fake = tempfile::tempdir().unwrap();
    let fake_mount = fake.path().join("mount");
    std::fs::write(
        &fake_mount,
        "#!/bin/sh\necho '注入的 mount 失败' >&2\nexit 1\n",
    )
    .unwrap();
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&fake_mount, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let path_env = format!(
        "{}:{}",
        fake.path().display(),
        std::env::var("PATH").unwrap_or_default()
    );
    if !enter_namespace(
        "a_failed_mount_leaves_no_workspace_behind",
        &[("PATH", OsStr::new(&path_env))],
    ) {
        return;
    }

    let (_d, base_path) = base_with_lower();
    let base = BaseWorkspace::new(&base_path).unwrap();
    let err = create_task_workspace(&base, &IntentId::new("i1")).unwrap_err();
    match &err {
        WorkspaceError::CommandFailed { code, stderr, .. } => {
            assert_eq!(*code, 1, "退出码应原样带回：{err}");
            assert!(
                stderr.contains("注入的 mount 失败"),
                "stderr 应原样带回，实际 {stderr:?}"
            );
        }
        other => panic!("期望 CommandFailed，得到 {other:?}"),
    }
    // 失败的创建不留痕：Base 内没有东西，overlay 存储里也没有该 Intent 的目录
    assert!(!base_path.join(".ai").exists(), "失败的创建在 Base 里留下了东西");
    assert_eq!(
        dir_entries(&overlay_root()),
        Vec::<String>::new(),
        "失败的创建在 overlay 存储里留下了残留"
    );
}
