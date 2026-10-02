//! 内核层隔离的行为用例（设计第 4.4、13 节）。
//!
//! **两种机制各跑同一套用例**（设计第 13 节）：Landlock 与 bubblewrap 逐个跑过
//! [`for_each_mechanism`] 里的每条用例。本机没有 `bwrap` 时 bubblewrap 臂**显式打印跳过**
//! 而不静默通过；两种机制都不可用时整条用例 panic——「一条都没跑却全绿」是最坏的一种通过。
//!
//! 三条用例针对三种不同的失效模式：
//! - 写不进去（隔离缺失）——每条用例都有实验臂；
//! - 什么都写不进去（隔离过强或沙箱根本没起来）——每条用例都配了对照臂；
//! - 隔离不随后代继承（只在父进程内施加）——由孙进程用例覆盖。
//!
//! 另有四条补在四处：
//! - 符号链接逃逸：类型层只做路径分量检查（设计第 4.1 节），「Task 根内有一个指向根外的
//!   符号链接」这一逃逸由本层承担（设计第 4.3 节末）。
//! - 硬链接，两条，钉的是互补的两半：
//!   `a_hardlink_to_the_base_cannot_be_manufactured_inside_the_task` 钉「逃逸造不出来」；
//!   `a_host_planted_hardlink_is_a_known_gap_the_sandbox_cannot_close` 是**特征化用例**，
//!   钉「宿主预置的硬链接两种机制都关不上」这一已知边界（它断言逃逸成功，见该用例文档）。
//! - 子进程的工作目录：设计第 4.2 节要求它是 Task 根，而 bubblewrap 分支重建命令会丢掉
//!   `Command::current_dir`，靠 `--chdir` 补回来。少了这一条，参数表删掉 `--chdir` 无人发现。
//! - 环境变量：`spawn` 的文档承诺 bubblewrap 分支**重放**调用方经 `Command::env` 显式设置
//!   的变量（`get_envs` 有读取接口，而 stdio 没有）。少了这一条，那一段承诺没有任何落点。
//!
//! **「两种机制各跑同一套用例」这句话只在「写」这一侧成立。** 两条机制在写上语义相同
//! （都由本文件的两臂用例覆盖），在读上**不同**，本文件不假定二者可互换：
//!
//! - Landlock 是「默认拒绝 + 白名单子树」，Base 不在任何被授予子树内，故**读写皆不可达**；
//! - bubblewrap 的 `--ro-bind / /` 把宿主根整个可读地挂进来，**Base 读得到**，只有写被拒。
//!
//! 该差异是两种机制各自的形状决定的，不是配置错误（它也让 bwrap 分支**可用**：`/proc`、
//! `/tmp`、`$HOME` 都读得到，而 Landlock 那边白名单之外一概不可达）。**故本文件不写任何
//! 「子进程读不到 Base」的断言**——它在 bubblewrap 分支必红。
//!
//! 对写这一侧的一处后果：本文件的实验臂是**过量决定**的——「写 Base 被拒」在 Landlock 下
//! 既可能因为「写被拒」成立，也可能因为「Base 整个不可达」成立，本文件分不开这两者。
//! 只读放行路径那一侧由 `src/sandbox.rs` 的 `read_only_paths_are_readable_but_not_writable`
//! 单测覆盖——它把探针放进**被授予范围内**，读成功 + 写被拒，两者就分开了。
//!
//! 另有两处不住在本文件：
//! - `bwrap` 不在 `PATH` 中时 `spawn` 以 `MechanismUnavailable` 拒绝启动——
//!   见 `src/sandbox.rs` 的 `bubblewrap_refuses_to_start_when_the_binary_is_missing`。
//!   它待在那里是因为那个句柄在 crate 外**构造不出来**（`BubblewrapSandbox` 的字段私有，
//!   这正是它的唯一性所在），而那正是被测的前提。
//! - 参数表本身（逐项、含顺序）见 `src/bubblewrap.rs` 的单测，不必真起进程。

