//! `task` 子命令的端到端用例：建工作区、沙箱执行、未 `--apply` 时的清理（设计下篇
//! 第 4.2 节第 1–6 步与第 8 步）。
//!
//! 全部经 `env!("CARGO_BIN_EXE_continuum-runtime")` 驱动**真实二进制**（与 P1 的
//! `recovery_roundtrip` 同法）：判据的对象是「驱动在真实内核、真实 git、真实挂载下做了
//! 什么」，import 本 crate 的函数看不到。
//!
//! 两种后端都覆盖：Git 仓库（worktree 后端，不需要命名空间）与非 Git 目录（overlay
//! 后端，驱动自己会把自身 re-exec 进 `unshare -Urm`）。后一条在无 `unshare` 的机器上
//! **显式跳过**并在 stderr 上标记执行/跳过条数（上篇的约定），不静默通过。
//!
//! 路径一律取 `canonicalize` 之后的值：临时目录可能落在符号链接之下，而子进程报出的
//! 工作目录（`pwd`）与驱动算出的路径都是规范路径，两侧不规范化会让失败点指向断言。

use continuum_persist::{Db, Value};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const BIN: &str = env!("CARGO_BIN_EXE_continuum-runtime");

/// Intent 标识。本文件的用例各自独占一个临时 Base，故固定的这一个就够。
const INTENT: &str = "i1";

/// overlay 根的环境变量。`continuum_workspace` 里那个常量是 `pub(crate)`，取不到，
/// 故这里是第二份写法——与 `continuum-workspace/tests/backend_overlay.rs` 同法。
const OVERLAY_ROOT_ENV: &str = "CONTINUUM_OVERLAY_ROOT";

// ── 夹具 ────────────────────────────────────────────────────────────────

/// 建一个真实 Git 仓库作 Base，返回（保活用的 `TempDir`，规范化的仓库根）。
///
/// 初始提交是必需的：worktree 后端先 `git branch`，而没有提交时 HEAD 未出生，
/// 那一步会失败——失败点会指向夹具而不是实现。
fn git_repo() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    // 固定初始分支名，否则取 `init.defaultBranch` 配置，各机器不同。
    run_git(&root, &["init", "-b", "main"]);
    // 缺这两项 `git commit` 会直接失败。
    run_git(&root, &["config", "user.email", "test@example.invalid"]);
    run_git(&root, &["config", "user.name", "Continuum 测试"]);
    std::fs::write(root.join("README.md"), "初始内容\n").unwrap();
    run_git(&root, &["add", "-A"]);
    run_git(&root, &["commit", "-m", "初始提交"]);
    (dir, root)
}

