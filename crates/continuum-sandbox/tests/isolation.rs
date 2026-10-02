//! 内核层隔离的行为用例（设计第 4.4、13 节）。
//!
//! 本文件只跑 Landlock 臂。bubblewrap 臂在 Task 7 以同一套用例重跑。
//!
//! 三条用例针对三种不同的失效模式：
//! - 写不进去（隔离缺失）——每条用例都有实验臂；
//! - 什么都写不进去（隔离过强或沙箱根本没起来）——每条用例都配了对照臂；
//! - 隔离不随后代继承（只在父进程内施加）——由孙进程用例覆盖。
//!
//! 符号链接用例另担一条：类型层只做路径分量检查（设计第 4.1 节），
//! 「Task 根内有一个指向根外的符号链接」这一逃逸由本层承担（设计第 4.3 节末）。

use continuum_sandbox::Sandbox;
use continuum_workspace::{BaseWorkspace, IntentId, TaskWorkspace};
use std::path::Path;
use std::process::{Command, ExitStatus};

/// 用 `Sandbox::spawn` 起 `sh -c <脚本>`，等待结束后返回退出状态。
///
/// 脚本是调用方给定的任意命令——沙箱不解释它，故断言只能落在退出状态与文件系统的
/// 实际变化上，不能落在 stderr 文本上（文本随 shell 而变）。
fn spawn_sh(sandbox: &Sandbox, task: &TaskWorkspace, script: &str) -> ExitStatus {
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(script);
    sandbox
        .spawn(task, cmd)
        .unwrap_or_else(|e| panic!("sandbox.spawn 失败：{e}"))
        .wait()
        .expect("等待子进程失败")
}

/// 建一对互不重叠的 Base 与 Task，返回 Task 与 Base 的路径。
///
/// `dir` 由调用方持有以保证存活；Base 目录先建出来，因为 `BaseWorkspace::new`
/// 要求路径已存在。`BaseWorkspace` 句柄不返回：它只是一条路径，`new_outside`
/// 不保留它，调用方要的是路径本身。
fn fixture(dir: &Path) -> (TaskWorkspace, std::path::PathBuf) {
    let base_path = dir.join("base");
    std::fs::create_dir_all(&base_path).unwrap();
    let base = BaseWorkspace::new(&base_path).unwrap();
    let task = TaskWorkspace::new_outside(&base, dir.join("task"), IntentId::new("i1")).unwrap();
    (task, base_path)
}

/// 实验臂：子进程写 Base 必须被拒。对照臂：写 Task 内必须成功。
/// 只有实验臂时，一个「拒绝一切写入」的沙箱同样能通过，故两臂必须成对。
#[test]
fn landlock_denies_writes_to_base_and_allows_writes_to_task() {
    let dir = tempfile::tempdir().unwrap();
    let (task, base_path) = fixture(dir.path());

    let sandbox = Sandbox::landlock();

    // 能力报告与实际行为必须一致：本机既然报「文件系统写入受限制」，下面的实验臂就
    // 必须真被拒。报告为假而隔离生效是漏报，报告为真而隔离没生效是谎报——
    // 后者比没有隔离更危险，故两者的这一致性由本条断言钉住。
    assert!(
        sandbox.capabilities().restricts_filesystem_writes,
        "本机 ABI 探测未报出文件系统隔离，但本文件要求隔离确实生效"
    );

    // 对照臂
    let status = spawn_sh(
        &sandbox,
        &task,
        &format!("echo ok > {}/in_task.txt", task.root().display()),
    );
    assert!(status.success(), "对照臂：Task 内写入应成功，实际 {status:?}");

    // 第三条臂：设备白名单不是摆设。重定向到 `/dev/null` 是子进程最常见的动作之一，
    // 少了这一臂，把设备放行集合清空也不会有用例变红。
    let status = spawn_sh(&sandbox, &task, "echo ok > /dev/null");
    assert!(
        status.success(),
        "对照臂：写 /dev/null 应成功，实际 {status:?}"
    );

    // 实验臂
    let status = spawn_sh(
        &sandbox,
        &task,
        &format!("echo bad > {}/in_base.txt", base_path.display()),
    );
    assert!(!status.success(), "实验臂：Base 写入应被拒，实际 {status:?}");
    assert!(
        !base_path.join("in_base.txt").exists(),
        "Base 中不得出现该文件"
    );
}

/// 隔离随进程继承：子进程再 fork 一个孙进程去写 Base，同样被拒。
///
/// 两臂同样成对：孙进程在 Task 内写入必须成功，否则「沙箱让嵌套 shell 整个不能写」
/// 也能让实验臂通过。
#[test]
fn landlock_confines_descendants_too() {
    let dir = tempfile::tempdir().unwrap();
    let (task, base_path) = fixture(dir.path());

    let sandbox = Sandbox::landlock();

    // 对照臂：孙进程写 Task 内
    let status = spawn_sh(
        &sandbox,
        &task,
        &format!(
            "sh -c 'echo ok > {}/in_task.txt'",
            task.root().display()
        ),
    );
    assert!(
        status.success(),
        "对照臂：孙进程写 Task 内应成功，实际 {status:?}"
    );

    // 实验臂：孙进程写 Base
    let status = spawn_sh(
        &sandbox,
        &task,
        &format!("sh -c 'echo bad > {}/in_base.txt'", base_path.display()),
    );
    assert!(
        !status.success(),
        "实验臂：孙进程写 Base 应被拒，实际 {status:?}"
    );
    assert!(
        !base_path.join("in_base.txt").exists(),
        "Base 中不得出现该文件"
    );
}

/// 符号链接逃逸：Task 根内有一个指向 Base 的符号链接，经它写文件同样被拒。
///
/// 类型层只做路径分量检查（设计第 4.1 节），`WritablePath::join("link/evil.txt")`
/// 的每一段都是 `Normal`，结构上检不出这一逃逸；该逃逸由本层承担。
#[test]
fn writing_through_a_symlink_inside_the_task_is_denied() {
    let dir = tempfile::tempdir().unwrap();
    let (task, base_path) = fixture(dir.path());

    // 在 Task 根内建一个指向 Base 的符号链接，再经它写文件
    let link = task.root().join("link");
    std::os::unix::fs::symlink(&base_path, &link).unwrap();

    let sandbox = Sandbox::landlock();

    // 对照臂：同一条路径上指向 Task 内的符号链接必须能写通。少了它，「一律拒绝经
    // 符号链接的写入」也能让实验臂通过，实验臂就没有判别力——本臂钉住被拒的原因是
    // 终点越出 Task 根，而不是路径上出现了符号链接。
    let inner = task.root().join("inner");
    std::fs::create_dir_all(&inner).unwrap();
    let good_link = task.root().join("good_link");
    std::os::unix::fs::symlink(&inner, &good_link).unwrap();
    let status = spawn_sh(
        &sandbox,
        &task,
        &format!("echo ok > {}/ok.txt", good_link.display()),
    );
    assert!(
        status.success(),
        "对照臂：经 Task 内符号链接写 Task 内应成功，实际 {status:?}"
    );

    let status = spawn_sh(
        &sandbox,
        &task,
        &format!("echo bad > {}/evil.txt", link.display()),
    );
    assert!(
        !status.success(),
        "经 Task 内符号链接写 Base 应被拒，实际 {status:?}"
    );
    assert!(
        !base_path.join("evil.txt").exists(),
        "Base 中不得出现该文件"
    );
}
