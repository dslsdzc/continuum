//! `task` 子命令的端到端用例：建工作区、沙箱执行、效应声明的 Journal 接线、
//! `--apply` 的策略裁决与集成、未 `--apply`（或命令失败）时的清理
//! （设计下篇第 4.2 节第 1–8 步）。
//!
//! **本文件的 `--apply` 用例全部受 [`require_auto_selected_sandbox`] 门控**（驱动要真的
//! 跑起来才谈得上集成）。裁决本身不依赖沙箱，故它的承重断言**不放在这里**：
//! `src/task_cmd.rs` 的 `#[cfg(test)] mod tests` 直接驱动 `arbitrate` / `mints`，
//! 无门控地逐格钉住那张映射表与本文件四条用例的模型面。本文件负责的另一半是
//! **接线**——驱动器真的按那个裁决去做了没有。
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

use continuum_effect::{
    Effect, EffectId, EffectState, EffectType, MarkExecutingAsUnknown, record_planned,
};
use continuum_persist::{Db, RecoveryRegistry, Value, run_recovery};
use continuum_policy::{Condition, Decision, Level, Policy, Scope};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

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

/// 库里 `effect` 表的全部行：`(effect_type, target, state)`，按类型与目标排序。
///
/// 直接读列而不经 `continuum_effect::load_effect`：本文件要看的正是**落库编码**
/// （两个枚举列的小写文本），经类型化的读回会把编码这一层盖掉（同 [`workspace_rows`]）。
fn effect_rows(db: &Path) -> Vec<(String, String, String)> {
    let db = Db::open(db).expect("打开数据库失败");
    let tx = db.begin().unwrap();
    let rows = tx
        .query(
            "SELECT effect_type, target, state FROM effect ORDER BY effect_type, target",
            &[],
        )
        .expect("读 effect 表失败");
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

/// `effect` 表各行的状态列，按 [`effect_rows`] 的次序。
fn effect_states(db: &Path) -> Vec<String> {
    effect_rows(db)
        .into_iter()
        .map(|(_, _, state)| state)
        .collect()
}

/// 在 `effect` 表里种下一条记录，模拟「上一次运行已经登记过这个键」。
///
/// 键与 id 由测试**手写字面量**（`<意图字节长>:<意图>:<类型>:<目标>`），不调驱动私有的
/// `effect_key`：手写的字面量钉住格式，调实现去拼则格式改了两者一起漂移、断言照过。
fn seed_effect(db: &Path, intent: &str, effect_type: EffectType, target: &str) {
    let mut migrations = continuum_persist::builtin_migrations();
    migrations.extend(continuum_effect::p2_effect_migrations());
    let handle = Db::open_with(db, migrations).expect("打开数据库失败");
    handle.migrate().expect("应用迁移失败");
    let key = format!(
        "{}:{intent}:{}:{target}",
        intent.len(),
        effect_type.as_str()
    );
    let tx = handle.begin().unwrap();
    record_planned(
        &tx,
        &Effect {
            id: EffectId::new(key.clone()),
            effect_type,
            target: target.to_owned(),
            parameters: serde_json::json!({}),
            authorization: "种下的既有记录".to_owned(),
            idempotency_key: key,
            state: EffectState::Planned,
            planned_at: 1,
            updated_at: 1,
        },
    )
    .expect("种下既有记录失败");
    tx.commit().unwrap();
}

/// 目标为 `target` 的那条效应记录的 `authorization` 列（设计第 6.7 节）。
fn authorization_of(db: &Path, target: &str) -> String {
    let db = Db::open(db).expect("打开数据库失败");
    let tx = db.begin().unwrap();
    let rows = tx
        .query(
            "SELECT authorization FROM effect WHERE target = ?1",
            &[Value::text(target)],
        )
        .expect("读 effect 表失败");
    let value = rows
        .first()
        .map(|r| match &r[0] {
            Value::Text(s) => s.clone(),
            other => panic!("authorization 列应为文本，实际 {other:?}"),
        })
        .unwrap_or_else(|| panic!("没有目标为 {target} 的效应记录"));
    tx.commit().unwrap();
    value
}

/// 库里 `audit_log` 的全部 `(kind, payload)`，按 seq 排序。
///
/// 读的是**落库的文本**：本文件要钉的正是 `kind` 列的编码（设计 §6.4 定的
/// `AuditKind::ExternalEffects`）与 payload 里的操作名，经 `AuditRecord` 的类型化读回会把
/// 这两层都盖掉。
fn audit_rows(db: &Path) -> Vec<(String, String)> {
    let db = Db::open(db).expect("打开数据库失败");
    let tx = db.begin().unwrap();
    let rows = tx
        .query("SELECT kind, payload FROM audit_log ORDER BY seq", &[])
        .expect("读 audit_log 失败");
    let out = rows
        .iter()
        .map(|r| {
            let text = |v: &Value| match v {
                Value::Text(s) => s.clone(),
                other => panic!("审计列应为文本，实际 {other:?}"),
            };
            (text(&r[0]), text(&r[1]))
        })
        .collect();
    tx.commit().unwrap();
    out
}

/// 一条条件恒真（空合取）的规则：任何 `PolicyContext` 都成立。
///
/// 恒真是刻意的：本文件的用例要钉的是**驱动有没有按裁决行事**，条件怎么求值属
/// `continuum-policy` 的用例（`tests/condition.rs` / `tests/arbitration.rs`）。
fn always(level: Level, decision: Decision) -> Policy {
    Policy {
        level,
        condition: Condition::parse(&serde_json::json!({"all": []}))
            .expect("空合取是合法条件"),
        decision,
        scope: Scope::User,
    }
}

/// 建出 `policy` 表并写入若干规则，供随后启动的驱动读回。
///
/// **用 `continuum_policy` 自己的 `save_policy` 写，不手写 SQL 字面量**：驱动读的是该
/// crate 的编码（`level` / `decision` / `scope` 三列与 `condition` 列），手抄一份到测试里
/// 就是给同一件事立第二个来源——`save_policy` 改了编码，手抄那份照样绿，而驱动读不回。
/// （读回来那几处反过来要手写 SQL，理由见 [`workspace_rows`]。）
///
/// 只应用 `policy` 这一条迁移：驱动自己开库时会把它那份清单里其余几条补上，
/// 编号 41 已记录在案故被跳过。这顺带证明两边的清单能接上，而不是各建各的表。
fn seed_policies(db: &Path, rules: &[(&str, Level, Decision)]) {
    let policies: Vec<(&str, Policy)> = rules
        .iter()
        .map(|(id, level, decision)| (*id, always(*level, *decision)))
        .collect();
    seed_policy_rows(db, &policies);
}

/// 同 [`seed_policies`]，但收**完整规则**（条件自定）——`effect_type` 这类条件写不出
/// 恒真的形式，而 P3 强制点 (2) 的用例要用它把「逐条效应」这一维变得可观察。
fn seed_policy_rows(db: &Path, rules: &[(&str, Policy)]) {
    let handle = Db::open_with(db, continuum_policy::p2_policy_migrations()).unwrap();
    handle.migrate().unwrap();
    let tx = handle.begin().unwrap();
    for (id, policy) in rules {
        continuum_policy::save_policy(&tx, id, policy).unwrap();
    }
    tx.commit().unwrap();
}

/// 一条**只对某一种效应**成立的 `Allow`，占**第 1 级**。
///
/// 第 1 级是刻意的：P3 强制点 (2) 的逐条效应裁决要用它压过其余落库规则（例如一条第 3 级
/// 的 `RequireApproval`），而集成那次裁决的上下文里 `effect_type` 缺省，本规则**不成立**
/// ——两者由此可以给出不同的裁决。上下文里没有 `effect_type` 的条件不成立，见
/// `continuum_policy::rule` 的 `Fact::EffectType`。
fn allow_only_effect(effect_type: EffectType) -> Policy {
    Policy {
        level: Level::SystemSafety,
        condition: Condition::parse(&serde_json::json!({
            "all": [{"fact": "effect_type", "eq": effect_type.as_str()}]
        }))
        .expect("effect_type 是封闭事实，取值取自枚举"),
        decision: Decision::Allow,
        scope: Scope::User,
    }
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

/// 第 8 步放弃工作区时**经 Gate 写了一条审计记录**（设计上篇第 6.4 节）。
///
/// `discard` 与三个写入 Base 的操作一样要入审计，变体映射是
/// `discard → AuditKind::ExternalEffects`。本条是该要求在本驱动里的唯一照片：把清理路径
/// 换回直接调 `discard_task_workspace`（本 task 之前正是如此）时，`kind` 那一行整个消失，
/// 而 `the_workspace_record_is_written_and_then_removed` 照绿——后者只看记录、目录与分支，
/// 看不见审计。
///
/// 不给 `--effect`：效应登记也写 `external effects` 的审计行（`continuum-effect` 的
/// `journal::audit`），带上它这条就分不清「多出来的一行」是谁写的。故本条的断言是
/// **恰一行**，而不是「至少一行」。
#[test]
fn discarding_the_workspace_writes_one_external_effects_audit_row() {
    const TEST: &str = "discarding_the_workspace_writes_one_external_effects_audit_row";
    if !require_auto_selected_sandbox(TEST) {
        return;
    }
    let (_d, base) = git_repo();
    let dbdir = tempfile::tempdir().unwrap();
    let db = dbdir.path().join("t.db");

    assert_success(&run_task(&base, &db, &["--exec", "true"], &[]));

    // 期望值**手写**在这里（钉 `kind` 列的编码），不调 `AuditKind::as_str` 去生成。
    let rows = audit_rows(&db);
    assert_eq!(rows.len(), 1, "第 8 步应恰写一条审计记录，实际 {rows:?}");
    assert_eq!(rows[0].0, "external effects", "变体映射应为 ExternalEffects");
    // payload 里点名是哪一种操作，且带 Intent 与后端——三者都在 Gate 的 `audit` 里拼。
    for needle in ["discard", "intent=i1", "backend=worktree"] {
        assert!(
            rows[0].1.contains(needle),
            "审计 payload 应含 {needle:?}，实际 {:?}",
            rows[0].1
        );
    }
    ran(TEST);
}

/// `--backend` 是一个**不存在**的选项：解析期即被拒，且拒绝发生在任何 I/O 之前。
///
/// 这条是承重断言，不是形式主义：上篇那条 Critical（`integrate_overlay` 删光 Base 的
/// `.git`）之所以打不到，正是因为没有任何入口能给出一个与记录不符的后端。
///
/// 它与 [`there_is_no_way_to_override_the_backend_from_the_command_line`] 是同一件事的两半，
/// 拆开写只有一个原因：**本用例不需要沙箱机制**（拒绝发生在解析期，驱动根本走不到选机制
/// 那一步），故它在**任何**机器上都跑；那一条要驱动真的跑起来，因而受
/// [`require_auto_selected_sandbox`] 门控、在没有隔离机制的机器上会跳过。承重的那一半
/// 不该跟着会跳过的那一半一起被跳过。
///
/// 它也是「未知选项」这条判据在 `--backend` 这个名字上的**唯一**照片：`tests/cli.rs` 的
/// `an_unknown_subcommand_or_option_is_rejected` 覆盖的是 `--apply` 与 `--base` 两个名字。
#[test]
fn the_backend_option_is_rejected_at_parse_time() {
    let (_d, base) = git_repo();
    let dbdir = tempfile::tempdir().unwrap();
    let db = dbdir.path().join("t.db");

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
}

/// 驱动放弃工作区时用的就是**落库记录里那个**后端。
///
/// 放弃若误用另一个后端：worktree 的工作区交给 overlay 后端会在 `require_layout` 处被拒
/// （那不是 overlay 布局），放弃不成，记录、Task 根与分支都会留着，下面三条断言随即变红。
///
/// **本用例单独分不出「取自记录」与「重新探测」**：在「Base 始终是同一个 git 仓库」这个
/// 前提下，`record.backend` 与现场 `detect_backend` 给出同一个值，两者不可分辨。真正把它
/// 分开的是 [`a_failed_discard_keeps_the_workspace_record`]（那条能把记录读回来看那一列），
/// M5 变红也靠它。唯一能让本臂具备该判别力的装置是「Base 在创建与回收之间改变形态」
/// （例如创建后 `git init`），本 task 没有造这个装置。
///
/// 入口那一半（`--backend` 被拒）在 [`the_backend_option_is_rejected_at_parse_time`]，
/// 那一条不设门控。
#[test]
fn there_is_no_way_to_override_the_backend_from_the_command_line() {
    const TEST: &str = "there_is_no_way_to_override_the_backend_from_the_command_line";
    if !require_auto_selected_sandbox(TEST) {
        return;
    }
    let (_d, base) = git_repo();
    let dbdir = tempfile::tempdir().unwrap();
    let db = dbdir.path().join("t.db");

    // 正常跑一次，放弃确实按记录里的 worktree 后端走完了。
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

// ── 第 7 步：`--apply` 的策略裁决与集成（Task 10）────────────────────────

/// 库里一条第 5 级 `Allow`，**不传 `--approve`**：集成照做，Base 出现改动。
///
/// 这是「铸造不必等 `--approve`」那一侧的接照片：设计第 5.7 节第一行。缺了它，
/// 一个「一律要求 `--approve`」的实现能通过下面所有「有 `--approve` 才放行」的用例。
#[test]
fn an_allowed_integration_is_applied_without_the_flag() {
    const TEST: &str = "an_allowed_integration_is_applied_without_the_flag";
    if !require_auto_selected_sandbox(TEST) {
        return;
    }
    let (_d, base) = git_repo();
    let dbdir = tempfile::tempdir().unwrap();
    let db = dbdir.path().join("t.db");
    seed_policies(&db, &[("r1", Level::RuntimeDefault, Decision::Allow)]);

    let out = run_task(
        &base,
        &db,
        &["--apply", "--exec", "sh", "-c", "echo applied > applied.txt"],
        &[],
    );
    assert_success(&out);

    // 判据（设计第 9 节）：Base 出现该改动、**Task 仍在**。
    assert_eq!(
        std::fs::read_to_string(base.join("applied.txt")).unwrap(),
        "applied\n",
        "集成应把 Task 的新文件落进 Base"
    );
    assert!(
        worktree_task_root(&base).exists(),
        "`--apply` 成功之后不清理工作区（设计第 4.2 节）"
    );
    assert_eq!(
        workspace_rows(&db).len(),
        1,
        "`--apply` 成功之后记录也留着"
    );
    ran(TEST);
}

/// 第 3 级 `RequireApproval` + 不传 `--approve`：**拒绝集成**，工作区保留。
///
/// 断言到**哪一种**拒绝（纪律 3）：信息里点名裁决值是「要求批准」，而不是笼统的非零退出
/// ——后者也可能是沙箱没起来、git 失败等等。
#[test]
fn a_require_approval_rule_needs_the_flag() {
    const TEST: &str = "a_require_approval_rule_needs_the_flag";
    if !require_auto_selected_sandbox(TEST) {
        return;
    }
    let (_d, base) = git_repo();
    let dbdir = tempfile::tempdir().unwrap();
    let db = dbdir.path().join("t.db");
    seed_policies(&db, &[("r1", Level::UserPersistent, Decision::RequireApproval)]);

    let out = run_task(
        &base,
        &db,
        &["--apply", "--exec", "sh", "-c", "echo nope > nope.txt"],
        &[],
    );
    assert!(
        !out.status.success(),
        "集成被拒时整条命令必须报失败\n--- stdout ---\n{}",
        stdout_of(&out)
    );
    let stderr = stderr_of(&out);
    assert!(
        stderr.contains("要求批准"),
        "应点名裁决为「要求批准」，实际 stderr：{stderr}"
    );
    assert!(
        !base.join("nope.txt").exists(),
        "被拒的集成不得在 Base 留下任何改动"
    );
    assert!(
        worktree_task_root(&base).exists(),
        "集成被拒时工作区必须保留——改动仍是完整可用的"
    );
    ran(TEST);
}

/// 第 3 级 `Deny` + `--approve` → **放行**（设计第 5.5 节）。
///
/// 与下一条方向相反，缺一不可：只留这一条，一个「`--approve` 一律放行」的实现全绿，
/// 而那正是把第 1 级也一起越过的 fail-open。
#[test]
fn the_flag_overrides_a_user_policy_deny() {
    const TEST: &str = "the_flag_overrides_a_user_policy_deny";
    if !require_auto_selected_sandbox(TEST) {
        return;
    }
    let (_d, base) = git_repo();
    let dbdir = tempfile::tempdir().unwrap();
    let db = dbdir.path().join("t.db");
    seed_policies(&db, &[("r1", Level::UserPersistent, Decision::Deny)]);

    let out = run_task(
        &base,
        &db,
        &[
            "--apply",
            "--approve",
            "--exec",
            "sh",
            "-c",
            "echo ok > allowed.txt",
        ],
        &[],
    );
    assert_success(&out);
    assert_eq!(
        std::fs::read_to_string(base.join("allowed.txt")).unwrap(),
        "ok\n",
        "第 3 级的 Deny 应被 --approve 越过，改动落进 Base"
    );
    ran(TEST);
}

/// **第 1 级 `Deny` + `--approve` → 仍拒**（设计第 5.5 节：唯一越不过的那一级）。
///
/// 与上一条是同一条命令结构、只换了规则所在层级。上一条绿而这条红，才说明「第 1 级
/// 不可越」是**层级**在起作用，而不是「`--approve` 根本没接线」。
#[test]
fn the_flag_cannot_override_system_safety() {
    const TEST: &str = "the_flag_cannot_override_system_safety";
    if !require_auto_selected_sandbox(TEST) {
        return;
    }
    let (_d, base) = git_repo();
    let dbdir = tempfile::tempdir().unwrap();
    let db = dbdir.path().join("t.db");
    seed_policies(&db, &[("r1", Level::SystemSafety, Decision::Deny)]);

    let out = run_task(
        &base,
        &db,
        &[
            "--apply",
            "--approve",
            "--exec",
            "sh",
            "-c",
            "echo bad > bad.txt",
        ],
        &[],
    );
    assert!(
        !out.status.success(),
        "第 1 级的 Deny 不得被 --approve 越过\n--- stdout ---\n{}",
        stdout_of(&out)
    );
    let stderr = stderr_of(&out);
    assert!(
        stderr.contains("禁止"),
        "应点名裁决为「禁止」（与上一条的「要求批准」可分辨），实际 stderr：{stderr}"
    );
    assert!(
        !base.join("bad.txt").exists(),
        "第 1 级 Deny 下 Base 必须一字未动"
    );
    assert!(
        worktree_task_root(&base).exists(),
        "被拒时工作区保留（同样是完整的改动）"
    );
    ran(TEST);
}

/// 集成被拒后**工作区与记录都在**——三样都断言（Task 根、记录、分支）。
///
/// 设计第 4.2 节：「丢弃一份未被批准的改动会让用户无从恢复」。三样分开断言是因为它们
/// 由不同代码路径产生：Task 根在磁盘上、记录在库里、分支在 git 里；只断言其中一样时，
/// 另两样被误删不会有对照片。
#[test]
fn a_refused_integration_keeps_the_workspace() {
    const TEST: &str = "a_refused_integration_keeps_the_workspace";
    if !require_auto_selected_sandbox(TEST) {
        return;
    }
    let (_d, base) = git_repo();
    let dbdir = tempfile::tempdir().unwrap();
    let db = dbdir.path().join("t.db");
    seed_policies(&db, &[("r1", Level::UserPersistent, Decision::RequireApproval)]);

    let out = run_task(
        &base,
        &db,
        &["--apply", "--exec", "sh", "-c", "echo kept > kept.txt"],
        &[],
    );
    assert!(!out.status.success(), "被拒的集成不得报成功");
    // 断言到**哪一种**拒绝（纪律 3）：规则是第 3 级 `RequireApproval`，故信息里应点名
    // 「要求批准」，而不是笼统的非零退出——后者也可能是沙箱没起来、git 失败等等。
    let stderr = stderr_of(&out);
    assert!(
        stderr.contains("要求批准"),
        "应点名裁决为「要求批准」，实际 stderr：{stderr}"
    );

    assert!(
        worktree_task_root(&base).exists(),
        "Task 根被删了：{}",
        worktree_task_root(&base).display()
    );
    assert_eq!(
        workspace_rows(&db).len(),
        1,
        "记录被删了，用户再也找不到那份改动"
    );
    assert!(
        local_branches(&base).contains(&format!("ai/{INTENT}")),
        "worktree 后端的分支被删了"
    );
    // Base 一字未动：顶层仍是 `.ai/`（worktree 后端的私有子树）、`.git/` 与用户的文件。
    assert_eq!(
        dir_entries(&base),
        vec![".ai", ".git", "README.md"],
        "被拒的集成在 Base 留下了东西"
    );
    // 那份没被批准的改动确实还在 Task 里（「无从恢复」的反面）。
    assert_eq!(
        std::fs::read_to_string(worktree_task_root(&base).join("kept.txt")).unwrap(),
        "kept\n",
        "被拒后 Task 里的改动应原样留着"
    );
    ran(TEST);
}

/// 命令退出码非 0：**不做集成**，但**照旧清理工作区**（设计第 4.2 节）。
///
/// 两个方向都承重。**不集成**：库里放一条「什么都放行」的第 5 级 `Allow`，若实现仍去
/// 集成，Base 会拿到半成品而调用方拿到的是失败。**仍清理**：这一支留下的是半成品，
/// 而记录是裸 `INSERT`，留着会让同一个 Intent 再也创建不了。
#[test]
fn a_failed_command_is_not_integrated_and_the_workspace_is_cleaned() {
    const TEST: &str = "a_failed_command_is_not_integrated_and_the_workspace_is_cleaned";
    if !require_auto_selected_sandbox(TEST) {
        return;
    }
    let (_d, base) = git_repo();
    let dbdir = tempfile::tempdir().unwrap();
    let db = dbdir.path().join("t.db");
    // 一条恒真的第 5 级 Allow：只要驱动走第 7 步，半成品就会落进 Base。
    seed_policies(&db, &[("r1", Level::RuntimeDefault, Decision::Allow)]);

    let out = run_task(
        &base,
        &db,
        &[
            "--apply",
            "--exec",
            "sh",
            "-c",
            "echo half > partial.txt; exit 3",
        ],
        &[],
    );
    // 失败是哪一种：命令自己以退出码 3 结束，不是策略拒绝、也不是沙箱没起来。
    assert!(!out.status.success(), "命令失败时整条命令必须报失败");
    let stderr = stderr_of(&out);
    assert!(
        stderr.contains("以退出码 3"),
        "期望「命令以退出码 3 失败」，实际 stderr：{stderr}"
    );
    assert!(
        !base.join("partial.txt").exists(),
        "命令失败时不得把半成品集成进 Base（第 7 步不走）"
    );
    assert!(
        workspace_rows(&db).is_empty(),
        "命令失败时仍要收尾：记录应被清掉，否则同一个 Intent 再也创建不了"
    );
    assert!(
        !worktree_task_root(&base).exists(),
        "命令失败时仍要收尾：Task 根应被清掉"
    );
    assert_eq!(
        local_branches(&base),
        vec!["main".to_owned()],
        "命令失败时仍要收尾：分支应被回收"
    );
    ran(TEST);
}

// ── 第 4、6 步：效应声明与 Journal 接线（Task 11）─────────────────────────

/// 各 `--effect` 在执行**之前**已写入 Journal，且命令看到的状态是 `EXECUTING`。
///
/// 判据（设计第 10 节判据 3）：命令在**自己的第一条指令**上读 Journal 的落库字节，
/// 读到 `executing` 才印 `EFFECT-EXECUTING`。命令若在记录写入之前就跑（或只写了
/// `PLANNED`/`AUTHORIZED`），这个标记不会出现。
///
/// **观察到的是「库文件里出现了 `executing` 这一串」，比单看状态列宽**：`effect.state`
/// 列的编码（设计第 8 节，小写）会写它，`journal.rs` 的 `advance` 在写审计 payload 时
/// 也会写 `"to":"executing"`（`crates/continuum-effect/src/journal.rs`）。`grep` 分不出
/// 这两处，故本用例的判据不能写成「读到的就是 `effect.state` 列」。**这不削弱它**：
/// 两处都只在第 4 步写入之后才可能出现，命令若抢在写入之前跑，哪一种都不会有。
///
/// **本用例显式点名 bubblewrap**：Landlock 的只读白名单是 `/usr /lib /lib64 /etc /bin
/// /sbin`（`continuum-sandbox/src/landlock.rs`），库所在的临时目录不在其中，命令在
/// Landlock 下连库文件都打不开；bubblewrap 的 `--ro-bind / /` 才让库**可读**。
/// 无 bwrap 的机器上显式跳过（带执行/跳过条数），不静默换一个读不到 Journal 的机制。
///
/// 库是 WAL 模式（`continuum-persist` 的 `Db::open_with`），刚提交的状态在 `<库>-wal`
/// 里，故两个文件都查。
///
/// **P3 起库里要先有放行这条效应的规则**：强制点 (2) 在执行前逐条问策略，铸不出能力即
/// 拒绝运行命令（`continuum-runtime/tests/capability_gate.rs`）。故这里放一条第 5 级
/// `Allow`——本用例验的是「写入早于执行」，不是「铸不出会怎样」。
#[test]
fn effects_are_recorded_before_the_command_runs() {
    const TEST: &str = "effects_are_recorded_before_the_command_runs";
    if !continuum_sandbox::Sandbox::bubblewrap()
        .capabilities()
        .restricts_filesystem_writes
    {
        skip(
            TEST,
            "PATH 中找不到 bwrap：Landlock 下命令读不到 Journal，本判据无法验",
        );
        return;
    }
    let (_d, base) = git_repo();
    let dbdir = tempfile::tempdir().unwrap();
    let db = dbdir.path().join("t.db");
    seed_policies(&db, &[("r1", Level::RuntimeDefault, Decision::Allow)]);

    let out = run_task(
        &base,
        &db,
        &[
            "--sandbox",
            "bubblewrap",
            "--effect",
            "push_branch:origin/main",
            "--exec",
            "sh",
            "-c",
            "if grep -q executing \"$PROBE_DB\" \"$PROBE_DB-wal\" 2>/dev/null; \
             then echo EFFECT-EXECUTING; else echo EFFECT-OTHER; fi",
        ],
        &[("PROBE_DB", db.as_os_str())],
    );
    assert_success(&out);
    let stdout = stdout_of(&out);
    assert!(
        stdout.contains("EFFECT-EXECUTING"),
        "命令执行时该效应的状态应已是 EXECUTING（写入早于执行）\n--- stdout ---\n{stdout}"
    );
    assert!(
        !stdout.contains("EFFECT-OTHER"),
        "两个标记不该同时出现\n--- stdout ---\n{stdout}"
    );
    ran(TEST);
}

/// 命令执行途中驱动被杀：记录停在 `EXECUTING`，恢复把它转成 `UNKNOWN`（**不猜**）。
///
/// 判据（设计第 10 节判据 4）：杀的是**驱动进程**——命令阻塞在 `sleep` 上，测试先看到
/// 命令已开始的标记（故第 4 步的记录已提交），再把驱动杀死。**不是命令自杀**：命令自杀
/// 时驱动还活着，会照第 6 步写 `FAILED`，那样验不到「崩溃留下 EXECUTING」。
///
/// 恢复在**测试进程内**用 `run_recovery` 跑（`recover` 子命令是 Task 12，本 task 不
/// 依赖它）。断言转 `UNKNOWN` 且**没有**变成 `COMMITTED`/`FAILED`——设计第 6.4 节：
/// `UNKNOWN` 是「不知道」，不得为了干净并入「确定没成功」。
#[test]
fn a_killed_command_leaves_executing_and_recovery_turns_it_unknown() {
    const TEST: &str = "a_killed_command_leaves_executing_and_recovery_turns_it_unknown";
    if !require_auto_selected_sandbox(TEST) {
        return;
    }
    let (_d, base) = git_repo();
    let dbdir = tempfile::tempdir().unwrap();
    let db = dbdir.path().join("t.db");
    // P3 起强制点 (2) 要求每一条 `--effect` 都能铸出能力，否则命令一步都不跑；本用例
    // 要命令真的跑起来，故先放一条第 5 级 `Allow`。
    seed_policies(&db, &[("r1", Level::RuntimeDefault, Decision::Allow)]);

    // 命令先写「已开始」再长睡。stdio 接到 null：被杀的驱动与仍活着的 `sleep` 子进程
    // 都会持有管道的写端，接管道会让测试在读取端空等。
    let mut driver = Command::new(BIN)
        .args(["task", "--base"])
        .arg(&base)
        .args(["--intent", INTENT, "--db"])
        .arg(&db)
        .args([
            "--effect",
            "charge:acct_1",
            "--exec",
            "sh",
            "-c",
            "echo started > started.txt; sleep 30",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("continuum-runtime 无法执行");

    let started = worktree_task_root(&base).join("started.txt");
    let deadline = Instant::now() + Duration::from_secs(20);
    while !started.exists() {
        if Instant::now() > deadline {
            let _ = driver.kill();
            let _ = driver.wait();
            panic!("命令 20 秒内没有开始：夹具或实现有问题，本判据没验到");
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    // 此刻驱动阻塞在子进程的 `wait()` 上；杀死它，让记录停在 EXECUTING。
    driver.kill().expect("无法杀死驱动进程");
    let status = driver.wait().expect("等待驱动进程失败");
    assert!(!status.success(), "被杀的驱动不该正常退出，实际 {status:?}");

    // 终态由驱动在命令结束后才写，而命令还在 sleep，故此刻记录必须仍是 EXECUTING。
    assert_eq!(
        effect_states(&db),
        vec!["executing".to_owned()],
        "驱动在命令执行途中被杀，记录应停在 EXECUTING"
    );

    // 在测试进程内跑恢复（不经 `recover` 子命令）。
    {
        let handle = Db::open(&db).expect("打开数据库失败");
        let mut registry = RecoveryRegistry::new();
        registry.register(Box::new(MarkExecutingAsUnknown::new()));
        run_recovery(&handle, &registry).expect("恢复失败");
    }

    let states = effect_states(&db);
    assert_eq!(states.len(), 1, "应恰有一条效应记录，实际 {states:?}");
    assert_eq!(
        states[0], "unknown",
        "EXECUTING 应被恢复转成 UNKNOWN，实际 {states:?}"
    );
    // 逐项各断言一次：`UNKNOWN` 不得被并入任一「确定」态（设计第 6.4 节）。
    assert_ne!(states[0], "committed", "不得把 UNKNOWN 猜成 COMMITTED");
    assert_ne!(states[0], "failed", "不得把 UNKNOWN 猜成 FAILED");
    ran(TEST);
}

/// 同 intent / 类型 / 目标跑第二次：拒绝**整条**命令，且库里仍只有一条记录。
///
/// 「命令自身的效应没发生第二次」的判断方法：命令印一个标记到 stdout。未 `--apply` 时
/// Task 根随后即被清理，写在文件里观察不到；stdout 由子进程继承、直达测试的管道，故
/// 看得见。第二次的 stdout 里不该有这个标记——**若实现只是静默跳过效应而照跑命令，
/// 标记就会出现**，这正是本条与「拒绝」的分界。
#[test]
fn the_same_idempotency_key_refuses_to_run_again() {
    const TEST: &str = "the_same_idempotency_key_refuses_to_run_again";
    if !require_auto_selected_sandbox(TEST) {
        return;
    }
    let (_d, base) = git_repo();
    let dbdir = tempfile::tempdir().unwrap();
    let db = dbdir.path().join("t.db");
    // 第一次要真的跑起来（强制点 (2) 要求这条效应能铸出能力），放一条第 5 级 `Allow`。
    seed_policies(&db, &[("r1", Level::RuntimeDefault, Decision::Allow)]);
    let after_db = [
        "--effect",
        "deploy:prod",
        "--exec",
        "sh",
        "-c",
        "echo COMMAND-RAN",
    ];

    let first = run_task(&base, &db, &after_db, &[]);
    assert_success(&first);
    assert!(
        stdout_of(&first).contains("COMMAND-RAN"),
        "第一次应真的执行命令\n--- stdout ---\n{}",
        stdout_of(&first)
    );
    assert_eq!(effect_states(&db), vec!["committed".to_owned()]);

    let second = run_task(&base, &db, &after_db, &[]);
    assert!(
        !second.status.success(),
        "同键的第二次必须拒绝整条命令\n--- stdout ---\n{}",
        stdout_of(&second)
    );
    let stderr = stderr_of(&second);
    assert!(
        stderr.contains("幂等键"),
        "应点名幂等键冲突，实际 stderr：{stderr}"
    );
    assert!(
        !stdout_of(&second).contains("COMMAND-RAN"),
        "被拒的命令不得执行（命令自身的效应不该发生第二次）\n--- stdout ---\n{}",
        stdout_of(&second)
    );
    assert_eq!(
        effect_states(&db),
        vec!["committed".to_owned()],
        "库里仍应只有一条记录"
    );
    // 被拒之后工作区与记录照第 8 步清理：`save_workspace` 是裸 INSERT，留着会让同一个
    // Intent 再也创建不了。
    assert!(
        workspace_rows(&db).is_empty(),
        "被拒后应清理工作区记录，实际 {:?}",
        workspace_rows(&db)
    );
    ran(TEST);
}

/// 退出码 0：**每一条**声明都被写成 `COMMITTED`。
///
/// 逐条断言（类型、目标、状态三者一起），不只看条数：条数对而状态串错、或两条互相
/// 串了类型/目标，都能让只数条数的断言通过。
#[test]
fn a_successful_command_commits_all_declared_effects() {
    const TEST: &str = "a_successful_command_commits_all_declared_effects";
    if !require_auto_selected_sandbox(TEST) {
        return;
    }
    let (_d, base) = git_repo();
    let dbdir = tempfile::tempdir().unwrap();
    let db = dbdir.path().join("t.db");
    // 每条 `--effect` 都要能铸出能力，命令才会跑（强制点 (2)），放一条第 5 级 `Allow`。
    seed_policies(&db, &[("r1", Level::RuntimeDefault, Decision::Allow)]);

    let out = run_task(
        &base,
        &db,
        &[
            "--effect",
            "charge:acct_1",
            "--effect",
            "deploy:prod",
            "--exec",
            "true",
        ],
        &[],
    );
    assert_success(&out);
    assert_eq!(
        effect_rows(&db),
        vec![
            (
                "charge".to_owned(),
                "acct_1".to_owned(),
                "committed".to_owned()
            ),
            (
                "deploy".to_owned(),
                "prod".to_owned(),
                "committed".to_owned()
            ),
        ],
        "每一条声明的效应都应被写成 COMMITTED"
    );
    ran(TEST);
}

/// 退出码非 0：**每一条**声明都被写成 `FAILED`，且没有一条停在 `EXECUTING`。
///
/// 「没有停在 EXECUTING」单独断言一次（设计第 6.4 节）：`FAILED` 是「确定没成功」，
/// 与 `UNKNOWN` 的「不知道」是两回事；一条把命令失败也留在 EXECUTING 的实现会让恢复
/// 钩子后来把它误转成 UNKNOWN。
#[test]
fn a_failing_command_fails_all_declared_effects() {
    const TEST: &str = "a_failing_command_fails_all_declared_effects";
    if !require_auto_selected_sandbox(TEST) {
        return;
    }
    let (_d, base) = git_repo();
    let dbdir = tempfile::tempdir().unwrap();
    let db = dbdir.path().join("t.db");
    // 每条 `--effect` 都要能铸出能力，命令才会跑（强制点 (2)），放一条第 5 级 `Allow`。
    seed_policies(&db, &[("r1", Level::RuntimeDefault, Decision::Allow)]);

    let out = run_task(
        &base,
        &db,
        &[
            "--effect",
            "charge:acct_1",
            "--effect",
            "deploy:prod",
            "--exec",
            "sh",
            "-c",
            "exit 3",
        ],
        &[],
    );
    assert!(!out.status.success(), "命令失败时整条命令必须报失败");
    // 断言到**哪一种**失败：命令自己以退出码 3 结束，不是别的错误。
    assert!(
        stderr_of(&out).contains("以退出码 3"),
        "期望「命令以退出码 3 失败」，实际 stderr：{}",
        stderr_of(&out)
    );
    assert_eq!(
        effect_rows(&db),
        vec![
            (
                "charge".to_owned(),
                "acct_1".to_owned(),
                "failed".to_owned()
            ),
            ("deploy".to_owned(), "prod".to_owned(), "failed".to_owned()),
        ],
        "每一条声明的效应都应被写成 FAILED"
    );
    for state in effect_states(&db) {
        assert_ne!(
            state, "executing",
            "失败的命令不得让记录停在 EXECUTING（设计第 6.4 节）"
        );
    }
    ran(TEST);
}

/// 多条 `--effect` 里**任意一条**的键已存在即拒绝**整条**命令——不是只跳过那一条。
///
/// 与 `the_same_idempotency_key_refuses_to_run_again` 的分界：那一条只声明一条 `--effect`，
/// 分不出「拒绝整条」与「只拒绝/跳过那一条」；本条先声明一条**新**的 `publish`，再声明
/// 一条**已登记**的 `push_branch`。若实现只跳过冲突的那一条，`publish` 会被写进库里、
/// 命令也会照跑——两条断言随即变红。这是设计第 6.5 节「已存在记录即拒绝运行整条命令」
/// 的字面落点（多效应下的「整条」）。
#[test]
fn any_declared_key_existing_refuses_the_whole_command() {
    const TEST: &str = "any_declared_key_existing_refuses_the_whole_command";
    if !require_auto_selected_sandbox(TEST) {
        return;
    }
    let (_d, base) = git_repo();
    let dbdir = tempfile::tempdir().unwrap();
    let db = dbdir.path().join("t.db");
    // 种下 (i1, push_branch, origin/main)；命令里 publish:t1 是新的一条。
    seed_effect(&db, INTENT, EffectType::PushBranch, "origin/main");

    let out = run_task(
        &base,
        &db,
        &[
            "--effect",
            "publish:t1",
            "--effect",
            "push_branch:origin/main",
            "--exec",
            "sh",
            "-c",
            "echo COMMAND-RAN",
        ],
        &[],
    );

    assert!(
        !out.status.success(),
        "任意一条键已存在即拒绝整条命令\n--- stdout ---\n{}",
        stdout_of(&out)
    );
    assert!(
        stderr_of(&out).contains("幂等键"),
        "应点名幂等键冲突，实际 stderr：{}",
        stderr_of(&out)
    );
    // **整条**被拒的分界：那条**新**声明的 publish 不得落库。只跳过冲突那条的实现会在这里变红。
    assert_eq!(
        effect_rows(&db),
        vec![(
            "push_branch".to_owned(),
            "origin/main".to_owned(),
            "planned".to_owned()
        )],
        "被拒的整条命令不得写下任何一条新效应"
    );
    assert!(
        !stdout_of(&out).contains("COMMAND-RAN"),
        "被拒的命令不得执行（命令自身的效应不该发生）\n--- stdout ---\n{}",
        stdout_of(&out)
    );
    ran(TEST);
}

/// `authorization` 列写下「`--approve` 是否给出」与「策略的裁决结果」两件事
/// （设计第 6.7 节）。
///
/// **三种裁决各一张照片**，加上 `--approve` 的两个取值——枚举式的要求不能抽一个代表：
/// 只测 `deny` 一种的话，把 `Decision::as_str` 的三条分支串错（例如
/// `require_approval` 与 `deny` 对调）不会有任何用例变红。期望值**手写字面量**，
/// 不调 `Decision::as_str` 去拼：拼出来的期望值会跟着实现一起漂移，钉不住格式。
///
/// 三次调用只有裁决那一路不同：先是一条**按效应限定**的第 1 级 `Allow`（集成那次裁决
/// 匹配不到它 → 默认 `Deny`），再放一条无条件的第 3 级 `RequireApproval`（集成那次 →
/// `RequireApproval`），最后传 `--approve`（第 2 级越过第 3 级 → `Allow`）。三次用不同
/// 的目标，避免撞上幂等键。
///
/// **P3 起还有一层要求**：强制点 (2) 逐条问策略，`charge` 那一条铸不出能力就不跑命令
/// （`capability_gate.rs`）。故每一次里 `charge` 的效应裁决都必须是 `Allow`——由那条
/// 按 `effect_type` 限定的第 1 级 `Allow` 提供（它压过第 3 级的 `RequireApproval`）。
/// 于是**本用例同时是两种裁决并存的照片**：同一次运行里，集成那次是 `Deny`/
/// `RequireApproval`，而 `charge` 那条效应是 `Allow`。
#[test]
fn the_authorization_field_records_the_flag_and_the_verdict() {
    const TEST: &str = "the_authorization_field_records_the_flag_and_the_verdict";
    if !require_auto_selected_sandbox(TEST) {
        return;
    }
    let (_d, base) = git_repo();
    let dbdir = tempfile::tempdir().unwrap();
    let db = dbdir.path().join("t.db");
    seed_policy_rows(&db, &[("r_eff", allow_only_effect(EffectType::Charge))]);

    // 一：集成那次裁决匹配不到任何规则 → 默认 Deny；未给 --approve。
    assert_success(&run_task(
        &base,
        &db,
        &["--effect", "charge:a1", "--exec", "true"],
        &[],
    ));
    assert_eq!(
        authorization_of(&db, "a1"),
        "approve=false;policy=deny",
        "无匹配默认 Deny 应连同「未给 --approve」一起写进 authorization"
    );

    // 二：第 3 级 RequireApproval，未给 --approve → RequireApproval。
    seed_policies(&db, &[("r1", Level::UserPersistent, Decision::RequireApproval)]);
    assert_success(&run_task(
        &base,
        &db,
        &["--effect", "charge:a2", "--exec", "true"],
        &[],
    ));
    assert_eq!(
        authorization_of(&db, "a2"),
        "approve=false;policy=require_approval",
        "RequireApproval 的编码应逐字写进 authorization"
    );

    // 三：同一条规则，给出 --approve → 第 2 级的 Allow 越过第 3 级。
    assert_success(&run_task(
        &base,
        &db,
        &["--effect", "charge:a3", "--approve", "--exec", "true"],
        &[],
    ));
    assert_eq!(
        authorization_of(&db, "a3"),
        "approve=true;policy=allow",
        "给出 --approve 时裁决为 Allow，两个事实都要写进 authorization"
    );
    ran(TEST);
}