/// 在 `dir` 内跑一条 git 命令，返回裁剪后的 stdout；失败即断言失败。
fn run_git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?} 失败（退出码 {:?}）：{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

/// `dir` 的全部本地分支名。
fn local_branches(dir: &Path) -> Vec<String> {
    run_git(dir, &["branch", "--format=%(refname:short)"])
        .lines()
        .map(|l| l.trim().to_owned())
        .filter(|l| !l.is_empty())
        .collect()
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

/// worktree 后端建的 Task 根：`<base>/.ai/worktrees/<intent>`（§255）。
fn worktree_task_root(base: &Path) -> PathBuf {
    base.join(".ai/worktrees").join(INTENT)
}

/// 跑一次 `task`，返回输出。`after_db` 是 `--db` 之后的全部参数（`--exec` 连同命令）。
fn run_task(base: &Path, db: &Path, after_db: &[&str], envs: &[(&str, &OsStr)]) -> Output {
    let mut cmd = Command::new(BIN);
    cmd.args(["task", "--base"])
        .arg(base)
        .args(["--intent", INTENT, "--db"])
        .arg(db)
        .args(after_db);
    for (key, value) in envs {
        cmd.env(key, value);
    }
    cmd.output().expect("continuum-runtime 无法执行")
}

/// 库里 `workspace` 表的全部行：`(intent_id, backend, path)`。
///
/// 直接读列而不经 `continuum_workspace::load_workspace`：本文件要看的正是**落库编码**
/// （`backend` 列的小写文本），经类型化的读回会把编码这一层盖掉。
fn workspace_rows(db: &Path) -> Vec<(String, String, String)> {
    let db = Db::open(db).expect("打开数据库失败");
    let tx = db.begin().unwrap();
    let rows = tx
        .query("SELECT intent_id, backend, path FROM workspace", &[])
        .expect("读 workspace 表失败");
    let out = rows
        .iter()
        .map(|r| {
            let text = |v: &Value| match v {
                Value::Text(s) => s.clone(),
                other => panic!("列应为文本，实际 {other:?}"),
            };
            (text(&r[0]), text(&r[1]), text(&r[2]))
        })
        .collect();
    tx.commit().unwrap();
    out
}

/// 退出码为 0 的守卫，失败时把 stdout/stderr 一并带出来（否则只知道断言挂了）。
fn assert_success(out: &Output) {
    assert!(
        out.status.success(),
        "task 退出码 {:?}\n--- stdout ---\n{}\n--- stderr ---\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn stdout_of(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr_of(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

// ── 命名空间（overlay 后端才需要）────────────────────────────────────────

/// 环境是否具备建立用户与挂载命名空间的能力。
///
/// 探测方式是把**本测试二进制**以 `--list` 重新执行进 `unshare -Urm`：能起来即说明命名
/// 空间可建立。与 `continuum-workspace/tests/backend_overlay.rs` 同法（用自身二进制而非
/// `/bin/true`，要探的正是本测试进程能否被这样执行）。
fn namespace_available() -> bool {
    let exe = std::env::current_exe().expect("无法取到测试可执行文件的路径");
    Command::new("unshare")
        .args(["-Urm"])
        .arg(exe)
        .arg("--list")
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

/// 环境不具备本用例所需条件时的**显式**跳过标记，带本条的执行/跳过条数。
///
/// 不静默通过：cargo 在用例通过时不回显 stderr，用 `--nocapture` 运行即可见。
/// 与 `continuum-workspace/tests/common/mod.rs` 的两条同形（那是另一个测试 crate，
/// 取不到，故这里重复一份夹具代码，不是产品代码）。
fn skip(test: &str, reason: &str) {
    eprintln!("【跳过】{test}：{reason}。本条：执行 0、跳过 1。用 `--nocapture` 可见本行。");
}

/// 与 [`skip`] 对称：本条真的跑过了（含断言）。
fn ran(test: &str) {
    eprintln!("【运行】{test}：本条：执行 1、跳过 0。");
}

/// 本机是否有**至少一个**可用的沙箱机制（驱动不给 `--sandbox` 时为自动选择）。
///
/// 判据与驱动取的是同一处：`Sandbox::capabilities().restricts_filesystem_writes`
/// （理由见 `sandbox_select` 的模块文档）。两个都为假时驱动会以 `NoneAvailable` 拒绝运行，
/// 故**所有不给 `--sandbox` 的用例**在那样的机器上都跑不起来。
fn any_sandbox_mechanism_available() -> bool {
    continuum_sandbox::Sandbox::landlock()
        .capabilities()
        .restricts_filesystem_writes
        || continuum_sandbox::Sandbox::bubblewrap()
            .capabilities()
            .restricts_filesystem_writes
}

/// 依赖「驱动能自动选出一个机制」的用例的入口。返回 `false` 表示已**显式**跳过。
///
/// 少了这一步，本机既无 Landlock（内核 ABI < 1）也无 `bwrap` 时这些用例会**硬失败**——
/// 把「本机没有隔离机制」报成「实现有缺陷」。与三条受能力门控的用例同法：
/// 不静默通过，读数（执行/跳过条数）在 `--nocapture` 下可见。
fn require_auto_selected_sandbox(test: &str) -> bool {
    if any_sandbox_mechanism_available() {
        return true;
    }
    skip(
        test,
        "本机既无 Landlock（内核 ABI < 1）也无 bwrap，驱动会以「两者都不可用」拒绝运行",
    );
    false
}

// ── 用例 ────────────────────────────────────────────────────────────────

/// 命令在沙箱里跑得起来，且**写进的是 Task 根**，Base 里没有。
///
/// 「文件出现在 Task 根里」由子进程自己报出：它 `pwd` 报出工作目录（设计第 4.2 节要求
/// 那是 Task 根），再以**相对路径**写文件——相对路径经工作目录解析，故两者合起来钉住了
/// 文件落在哪儿。未 `--apply` 时 Task 根随后就被清理，跑完之后无法再观察它。
#[test]
fn a_task_command_runs_in_the_sandbox_and_can_write_the_task() {
    const TEST: &str = "a_task_command_runs_in_the_sandbox_and_can_write_the_task";
    if !require_auto_selected_sandbox(TEST) {
        return;
    }
    let (_d, base) = git_repo();
    let dbdir = tempfile::tempdir().unwrap();
    let db = dbdir.path().join("t.db");

    let out = run_task(
        &base,
        &db,
        &["--exec", "sh", "-c", "echo hi > inside.txt && pwd && cat inside.txt"],
        &[],
    );
    assert_success(&out);
    let stdout = stdout_of(&out);

    let expected_root = worktree_task_root(&base);
    assert!(
        stdout.lines().any(|l| l == expected_root.to_str().unwrap()),
        "子进程的工作目录应是 Task 根 {}\n--- stdout ---\n{stdout}",
        expected_root.display()
    );
    assert!(
        stdout.contains("hi"),
        "写进 Task 根的文件应能读回\n--- stdout ---\n{stdout}"
    );

    // Base 里没有它，而且 Base 的顶层只多出 worktree 后端的私有子树 `.ai/`
    // （它是 worktree 后端本来就建的，见 `continuum_workspace::worktree`），
    // 用户文件一件不多、一件不改。
    assert!(
        !base.join("inside.txt").exists(),
        "Task 内的文件出现在 Base 里：{}",
        base.display()
    );
    assert_eq!(
        dir_entries(&base),
        vec![".ai", ".git", "README.md"],
        "Base 顶层多出了东西"
    );
    assert_eq!(
        dir_entries(&base.join(".ai/worktrees")),
        Vec::<String>::new(),
        ".ai/worktrees 里留下了 residue"
    );
    assert_eq!(
        std::fs::read(base.join("README.md")).unwrap(),
        "初始内容\n".as_bytes(),
        "Base 的用户文件被改了"
    );
    ran(TEST);
}

/// 同一条命令**写不进 Base**，而写 Task 仍成功（对照臂）。
///
/// 实验臂用 `..` 向上逃逸到 Base，不在 argv 里写 Base 路径——设计第 4.2 节要求 argv
/// 不含 Base 路径，那是调用方义务；用相对路径也顺便测了「子进程自己往上走」这一形态。
#[test]
fn the_same_command_cannot_write_the_base() {
    const TEST: &str = "the_same_command_cannot_write_the_base";
    if !require_auto_selected_sandbox(TEST) {
        return;
    }
    let (_d, base) = git_repo();
    let dbdir = tempfile::tempdir().unwrap();
    let db = dbdir.path().join("t.db");

    // 实验臂：Task 根是 `<base>/.ai/worktrees/<intent>`，故 `../../..` 即 Base。
    let out = run_task(
        &base,
        &db,
        &["--exec", "sh", "-c", "echo bad > ../../../escaped.txt"],
        &[],
    );
    assert!(
        !out.status.success(),
        "往 Base 写的命令不该让驱动报成功\n--- stdout ---\n{}",
        stdout_of(&out)
    );
    // 断言到**哪一种**失败：命令自己非零退出（`CommandFailed`），不是沙箱不可用之类的
    // 别的错误——后者也会让退出码非零，但那是夹具坏了。
    let stderr = stderr_of(&out);
    assert!(
        stderr.contains("以退出码"),
        "期望「命令以退出码 N 失败」，实际 stderr：{stderr}"
    );
    assert!(
        !base.join("escaped.txt").exists(),
        "沙箱没有挡住往 Base 的写入：{}",
        base.join("escaped.txt").display()
    );
    // 第 6 步：**命令退出码非 0 时仍走第 8 步的清理**（失败也要收尾）。三样都要没了。
    // 这一条是可观察的：`ai/<intent>` 分支若还在，下面控制臂的创建会在 `git branch`
    // 那一步撞上它，从而以另一条错误失败——但那只说明「留下了东西」，说明不了是记录、
    // 目录还是分支，故这里逐个断言。
    assert!(
        workspace_rows(&db).is_empty(),
        "命令失败时记录也应随工作区一并删除，实际 {:?}",
        workspace_rows(&db)
    );
    assert!(
        !worktree_task_root(&base).exists(),
        "命令失败时 Task 根也应被清理：{}",
        worktree_task_root(&base).display()
    );
    assert_eq!(
        local_branches(&base),
        vec!["main".to_owned()],
        "命令失败时分支也应被回收"
    );

    // 对照臂：同一条命令结构、只把目标换成 Task 内的相对路径，必须成功且读得回。
    let out = run_task(
        &base,
        &db,
        &["--exec", "sh", "-c", "echo ok > kept.txt && cat kept.txt"],
        &[],
    );
    assert_success(&out);
    assert!(
        stdout_of(&out).contains("ok"),
        "对照臂：Task 内的写入应成功并读得回，实际 stdout：{}",
        stdout_of(&out)
    );
    assert!(
        !base.join("kept.txt").exists(),
        "对照臂的文件落在了 Base 里"
    );
    ran(TEST);
}

/// 未 `--apply`：跑完之后记录与工作区都不留。
///
/// 两处断言的对象不同：`workspace` 表（落库记录）与 Task 根（磁盘实物）、以及
/// worktree 后端特有的分支。三者都要没了，才算第 8 步走完。
#[test]
fn the_workspace_record_is_written_and_then_removed() {
    const TEST: &str = "the_workspace_record_is_written_and_then_removed";
    if !require_auto_selected_sandbox(TEST) {
        return;
    }
    let (_d, base) = git_repo();
    let dbdir = tempfile::tempdir().unwrap();
    let db = dbdir.path().join("t.db");

    assert_success(&run_task(&base, &db, &["--exec", "true"], &[]));

    assert!(
        workspace_rows(&db).is_empty(),
        "未 --apply 时记录应随工作区一并删除，实际 {:?}",
        workspace_rows(&db)
    );
    assert!(
        !worktree_task_root(&base).exists(),
        "Task 根仍在：{}",
        worktree_task_root(&base).display()
    );
    // 分支是 worktree 后端放弃时的第二件事。它若还在，说明放弃没走完（或没走对后端）。
    assert_eq!(
        local_branches(&base),
        vec!["main".to_owned()],
        "只应剩下用户自己的分支"
    );
    ran(TEST);
}

/// 命令行里没有任何入口能给出**另一个**后端。两个方向各有一条照片：
///
/// - **入口不存在**（本用例前半）：`--backend` 在解析期即被拒，且拒绝发生在任何 I/O
///   之前（Base 一字未动、库文件都没建）。这条不是形式主义：上篇那条 Critical
///   （`integrate_overlay` 删光 Base 的 `.git`）之所以打不到，正是因为没有任何入口能给出
///   一个与记录不符的后端。
/// - **驱动用的就是记录里那个**：本用例后半走 worktree 方向——放弃若误用 overlay 后端，
///   `require_layout` 会先拒（那不是 overlay 布局），放弃不成，记录、Task 根与分支都会
///   留着，下面三条断言随即变红。
///
/// **本用例这一臂单独分不出「取自记录」与「重新探测」**：在「Base 始终是同一个 git 仓库」
/// 这个前提下，`record.backend` 与现场 `detect_backend` 给出同一个值，两者不可分辨。
/// 真正把它分开的是 [`a_failed_discard_keeps_the_workspace_record`]（那条能把记录读回来
/// 看那一列），M5 变红也靠它。唯一能让本臂具备该判别力的装置是「Base 在创建与回收之间
/// 改变形态」（例如创建后 `git init`），本 task 没有造这个装置。
///
/// 整条用例都在 [`require_auto_selected_sandbox`] 的门控之下——**包括方向一**（它本身
/// 不需要机制：`--backend` 在解析期就被拒）。本机没有任何机制时，驱动对**任何** `task`
/// 调用都拒绝运行，这个文件的前提在那样的机器上不成立，故整条跳过、读数是干净的
/// 「执行 0、跳过 1」，而不是让方向一单独留下一个半吊子读数。
#[test]
fn there_is_no_way_to_override_the_backend_from_the_command_line() {
    const TEST: &str = "there_is_no_way_to_override_the_backend_from_the_command_line";
    if !require_auto_selected_sandbox(TEST) {
        return;
    }
    let (_d, base) = git_repo();
    let dbdir = tempfile::tempdir().unwrap();
    let db = dbdir.path().join("t.db");

    // 方向一：`--backend` 不认识。
    let out = run_task(&base, &db, &["--backend", "overlay", "--exec", "true"], &[]);
    assert!(!out.status.success(), "`--backend` 不得被当成有效选项");
    let stderr = stderr_of(&out);
    assert!(
        stderr.contains("--backend"),
        "错误信息应点名 --backend，实际 stderr：{stderr}"
    );
    // 拒绝发生在建工作区之前：Base 里不该多出 `.ai/`，库文件也不该被创建
    // （`Db::open` 会建出空文件，故这里查的是文件本身，不是表里的行）。
    assert!(
        !base.join(".ai").exists(),
        "被拒的调用在 Base 里建出了 .ai/"
    );
    assert!(!db.exists(), "被拒的调用不该碰数据库");

    // 方向二：正常跑一次，放弃确实按记录里的 worktree 后端走完了。
    assert_success(&run_task(&base, &db, &["--exec", "true"], &[]));
    assert!(workspace_rows(&db).is_empty(), "记录应随工作区一并删除");
    assert!(!worktree_task_root(&base).exists(), "Task 根仍在");
    assert_eq!(
        local_branches(&base),
        vec!["main".to_owned()],
        "worktree 后端的放弃应把 ai/{INTENT} 一并删掉"
    );
    ran(TEST);
}

/// 第 8 步的次序：放弃**失败**时不删记录，工作区也原样留着。
///
/// 两个方向的不一致里，「有记录无工作区」（放弃成功却没删记录）会留下一条指向虚无的记录，
/// 而「有工作区无记录」（删了记录却放弃失败）要到同一 Intent 再次创建时才显形。本用例钉
/// 的是后者的反面：放弃失败时两样都留着，且记录里的**后端与路径**正是驱动实际用的那套
/// （这就是「驱动实际使用的后端 == workspace 表那一行」的直接照片）。
///
/// 注入手法：PATH 前置一个假 `git`，只拦 `worktree remove` 这一条，其余原样转交真正的
/// git——创建那一步（`git branch`、`git worktree add`）必须照常成功。
#[test]
fn a_failed_discard_keeps_the_workspace_record() {
    const TEST: &str = "a_failed_discard_keeps_the_workspace_record";
    if !require_auto_selected_sandbox(TEST) {
        return;
    }
    let (_d, base) = git_repo();
    let dbdir = tempfile::tempdir().unwrap();
    let db = dbdir.path().join("t.db");

    let real_git = {
        let out = Command::new("sh")
            .args(["-c", "command -v git"])
            .output()
            .unwrap();
        String::from_utf8(out.stdout).unwrap().trim().to_owned()
    };
    assert!(!real_git.is_empty(), "找不到真正的 git，夹具不成立");

    let fake = tempfile::tempdir().unwrap();
    let fake_git = fake.path().join("git");
    std::fs::write(
        &fake_git,
        format!(
            "#!/bin/sh\n\
             # 只拦 `-C <base> worktree remove …`，其余原样转交真正的 git。\n\
             if [ \"$3\" = \"worktree\" ] && [ \"$4\" = \"remove\" ]; then\n\
               echo '注入的 worktree remove 失败' >&2\n\
               exit 1\n\
             fi\n\
             exec {real_git} \"$@\"\n"
        ),
    )
    .unwrap();
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&fake_git, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let path_env = format!(
        "{}:{}",
        fake.path().display(),
        std::env::var("PATH").unwrap_or_default()
    );

    let out = run_task(&base, &db, &["--exec", "true"], &[("PATH", OsStr::new(&path_env))]);
    assert!(
        !out.status.success(),
        "放弃失败时整条命令必须报失败\n--- stdout ---\n{}",
        stdout_of(&out)
    );
    // 断言到**哪一条**错：注入的那次失败原样带回（含 git 的 stderr 与分支名）。
    let stderr = stderr_of(&out);
    assert!(
        stderr.contains("注入的 worktree remove 失败"),
        "期望注入的放弃失败原样带回，实际 stderr：{stderr}"
    );
    assert!(
        stderr.contains("清理"),
        "应说明失败发生在清理那一步，实际 stderr：{stderr}"
    );

    // 记录留着，且后端列就是 worktree、路径列就是那个 Task 根。
    let expected_path = worktree_task_root(&base);
    assert_eq!(
        workspace_rows(&db),
        vec![(
            INTENT.to_owned(),
            "worktree".to_owned(),
            expected_path.to_str().unwrap().to_owned()
        )],
        "放弃失败时记录必须留着，且它记的正是驱动实际用的后端与路径"
    );
    // 工作区也留着：放弃未成功不得半途删掉目录或分支。
    assert!(expected_path.exists(), "放弃失败却把 Task 根删了");
    assert!(
        local_branches(&base).contains(&format!("ai/{INTENT}")),
        "放弃失败却把分支删了"
    );
    ran(TEST);
}

/// 显式指定 `--sandbox bubblewrap` 时**跑的确实是 bubblewrap**。
///
/// 两条机制在**读**这一侧语义不同，故这条可观察：bubblewrap 的 `--ro-bind / /` 把宿主根
/// 整个**可读**地挂进来，Base 读得到（写仍被拒）；Landlock 是「默认拒绝 + 白名单子树」，
/// Base 的内容不可达。若装配处把机制映射到**另一个**句柄（显式 bubblewrap 却拿到
/// Landlock），这条就读不到 Base 而变红——这正是本用例与它的姊妹用例要钉的那条缝。
///
/// 两臂不写在同一个用例里：各自的前提（`bwrap` 在不在、内核 ABI）不同，跳过时读数要
/// 分得清是哪一臂没跑。
#[test]
fn an_explicitly_named_bubblewrap_runs_under_bubblewrap() {
    const TEST: &str = "an_explicitly_named_bubblewrap_runs_under_bubblewrap";
    if !continuum_sandbox::Sandbox::bubblewrap()
        .capabilities()
        .restricts_filesystem_writes
    {
        skip(TEST, "PATH 中找不到 bwrap，本机没有该机制");
        return;
    }

    let (_d, base) = git_repo();
    let dbdir = tempfile::tempdir().unwrap();
    let db = dbdir.path().join("t.db");

    // Task 根是 `<base>/.ai/worktrees/<intent>`，故 `../../..` 即 Base。
    let out = run_task(
        &base,
        &db,
        &["--sandbox", "bubblewrap", "--exec", "sh", "-c", "cat ../../../README.md"],
        &[],
    );
    assert_success(&out);
    assert!(
        stdout_of(&out).contains("初始内容"),
        "bubblewrap 下 Base 应读得到（--ro-bind / /），实际 stdout：{}",
        stdout_of(&out)
    );
    ran(TEST);
}

/// 显式指定 `--sandbox landlock` 时**跑的确实是 Landlock**：Base 的内容不可达。
///
/// 前提是 Base 不在 Landlock 的只读白名单里（白名单是 `/usr`、`/lib`、`/lib64`、`/etc`、
/// `/bin`、`/sbin`）。夹具用的是**系统临时目录**，任一常见发行版下都不在其中；这条前提
/// 由本臂自己验证——读得成即是前提不成立或句柄接错，两种情形都该变红而不是被跳过。
#[test]
fn an_explicitly_named_landlock_runs_under_landlock() {
    const TEST: &str = "an_explicitly_named_landlock_runs_under_landlock";
    if !continuum_sandbox::Sandbox::landlock()
        .capabilities()
        .restricts_filesystem_writes
    {
        skip(TEST, "内核 Landlock ABI < 1，本机没有该机制");
        return;
    }

    let (_d, base) = git_repo();
    let dbdir = tempfile::tempdir().unwrap();
    let db = dbdir.path().join("t.db");

    let out = run_task(
        &base,
        &db,
        &["--sandbox", "landlock", "--exec", "sh", "-c", "cat ../../../README.md"],
        &[],
    );
    assert!(
        !out.status.success(),
        "Landlock 下 Base 的内容应不可达，命令却成功了\n--- stdout ---\n{}",
        stdout_of(&out)
    );
    // 断言到**哪一种**失败：命令自己非零退出，而不是机制选择/施加隔离失败。
    assert!(
        stderr_of(&out).contains("以退出码"),
        "期望「命令以退出码 N 失败」，实际 stderr：{}",
        stderr_of(&out)
    );
    ran(TEST);
}

/// overlay 后端：驱动把自身 re-exec 进 `unshare -Urm`，跑完把工作区与记录都清掉。
///
/// 「re-exec 真的发生了」由子进程报出的工作目录钉住：那是 overlay 的挂载点，只有挂上了
/// 才存在，而挂载要求本进程已在用户与挂载命名空间内——本测试进程不在其中，故不 re-exec
/// 就只会在建区那一步报 `NotInNamespace`，本用例会以另一条错误失败。
///
/// 无 `unshare` 的机器上**显式跳过**（上篇的约定）：这条窗口在读数时可见，不伪装成通过。
#[test]
fn an_overlay_task_reexecs_itself_into_a_namespace_and_cleans_up() {
    const TEST: &str = "an_overlay_task_reexecs_itself_into_a_namespace_and_cleans_up";
    if !namespace_available() {
        skip(TEST, "unshare -Urm 无法建立用户与挂载命名空间");
        return;
    }
    // 命名空间能建起来还不够：本用例不给 `--sandbox`，驱动还要能自动选出一个机制。
    if !require_auto_selected_sandbox(TEST) {
        return;
    }

    let holder = tempfile::tempdir().unwrap();
    // 全部路径都从规范化后的根推出：驱动不做规范化，而子进程报出的 `pwd` 是规范路径。
    let holder_root = holder.path().canonicalize().unwrap();
    let base = holder_root.join("base");
    std::fs::create_dir(&base).unwrap();
    std::fs::write(base.join("lower.txt"), b"lower\n").unwrap();
    let overlay_root = holder_root.join("overlays");
    let db = holder_root.join("t.db");

    let out = run_task(
        &base,
        &db,
        &["--exec", "sh", "-c", "echo hi > inside.txt && pwd && cat lower.txt"],
        &[(OVERLAY_ROOT_ENV, overlay_root.as_os_str())],
    );
    assert_success(&out);
    let stdout = stdout_of(&out);

    // 工作目录在 overlay 存储里，形状为 `<overlay 根>/<Base 标识>/<intent>/mnt`。
    let cwd = stdout
        .lines()
        .find(|l| l.starts_with(overlay_root.to_str().unwrap()))
        .unwrap_or_else(|| panic!("子进程未报出 overlay 里的工作目录\n--- stdout ---\n{stdout}"));
    assert!(
        Path::new(cwd).ends_with(format!("{INTENT}/mnt")),
        "工作目录应是 overlay 布局的 mnt：{cwd}"
    );
    // lower 的内容经覆盖层透传给子进程（顺带证明工作区确实挂起来了，不是空目录）
    assert!(
        stdout.contains("lower"),
        "Task 内应读得到 Base 的 lower 层内容\n--- stdout ---\n{stdout}"
    );

    // 未 --apply：记录与工作区都清掉，Base 一字未动。
    assert!(workspace_rows(&db).is_empty(), "记录应随工作区一并删除");
    assert_eq!(
        dir_entries(&overlay_root),
        Vec::<String>::new(),
        "overlay 存储里留下了残留"
    );
    assert!(!base.join(".ai").exists(), "Base 里出现了 .ai/");
    assert_eq!(
        dir_entries(&base),
        vec!["lower.txt"],
        "Base 里多出了东西"
    );

    ran(TEST);
}