use continuum_sandbox::Sandbox;
use continuum_workspace::{BaseWorkspace, IntentId, TaskWorkspace};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

/// 本机可用的两种机制。每次调用新建句柄：`Sandbox` 的构造期探测只在构造时做一次，
/// 跨用例复用会让「探测结果与用例内实际行为对不上」这类问题躲过测试。
fn mechanisms() -> [(&'static str, Sandbox); 2] {
    [
        ("landlock", Sandbox::landlock()),
        ("bubblewrap", Sandbox::bubblewrap()),
    ]
}

/// 对每种机制各跑一次 `body`，每次给一份**全新的**夹具。
///
/// 每次新夹具是必需的而非图省事：两条臂会往 Task 与 Base 里写文件，共用一份夹具时
/// 先跑的机制留下的文件会让后跑的机制里「文件不存在」那类断言失真。
///
/// 不可用的机制打印跳过说明后略过；**全部不可用时 panic**——否则本文件会在一条用例都没
/// 真正跑过的情况下全绿，而「静默通过」正是本项目要求显式标记跳过要防的东西。
fn for_each_mechanism(mut body: impl FnMut(&str, &Sandbox, &TaskWorkspace, &Path)) {
    let mut ran = 0;
    for (name, sandbox) in mechanisms() {
        if !sandbox.capabilities().restricts_filesystem_writes {
            eprintln!(
                "【跳过】机制 {name} 在本机不可用（capabilities 报告文件系统写入不受限），\
                 本用例没有在它上面验证任何隔离"
            );
            continue;
        }
        let dir = tempfile::tempdir().unwrap();
        let (task, base_path) = fixture(dir.path());
        eprintln!("【运行】机制 {name}");
        body(name, &sandbox, &task, &base_path);
        ran += 1;
    }
    assert!(
        ran > 0,
        "两种机制在本机都不可用，本用例没有在任何一个沙箱上验证隔离"
    );
}

/// 用 `Sandbox::spawn` 起 `sh -c <脚本>`，等待结束后返回退出状态。
///
/// 脚本是调用方给定的任意命令——沙箱不解释它，故断言只能落在退出状态与文件系统的
/// 实际变化上，不能落在 stderr 文本上（文本随 shell 而变）。
///
/// `mechanism` 只进 panic 消息：断言失败时得知道红的是哪条臂，否则 `for_each_mechanism`
/// 里两次运行共用一条 panic 文本，看不出是哪一次挂的。
fn spawn_sh(sandbox: &Sandbox, mechanism: &str, task: &TaskWorkspace, script: &str) -> ExitStatus {
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(script);
    sandbox
        .spawn(task, cmd)
        .unwrap_or_else(|e| panic!("[{mechanism}] sandbox.spawn 失败：{e}"))
        .wait()
        .expect("等待子进程失败")
}

/// 建一对互不重叠的 Base 与 Task，返回 Task 与 Base 的路径。
///
/// `dir` 由调用方持有以保证存活；Base 目录先建出来，因为 `BaseWorkspace::new`
/// 要求路径已存在。`BaseWorkspace` 句柄不返回：它只是一条路径，`new_outside`
/// 不保留它，调用方要的是路径本身。
fn fixture(dir: &Path) -> (TaskWorkspace, PathBuf) {
    let base_path = dir.join("base");
    std::fs::create_dir_all(&base_path).unwrap();
    let base = BaseWorkspace::new(&base_path).unwrap();
    let task = TaskWorkspace::new_outside(&base, dir.join("task"), IntentId::new("i1")).unwrap();
    (task, base_path)
}

/// 实验臂：子进程写 Base 必须被拒。对照臂：写 Task 内必须成功。
/// 只有实验臂时，一个「拒绝一切写入」的沙箱同样能通过，故两臂必须成对。
#[test]
fn denies_writes_to_base_and_allows_writes_to_task() {
    for_each_mechanism(|mechanism, sandbox, task, base_path| {
        // 对照臂
        let status = spawn_sh(
            sandbox,
            mechanism,
            task,
            &format!("echo ok > {}/in_task.txt", task.root().display()),
        );
        assert!(
            status.success(),
            "[{mechanism}] 对照臂：Task 内写入应成功，实际 {status:?}"
        );

        // 第三条臂：设备白名单不是摆设。重定向到 `/dev/null` 是子进程最常见的动作之一。
        // 对 bubblewrap 它是 `--dev /dev` 唯一的用例——少了那一项，`/dev` 是宿主 `/dev`
        // 的只读 bind，这一臂以 `Permission denied` 失败。
        let status = spawn_sh(sandbox, mechanism, task, "echo ok > /dev/null");
        assert!(
            status.success(),
            "[{mechanism}] 对照臂：写 /dev/null 应成功，实际 {status:?}"
        );

        // 实验臂
        let status = spawn_sh(
            sandbox,
            mechanism,
            task,
            &format!("echo bad > {}/in_base.txt", base_path.display()),
        );
        assert!(
            !status.success(),
            "[{mechanism}] 实验臂：Base 写入应被拒，实际 {status:?}"
        );
        assert!(
            !base_path.join("in_base.txt").exists(),
            "[{mechanism}] Base 中不得出现该文件"
        );
    });
}

/// 隔离随进程继承：子进程再 fork 一个孙进程去写 Base，同样被拒。
///
/// 两臂同样成对：孙进程在 Task 内写入必须成功，否则「沙箱让嵌套 shell 整个不能写」
/// 也能让实验臂通过。
///
/// 两种机制走的是不同路径：Landlock 靠规则集随 `exec` 被继承，bubblewrap 靠子进程
/// 活在同一个挂载命名空间里。用例相同，被验的机制不同。
#[test]
fn confines_descendants_too() {
    for_each_mechanism(|mechanism, sandbox, task, base_path| {
        // 对照臂：孙进程写 Task 内
        let status = spawn_sh(
            sandbox,
            mechanism,
            task,
            &format!("sh -c 'echo ok > {}/in_task.txt'", task.root().display()),
        );
        assert!(
            status.success(),
            "[{mechanism}] 对照臂：孙进程写 Task 内应成功，实际 {status:?}"
        );

        // 实验臂：孙进程写 Base
        let status = spawn_sh(
            sandbox,
            mechanism,
            task,
            &format!("sh -c 'echo bad > {}/in_base.txt'", base_path.display()),
        );
        assert!(
            !status.success(),
            "[{mechanism}] 实验臂：孙进程写 Base 应被拒，实际 {status:?}"
        );
        assert!(
            !base_path.join("in_base.txt").exists(),
            "[{mechanism}] Base 中不得出现该文件"
        );
    });
}

/// 符号链接逃逸：Task 根内有一个指向 Base 的符号链接，经它写文件同样被拒。
///
/// 类型层只做路径分量检查（设计第 4.1 节），`WritablePath::join("link/evil.txt")`
/// 的每一段都是 `Normal`，结构上检不出这一逃逸；该逃逸由本层承担。
#[test]
fn writing_through_a_symlink_inside_the_task_is_denied() {
    for_each_mechanism(|mechanism, sandbox, task, base_path| {
        // 在 Task 根内建一个指向 Base 的符号链接，再经它写文件
        let link = task.root().join("link");
        std::os::unix::fs::symlink(base_path, &link).unwrap();

        // 对照臂：同一条路径上指向 Task 内的符号链接必须能写通。少了它，「一律拒绝经
        // 符号链接的写入」也能让实验臂通过，实验臂就没有判别力——本臂钉住被拒的原因是
        // 终点越出 Task 根，而不是路径上出现了符号链接。
        let inner = task.root().join("inner");
        std::fs::create_dir_all(&inner).unwrap();
        let good_link = task.root().join("good_link");
        std::os::unix::fs::symlink(&inner, &good_link).unwrap();
        let status = spawn_sh(
            sandbox,
            mechanism,
            task,
            &format!("echo ok > {}/ok.txt", good_link.display()),
        );
        assert!(
            status.success(),
            "[{mechanism}] 对照臂：经 Task 内符号链接写 Task 内应成功，实际 {status:?}"
        );

        let status = spawn_sh(
            sandbox,
            mechanism,
            task,
            &format!("echo bad > {}/evil.txt", link.display()),
        );
        assert!(
            !status.success(),
            "[{mechanism}] 经 Task 内符号链接写 Base 应被拒，实际 {status:?}"
        );
        assert!(
            !base_path.join("evil.txt").exists(),
            "[{mechanism}] Base 中不得出现该文件"
        );
    });
}

/// 硬链接逃逸：子进程**无法在沙箱内把逃逸制造出来**——`ln <base>/f <task>/hl` 被拒。
///
/// # 这条用例钉的是什么，不钉什么
///
/// 钉的是：逃逸不能从沙箱内部制造。两条机制都真被拒，**呈现形式也相同**——`ln` 都以
/// `EXDEV`（`Invalid cross-device link`）失败，本机实测两条臂的 stderr 文本一致。
/// 区别在**内核侧的原因**，不在呈现形式：
///
/// - Landlock：跨目录 link 需**源目录**的 `REFER` 授权，而 Base 不在任何被授予子树内，
///   该授权缺失；内核把这类拒绝也以 `EXDEV` 呈现（与真正的跨设备同形）。
/// - bubblewrap：源与目标落在不同的 vfsmount（`--ro-bind / /` 与
///   `--bind <task> <task>` 是两次挂载），是真正的跨设备。
///
/// **不得写成「Landlock 下 Base 整个不可达、`ln` 读不到源」。** 本机实测（Landlock
/// ABI=10）：Landlock 下 `stat <base>/secret.txt` **成功**（返回 size 6）——Landlock
/// 只拦 `handle_access` 声明过的访问类别，`stat` 不在其中，Base 的**路径**走得通；真正
/// 被拒的是 `link` 所需的 `REFER`。无沙箱对照下同一条 `ln` **成功**，证明拒绝确由沙箱
/// 造成。故本用例只断言失败与「Base 未变」，不断言 errno 或 stderr 文本。
///
/// **不钉的是**：宿主（沙箱外、可信方）事先在 Task 根内建好一个指向 Base inode 的硬链接
/// 之后，子进程写它的**后果**。那一半两种机制都拦不住，实测如下（本机 bwrap 0.13.0，
/// Landlock ABI=10）：经该硬链接写入，两种机制下**都成功改写了 Base 的文件内容**。
/// 原因是两者都按「路径」判定——挂载隔离看路径属于哪次挂载，Landlock 看路径落在哪个
/// 被授予子树——而同一个 inode 经 Task 这条可写路径可达。这不是参数少写，是这两类机制
/// 的能力边界；设计第 13 节也只要求覆盖符号链接逃逸。故本文件不写一条「断言该写入被拒」
/// 的用例：它在本机必红，红的原因不在实现。
#[test]
fn a_hardlink_to_the_base_cannot_be_manufactured_inside_the_task() {
    for_each_mechanism(|mechanism, sandbox, task, base_path| {
        let secret = base_path.join("secret.txt");
        std::fs::write(&secret, b"secret").unwrap();
        let link = task.root().join("hl.txt");

        let status = spawn_sh(
            sandbox,
            mechanism,
            task,
            &format!("ln {} {}", secret.display(), link.display()),
        );
        assert!(
            !status.success(),
            "[{mechanism}] 子进程不得能在 Task 内造出指向 Base 的硬链接，实际 {status:?}"
        );
        assert!(
            !link.exists(),
            "[{mechanism}] Task 内不得出现该硬链接——存在即意味着上面的 ln 成功了"
        );
        assert_eq!(
            std::fs::read(&secret).unwrap(),
            b"secret",
            "[{mechanism}] Base 的文件内容不得改变"
        );
    });
}

/// 特征化用例：**宿主预置的硬链接是两种机制都关不上的残余缺口。**
///
/// **本用例断言逃逸成功，这是刻意的。** 它记录的是一条能力边界，不是一条需求：宿主
/// （沙箱外、可信方）先在 Task 根内建一个指向 Base 内某 inode 的硬链接之后，子进程写这个
/// 链接与写 Base 的同名文件写在同一个 inode 上——挂载隔离按「路径属于哪次挂载」判定，
/// Landlock 按「路径落在哪个被授予子树」判定，而该路径两条机制下都判为「Task 这条可写
/// 路径」。实测（本机 bwrap 0.13.0，Landlock ABI=10）：两种机制下写入都成功，Base 的文件
/// 内容都被改写。
///
/// 为什么值得写成用例而不是一句注释：**注释不会在有人改沙箱时报警，这条会。** 哪天这两种
/// 机制（或将来新增的第三种）真把这条堵上了，本用例变红——那时该做的是把它改成「被拒」
/// 并更新这段文档，而不是删掉它。
///
/// 威胁模型边界：本逃逸要求该链接**先存在**，而沙箱内的子进程造不出它（见
/// `a_hardlink_to_the_base_cannot_be_manufactured_inside_the_task`），两种 Workspace 后端
/// 也都以复制而非硬链接落地文件。故本层不作防护。设计侧已按此记入。
#[test]
fn a_host_planted_hardlink_is_a_known_gap_the_sandbox_cannot_close() {
    for_each_mechanism(|mechanism, sandbox, task, base_path| {
        let secret = base_path.join("secret.txt");
        std::fs::write(&secret, b"secret").unwrap();
        // 链接由**宿主侧、沙箱外**建立——这正是本缺口的前提条件。
        let link = task.root().join("hl.txt");
        std::fs::hard_link(&secret, &link).unwrap();

        let status = spawn_sh(
            sandbox,
            mechanism,
            task,
            &format!("echo HACKED > {}", link.display()),
        );
        assert!(
            status.success(),
            "[{mechanism}] 现状是本条逃逸会成功；它若失败，说明边界变了，见本用例文档"
        );
        assert_eq!(
            std::fs::read_to_string(&secret).unwrap(),
            "HACKED\n",
            "[{mechanism}] Base 的文件内容应被经硬链接的写入改写——这正是本用例要记录的缺口"
        );
    });
}

/// 子进程的工作目录是 Task 根（设计第 4.2 节）。
///
/// 不能靠捕获子进程的 stdout：bubblewrap 分支重建命令时 stdio 已被丢弃
/// （`Command` 没有 `get_stdout`），故让子进程自己把 `pwd` 写进文件再读回来。
/// 这一条也是 `--chdir <task>` 唯一的用例。
///
/// 两臂成对：`pwd` 的输出必须**等于** Task 根，而不只是「非空」——一个不设工作目录的
/// 沙箱会让子进程继承测试进程的 cwd，那时 `pwd` 照样有输出。
#[test]
fn the_child_working_directory_is_the_task_root() {
    for_each_mechanism(|mechanism, sandbox, task, _base_path| {
        let out = task.root().join("cwd.txt");
        let status = spawn_sh(sandbox, mechanism, task, &format!("pwd > {}", out.display()));
        assert!(
            status.success(),
            "[{mechanism}] 取工作目录的脚本应成功，实际 {status:?}"
        );
        let cwd = std::fs::read_to_string(&out).unwrap();
        assert_eq!(
            cwd.trim_end(),
            task.root().to_str().unwrap(),
            "[{mechanism}] 子进程的工作目录应是 Task 根"
        );
    });
}

/// 调用方经 `Command::env` 显式设置的环境变量随沙箱全程到达子进程。
///
/// 这一条主要钉 bubblewrap 分支：它**重建**命令（`bwrap <加固参数> -- <原命令>`），
/// 而重建会丢掉调用方在 `Command` 上设过的**一部分**东西——stdio 丢掉（`Command` 没有
/// `get_stdout` 之类的读取接口），`env` 保得住（`get_envs` 有）。`spawn` 的文档承诺了
/// env 重放，本用例是该承诺唯一的落点：删掉重放那一段，bubblewrap 臂变红。
/// Landlock 分支原样传递同一个 `Command`，env 天然保留，本用例在它上面是顺带覆盖。
///
/// 变量名取一个**父进程未设**的：若它恰好在父环境里，「未重放」与「继承」两种情形在
/// 子进程看来相同，用例就没有判别力。子进程把值写进 Task 内的文件再读回来——bubblewrap
/// 分支的 stdio 已在重建时丢掉，不能靠 stdout。
#[test]
fn explicitly_set_environment_variables_reach_the_child() {
    for_each_mechanism(|mechanism, sandbox, task, _base_path| {
        let out = task.root().join("env.txt");
        let mut cmd = Command::new("sh");
        cmd.arg("-c")
            .arg(format!("printf %s \"$CONTINUUM_SANDBOX_ENV_PROBE\" > {}", out.display()));
        cmd.env("CONTINUUM_SANDBOX_ENV_PROBE", "value-from-caller-7f3a");

        let status = sandbox
            .spawn(task, cmd)
            .unwrap_or_else(|e| panic!("[{mechanism}] sandbox.spawn 失败：{e}"))
            .wait()
            .expect("等待子进程失败");
        assert!(
            status.success(),
            "[{mechanism}] 环境变量探针脚本应成功，实际 {status:?}"
        );
        assert_eq!(
            std::fs::read_to_string(&out).unwrap(),
            "value-from-caller-7f3a",
            "[{mechanism}] 调用方显式设置的环境变量必须原样到达子进程"
        );
    });
}

/// bubblewrap 的能力报告只报它**实际行使**的隔离项。
///
/// 两项断言，缺一不可：
/// - 报告侧：`isolates_network` 与 `isolates_pid` 为假。本子项目的参数集不含
///   `--unshare-net` / `--unshare-pid`，机制「本来能」隔离这两项不构成报真的理由。
/// - 行为侧：把子进程的 `/proc/self/ns/net`、`/proc/self/ns/pid` 读出来与宿主比，
///   必须**逐字相同**。只断言报告侧是不够的——参数表哪天多出一项 `--unshare-net`，
///   报告仍然是假，两边就分家了；本用例把两边钉在一起。
///
/// 子进程的输出同样不能靠 stdout（bubblewrap 分支的 stdio 已被丢弃），故写进文件。
#[test]
fn bubblewrap_reports_only_the_capabilities_it_actually_exercises() {
    let sandbox = Sandbox::bubblewrap();
    let caps = sandbox.capabilities();

    assert_eq!(caps.mechanism, "bubblewrap");
    // 网络与 PID 与本配置无关，恒为假——这一项不依赖本机是否有 bwrap。
    assert!(
        !caps.isolates_network,
        "本配置不传 --unshare-net，报告网络隔离为真是谎报"
    );
    assert!(!caps.isolates_pid, "本配置不传 --unshare-pid，报告 PID 隔离为真是谎报");

    if !caps.restricts_filesystem_writes {
        eprintln!(
            "【跳过】机制 bubblewrap 在本机不可用（PATH 中无 bwrap），\
             本用例未验证命名空间的实际取值"
        );
        return;
    }

    // 行为侧。每次新夹具，理由同 `for_each_mechanism`。
    let dir = tempfile::tempdir().unwrap();
    let (task, _base_path) = fixture(dir.path());
    let out = task.root().join("ns.txt");
    let status = spawn_sh(
        &sandbox,
        "bubblewrap",
        &task,
        &format!(
            "readlink /proc/self/ns/net > {f}; readlink /proc/self/ns/pid >> {f}",
            f = out.display()
        ),
    );
    assert!(status.success(), "读取命名空间的脚本应成功，实际 {status:?}");

    let host_net = std::fs::read_link("/proc/self/ns/net").unwrap();
    let host_pid = std::fs::read_link("/proc/self/ns/pid").unwrap();
    assert_eq!(
        std::fs::read_to_string(&out).unwrap(),
        format!("{}\n{}\n", host_net.display(), host_pid.display()),
        "报告说网络与 PID 未隔离，子进程里读到的命名空间就必须与宿主逐字相同"
    );
}
