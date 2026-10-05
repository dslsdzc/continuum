//! 强制点 (2)：执行点的 Capability 校验（设计第 4.1、4.2 节）。
//!
//! 驱动按「命令即效应」运行调用方给的命令，**命令跑起来就是副作用**，故校验排在
//! `Sandbox::spawn` 之前：对命令声明的那几条 `--effect` 逐条问策略，据裁决铸出
//! `Capability`；**铸不出即拒绝运行整条命令**（一条都不跑）。
//!
//! # 这里验的是「接线」，不是六格表本身
//!
//! 「裁决值 × 是否附了显式确认 → 铸不铸」那张六格表只有一个产生点：驱动的 `mints`
//! （`src/task_cmd.rs`），它的逐格照片在 `task_cmd.rs` 的
//! `the_mapping_from_a_decision_to_minting_has_six_cells`。本文件验的是**驱动有没有按
//! 那个裁决行事**——三条臂各一条端到端用例（`Allow` / `RequireApproval` / `Deny`），
//! 因为换掉 `mints` 的入参（例如把 `args.approve` 写成常量）在只测一条臂时不会变红。
//!
//! # 为什么没有「能力已登记」这条判据（与 P2 的「写入早于执行」同法的那条）
//!
//! 计划的 Task 7 原稿写过一条 `the_capability_is_checked_before_the_command_runs`，
//! 要求命令在**读库**时观察到「自己的能力已登记」。**这条写不出来，也不许假装**：
//! `Capability` 与凭据**不落库**（设计 §2.3「凭据不落库」、§7），库里的 `effect` 行
//! 记的是「授权是怎么来的」的描述（`authorization` 列，P2 第 6.7 节），**不是凭据**。
//! 故「能力已登记」在库里没有任何对应物可读。
//!
//! 诚实的类似物是 P2 那条既有判据本身：**效应行在命令跑起来之前就已写入**
//! （`AUTHORIZED → EXECUTING`），命令读得到。本文件用
//! [`the_effects_are_registered_before_the_command_runs_under_the_gate`] 把它在**强制点
//! 就位之后**重新钉一遍——插进来的这道校验若把次序弄反，本条会红。
//!
//! **这一条钉不住「校验发生在命令之前」**：两种次序下效应行都在命令前写好。真正钉住
//! 次序的是 [`an_effect_without_a_capability_refuses_to_run_the_command`]——被拒时命令
//! 一步都没跑（stdout 上没有任何命令的痕迹）。
//!
//! # 与集成批准（`GateApproval`）并存
//!
//! 能力绑的是「**这一类动作**准不准」，`GateApproval` 绑的是「**这一次集成**准不准」，
//! 两者互不取代（设计 §4.3）。**并存关系的照片沿用 P2 的既有用例，此处不新写**：
//! `crates/continuum-workspace/tests/gate.rs` 的
//! `an_approval_for_one_integration_does_not_authorize_another`（拿一次集成的批准值去
//! 用另一次，必须 `ApprovalMismatch`）。本文件不重复它，因为能力侧没有可与它对照的
//! 「另一枚能力」——能力按 kind 与 scope 各自成立，不存在「拿 A 能力的批准去用 B 集成」
//! 这条通道。
//!
//! 全部经 `env!("CARGO_BIN_EXE_continuum-runtime")` 驱动**真实二进制**；只声明效应的
//! 用例需要驱动能真的选出沙箱机制（选不出时 `run` 在沙箱那一步就失败，验不到强制点），
//! 故受 [`require_auto_selected_sandbox`] 门控，不静默通过。
//!
//! 夹具与 `task_cli.rs` 重复一份：测试 crate 之间取不到对方的私有夹具，而本仓对「同一
//! 件事两个来源」的判据只针对产品代码（`continuum-workspace` 的 `tests/common` 同样是
//! 另一份）。

use continuum_effect::EffectType;
use continuum_persist::{Db, Value};
use continuum_policy::{Condition, Decision, Level, Policy, Scope};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const BIN: &str = env!("CARGO_BIN_EXE_continuum-runtime");
const INTENT: &str = "i1";

// ── 夹具 ────────────────────────────────────────────────────────────────

/// 建一个真实 Git 仓库作 Base，返回（保活用的 `TempDir`，规范化的仓库根）。
fn git_repo() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    run_git(&root, &["init", "-b", "main"]);
    run_git(&root, &["config", "user.email", "test@example.invalid"]);
    run_git(&root, &["config", "user.name", "Continuum 测试"]);
    std::fs::write(root.join("README.md"), "初始内容\n").unwrap();
    run_git(&root, &["add", "-A"]);
    run_git(&root, &["commit", "-m", "初始提交"]);
    (dir, root)
}

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

fn local_branches(dir: &Path) -> Vec<String> {
    run_git(dir, &["branch", "--format=%(refname:short)"])
        .lines()
        .map(|l| l.trim().to_owned())
        .filter(|l| !l.is_empty())
        .collect()
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

/// 库里 `workspace` 表的全部行数（本文件只关心「有没有留下记录」）。
fn workspace_row_count(db: &Path) -> usize {
    let db = Db::open(db).expect("打开数据库失败");
    let tx = db.begin().unwrap();
    let rows = tx
        .query("SELECT intent_id FROM workspace", &[])
        .expect("读 workspace 表失败");
    tx.commit().unwrap();
    rows.len()
}

/// 库里 `effect` 表的全部行：`(effect_type, target, state)`，按类型与目标排序。
///
/// 直接读列而不经 `continuum_effect::load_effect`：要看的正是**落库编码**（两个枚举列的
/// 小写文本），类型化读回会把这一层盖掉（与 `task_cli.rs` 同法）。
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

/// 一条条件恒真（空合取）的规则：任何 `PolicyContext` 都成立。
fn always(level: Level, decision: Decision) -> Policy {
    Policy {
        level,
        condition: Condition::parse(&serde_json::json!({"all": []}))
            .expect("空合取是合法条件"),
        decision,
        scope: Scope::User,
    }
}

/// 一条**只对某一种效应**成立的 `Allow`（第 5 级）。上下文里没有 `effect_type` 时
/// 它不成立（`continuum_policy::rule` 的 `Fact::EffectType`：缺省即不成立）。
///
/// 用它把「逐条效应」这一维变得可观察：集成那次裁决（`effect_type` 为 `None`）与
/// 别的效应都不会被它放行。
fn allow_effect(effect_type: EffectType) -> Policy {
    Policy {
        level: Level::RuntimeDefault,
        condition: Condition::parse(&serde_json::json!({
            "all": [{"fact": "effect_type", "eq": effect_type.as_str()}]
        }))
        .expect("effect_type 是封闭事实，取值取自枚举"),
        decision: Decision::Allow,
        scope: Scope::User,
    }
}

/// 建出 `policy` 表并写入若干规则，供随后启动的驱动读回（经 `save_policy`，不手写 SQL
/// 字面量：手抄编码会给同一件事立第二个来源）。
fn seed_policy_rows(db: &Path, rules: &[(&str, Policy)]) {
    let handle = Db::open_with(db, continuum_policy::p2_policy_migrations()).unwrap();
    handle.migrate().unwrap();
    let tx = handle.begin().unwrap();
    for (id, policy) in rules {
        continuum_policy::save_policy(&tx, id, policy).unwrap();
    }
    tx.commit().unwrap();
}

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

fn skip(test: &str, reason: &str) {
    eprintln!("【跳过】{test}：{reason}。本条：执行 0、跳过 1。用 `--nocapture` 可见本行。");
}

fn ran(test: &str) {
    eprintln!("【运行】{test}：本条：执行 1、跳过 0。");
}

/// 本机是否有至少一个可用的沙箱机制（不给 `--sandbox` 时驱动自动选）。
fn any_sandbox_mechanism_available() -> bool {
    continuum_sandbox::Sandbox::landlock()
        .capabilities()
        .restricts_filesystem_writes
        || continuum_sandbox::Sandbox::bubblewrap()
            .capabilities()
            .restricts_filesystem_writes
}

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

/// `Deny` 那条臂：铸不出能力 ⇒ **命令一步都没跑**，退出码非零且是**点名了那条效应**的
/// 具体错误；已登记的效应记录与工作区一并清理（否则同一个 Intent 再也创建不了）。
///
/// 库里一条规则都没有 ⇒ 逐条效应的裁决是「无匹配默认 `Deny`」（设计下篇第 5.3 节）⇒
/// 铸不出。**这一条同时钉住「校验发生在命令之前」**：被拒时 stdout 上没有任何命令的
/// 痕迹——若实现先跑命令再校验，`COMMAND-RAN` 已经印出来了。
///
/// 末尾的**对照臂**（同一条命令、同一个 Intent，库里放一条第 5 级 `Allow`）一举三得：
/// 证明「被拒」不是沙箱/git 造成的；证明被拒之后的清理确实让同一 Intent 可以重跑；
/// 也是「`Allow` 那条臂」在**同一夹具**下的照片。
#[test]
fn an_effect_without_a_capability_refuses_to_run_the_command() {
    const TEST: &str = "an_effect_without_a_capability_refuses_to_run_the_command";
    if !require_auto_selected_sandbox(TEST) {
        return;
    }
    let (_d, base) = git_repo();
    let dbdir = tempfile::tempdir().unwrap();
    let db = dbdir.path().join("t.db");
    let after_db = [
        "--effect",
        "charge:acct_1",
        "--exec",
        "sh",
        "-c",
        "echo COMMAND-RAN",
    ];

    let refused = run_task(&base, &db, &after_db, &[]);
    assert!(
        !refused.status.success(),
        "铸不出能力时必须拒绝运行命令\n--- stdout ---\n{}",
        stdout_of(&refused)
    );
    let stderr = stderr_of(&refused);
    assert!(
        stderr.contains("charge:acct_1"),
        "错误应点名是哪一条效应，实际 stderr：{stderr}"
    );
    assert!(
        stderr.contains("禁止"),
        "错误应点名裁决是「禁止」（与「要求批准」可分辨），实际 stderr：{stderr}"
    );
    assert!(
        !stdout_of(&refused).contains("COMMAND-RAN"),
        "命令不得执行——校验排在命令之前\n--- stdout ---\n{}",
        stdout_of(&refused)
    );

    // 记录与工作区都不留：拒绝时命令一步没跑，留着记录会让同一个 Intent 再也创建不了。
    assert!(
        effect_rows(&db).is_empty(),
        "被拒的命令不得留下任何效应记录，实际 {:?}",
        effect_rows(&db)
    );
    assert_eq!(
        workspace_row_count(&db),
        0,
        "被拒之后工作区记录应被清理"
    );
    assert!(
        !worktree_task_root(&base).exists(),
        "被拒之后 Task 根应被清理"
    );
    assert_eq!(
        local_branches(&base),
        vec!["main".to_owned()],
        "被拒之后 worktree 分支应被回收"
    );

    // 对照臂：库里放一条第 5 级 Allow，同一条命令照常跑起来。
    seed_policy_rows(&db, &[("r1", always(Level::RuntimeDefault, Decision::Allow))]);
    let accepted = run_task(&base, &db, &after_db, &[]);
    assert_success(&accepted);
    assert!(
        stdout_of(&accepted).contains("COMMAND-RAN"),
        "对照臂里命令应真的执行\n--- stdout ---\n{}",
        stdout_of(&accepted)
    );
    ran(TEST);
}

/// `Allow` 那条臂：铸得出能力 ⇒ 命令跑起来，Journal 按既有约定落库（退出码 0 ⇒
/// `COMMITTED`）。
///
/// 与上一条同一夹具结构，只差库里有没有那条 `Allow`——两条互为对照臂，故「被拒/放行」
/// 不是别的原因造成的。
#[test]
fn an_effect_with_a_capability_runs_the_command() {
    const TEST: &str = "an_effect_with_a_capability_runs_the_command";
    if !require_auto_selected_sandbox(TEST) {
        return;
    }
    let (_d, base) = git_repo();
    let dbdir = tempfile::tempdir().unwrap();
    let db = dbdir.path().join("t.db");
    seed_policy_rows(&db, &[("r1", always(Level::RuntimeDefault, Decision::Allow))]);

    let out = run_task(
        &base,
        &db,
        &[
            "--effect",
            "charge:acct_1",
            "--exec",
            "sh",
            "-c",
            "echo COMMAND-RAN",
        ],
        &[],
    );
    assert_success(&out);
    assert!(
        stdout_of(&out).contains("COMMAND-RAN"),
        "铸得出能力时命令应真的执行\n--- stdout ---\n{}",
        stdout_of(&out)
    );
    assert_eq!(
        effect_rows(&db),
        vec![(
            "charge".to_owned(),
            "acct_1".to_owned(),
            "committed".to_owned()
        )],
        "退出码 0 ⇒ 该效应记为 COMMITTED"
    );
    ran(TEST);
}

/// `RequireApproval` 那条臂，**两侧都钉**：不传 `--approve` 即拒绝运行；传了就放行。
///
/// 只钉一侧不算数：只测「不传即拒」时，把 `mints` 的第二个入参写成常量 `false` 的实现
/// 全绿；只测「传了即放行」时，写成常量 `true` 的实现全绿（那会让 `Deny` 也放行，
/// 由 [`an_effect_without_a_capability_refuses_to_run_the_command`] 挡住）。
#[test]
fn a_require_approval_effect_needs_the_flag_to_run() {
    const TEST: &str = "a_require_approval_effect_needs_the_flag_to_run";
    if !require_auto_selected_sandbox(TEST) {
        return;
    }
    let (_d, base) = git_repo();
    let dbdir = tempfile::tempdir().unwrap();
    let db = dbdir.path().join("t.db");
    seed_policy_rows(
        &db,
        &[("r1", always(Level::UserPersistent, Decision::RequireApproval))],
    );

    // 未给 --approve：要求批准而没批准 ⇒ 铸不出 ⇒ 命令不跑。
    let without = run_task(
        &base,
        &db,
        &[
            "--effect",
            "charge:acct_1",
            "--exec",
            "sh",
            "-c",
            "echo COMMAND-RAN",
        ],
        &[],
    );
    assert!(
        !without.status.success(),
        "要求批准而未给 --approve 时必须拒绝运行\n--- stdout ---\n{}",
        stdout_of(&without)
    );
    assert!(
        stderr_of(&without).contains("要求批准"),
        "错误应点名裁决是「要求批准」，实际 stderr：{}",
        stderr_of(&without)
    );
    assert!(
        !stdout_of(&without).contains("COMMAND-RAN"),
        "命令不得执行"
    );

    // 给了 --approve：第 2 级的内建 Allow 越过第 3 级的 RequireApproval ⇒ 铸得出。
    let with = run_task(
        &base,
        &db,
        &[
            "--approve",
            "--effect",
            "charge:acct_1",
            "--exec",
            "sh",
            "-c",
            "echo COMMAND-RAN",
        ],
        &[],
    );
    assert_success(&with);
    assert!(
        stdout_of(&with).contains("COMMAND-RAN"),
        "给了 --approve 之后命令应真的执行\n--- stdout ---\n{}",
        stdout_of(&with)
    );
    ran(TEST);
}

/// **逐条**效应都查：一条铸不出即拒绝**整条**命令——不是只拒绝那一条、其余的照跑。
///
/// 库里只放一条**按 `effect_type` 限定**的 `Allow`（只放行 `charge`），命令声明
/// `charge:acct_1` 与 `deploy:prod` 两条。只查第一条（或只查出一条能铸就放行）的实现
/// 会让命令跑起来，本条即红。
///
/// 对照臂放在末尾：只声明那条能铸出的 `charge` 时命令照常跑，证明「被拒」来自 `deploy`
/// 那一条，而不是夹具或策略表整体有问题。
#[test]
fn an_effect_that_cannot_be_minted_refuses_the_whole_command() {
    const TEST: &str = "an_effect_that_cannot_be_minted_refuses_the_whole_command";
    if !require_auto_selected_sandbox(TEST) {
        return;
    }
    let (_d, base) = git_repo();
    let dbdir = tempfile::tempdir().unwrap();
    let db = dbdir.path().join("t.db");
    seed_policy_rows(&db, &[("r1", allow_effect(EffectType::Charge))]);

    let refused = run_task(
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
            "echo COMMAND-RAN",
        ],
        &[],
    );
    assert!(
        !refused.status.success(),
        "有一条效应铸不出能力时必须拒绝运行整条命令\n--- stdout ---\n{}",
        stdout_of(&refused)
    );
    let stderr = stderr_of(&refused);
    assert!(
        stderr.contains("deploy:prod"),
        "错误应点名是 deploy:prod 那一条铸不出，实际 stderr：{stderr}"
    );
    assert!(
        !stdout_of(&refused).contains("COMMAND-RAN"),
        "整条命令一步都不该跑\n--- stdout ---\n{}",
        stdout_of(&refused)
    );
    assert!(
        effect_rows(&db).is_empty(),
        "整条被拒 ⇒ 一条记录都不该留下（含能铸出的那条），实际 {:?}",
        effect_rows(&db)
    );
    assert_eq!(workspace_row_count(&db), 0, "被拒之后工作区记录应被清理");

    // 对照臂：只声明能铸出的那条 charge，命令照常跑。
    let accepted = run_task(
        &base,
        &db,
        &["--effect", "charge:acct_1", "--exec", "sh", "-c", "echo COMMAND-RAN"],
        &[],
    );
    assert_success(&accepted);
    assert!(
        stdout_of(&accepted).contains("COMMAND-RAN"),
        "对照臂里命令应真的执行\n--- stdout ---\n{}",
        stdout_of(&accepted)
    );
    ran(TEST);
}

/// 强制点就位之后，**效应行仍在命令跑起来之前写好**（P2 的「写入早于执行」在插进这道
/// 校验之后重新钉一遍）。
///
/// 见模块文档「为什么没有『能力已登记』这条判据」：能力不落库，可观察的只剩这个 P2 的
/// 既有事实。命令在**自己的第一条指令**上读 Journal 的落库字节，读到 `executing` 才印
/// `EFFECT-EXECUTING`；命令若抢在写入之前跑，这个标记不会出现。
///
/// **本用例显式点名 bubblewrap**：Landlock 的只读白名单是 `/usr /lib /lib64 /etc /bin
/// /sbin`，库所在的临时目录不在其中，命令在 Landlock 下连库文件都打不开；bubblewrap 的
/// `--ro-bind / /` 才让库可读。无 bwrap 的机器上显式跳过（带执行/跳过条数）。
#[test]
fn the_effects_are_registered_before_the_command_runs_under_the_gate() {
    const TEST: &str = "the_effects_are_registered_before_the_command_runs_under_the_gate";
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
    seed_policy_rows(&db, &[("r1", always(Level::RuntimeDefault, Decision::Allow))]);

    let out = run_task(
        &base,
        &db,
        &[
            "--sandbox",
            "bubblewrap",
            "--effect",
            "charge:acct_1",
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

/// 多条效应**都**铸不出时报的是**按声明次序第一条**——把次序对调，报的就换成另一条。
///
/// 这不是「整条拒绝」的又一张照片（那是
/// [`an_effect_that_cannot_be_minted_refuses_the_whole_command`]），而是**报哪一条**：
/// 库里一条规则都没有 ⇒ 两条都是默认 `Deny` ⇒ 哪一条都铸不出。若实现报的是最后一条、
/// 或按别的次序挑一条，对调前后必有一条对不上。两侧都断言（报了这一条、且没报另一条）。
///
/// **不受沙箱门控**：校验排在 `Sandbox::spawn` 之前，本用例两次调用都在那之前返回，
/// 故在既无 Landlock 也无 bwrap 的机器上也真的跑得起来（本文件其余用例不行）。
#[test]
fn the_first_unmintable_effect_in_declaration_order_is_reported() {
    const TEST: &str = "the_first_unmintable_effect_in_declaration_order_is_reported";
    let (_d, base) = git_repo();
    let dbdir = tempfile::tempdir().unwrap();
    let db = dbdir.path().join("t.db");

    let first = run_task(
        &base,
        &db,
        &[
            "--effect",
            "charge:a1",
            "--effect",
            "deploy:d1",
            "--exec",
            "true",
        ],
        &[],
    );
    assert!(
        !first.status.success(),
        "两条效应都铸不出时必须拒绝运行\n--- stdout ---\n{}",
        stdout_of(&first)
    );
    let stderr = stderr_of(&first);
    assert!(
        stderr.contains("charge:a1"),
        "应报声明次序第一条 charge:a1，实际 stderr：{stderr}"
    );
    assert!(
        !stderr.contains("deploy:d1"),
        "不该报后面那一条，实际 stderr：{stderr}"
    );

    // 对调声明次序：报的随之换成新的第一条。
    let second = run_task(
        &base,
        &db,
        &[
            "--effect",
            "deploy:d1",
            "--effect",
            "charge:a1",
            "--exec",
            "true",
        ],
        &[],
    );
    assert!(
        !second.status.success(),
        "两条效应都铸不出时必须拒绝运行\n--- stdout ---\n{}",
        stdout_of(&second)
    );
    let stderr = stderr_of(&second);
    assert!(
        stderr.contains("deploy:d1"),
        "对调次序后应报新的第一条 deploy:d1，实际 stderr：{stderr}"
    );
    assert!(
        !stderr.contains("charge:a1"),
        "不该报后面那一条，实际 stderr：{stderr}"
    );
    ran(TEST);
}
