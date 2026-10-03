//! 驱动的子命令与参数解析（设计下篇第 4.1 节）。
//!
//! 这些用例经 `continuum_runtime::cli` 直接看**解析结果**——错误信息点的是哪一个
//! 选项、效应解出的是哪一型与哪一个目标。经二进制驱动只能看到退出码与 stdout，
//! 而驱动的 `task` 子命令尚未接线（Task 12 接入前只打印占位并返回非零），
//! 故那两条判据在端到端一侧观察不到。

use continuum_effect::EffectType;
use continuum_runtime::cli::{self, CliError, Command, EffectSpec, RecoverArgs, SandboxMechanism};
use continuum_workspace::IntentId;
use std::path::PathBuf;

/// 一条最小合法 `task` 命令的**前置部分**，即 `--exec` 之前的那些 token。
fn task_prefix() -> Vec<&'static str> {
    vec!["task", "--base", "/b", "--intent", "i1"]
}

/// 解析并断言落在 `Task` 变体上。
fn task_args<I, S>(args: I) -> cli::TaskArgs
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    match cli::parse(args).expect("应解析成功") {
        Command::Task(a) => a,
        other => panic!("应解析为 Task，实际 {other:?}"),
    }
}

/// 在最小合法 `task` 命令上追加若干 token（插在 `--exec` **之前**），返回整条 argv。
///
/// 插在 `--exec` 之前是必须的：`--exec` 消费其后全部 token，写在它之后的选项会
/// 成为命令的一部分（见 `cli` 的模块文档），那不是这些用例要考的东西。
fn task_argv(extra: &[&str]) -> Vec<String> {
    let mut args: Vec<String> = task_prefix().iter().map(|s| (*s).to_owned()).collect();
    args.extend(extra.iter().map(|s| (*s).to_owned()));
    args.push("--exec".to_owned());
    args.push("true".to_owned());
    args
}

/// 同上，但断言解析成功。
fn task_with(extra: &[&str]) -> cli::TaskArgs {
    task_args(task_argv(extra))
}

#[test]
fn task_requires_base_intent_and_exec() {
    // 三个必填项各缺一次。断言的是**具体哪一个**错误变体，不只是「返回了 Err」：
    // 只报「参数有误」的解析器同样能让「返回了 Err」通过。
    let cases: [(Vec<&str>, &'static str); 3] = [
        (vec!["task", "--intent", "i1", "--exec", "true"], "--base"),
        (vec!["task", "--base", "/b", "--exec", "true"], "--intent"),
        (vec!["task", "--base", "/b", "--intent", "i1"], "--exec"),
    ];
    for (args, missing) in cases {
        let err = cli::parse(args).expect_err("缺必填项应解析失败");
        assert_eq!(err, CliError::MissingOption { option: missing });
        // 错误信息必须**点名**缺的那个：否则调用方只能逐个试。
        assert!(
            err.to_string().contains(missing),
            "错误信息应点名 {missing}，实际：{err}"
        );
    }

    // `--exec` 给了但后面一个 token 都没有：同样是「--exec 少了东西」，但走的是
    // 「有选项、无取值」那一路——两种情形分开断言，免得一条被换成了另一条。
    let err = cli::parse(["task", "--base", "/b", "--intent", "i1", "--exec"]).unwrap_err();
    assert_eq!(err, CliError::MissingValue { option: "--exec" });

    // `recover` 的 `--db` 同样是必填。
    let err = cli::parse(["recover"]).unwrap_err();
    assert_eq!(err, CliError::MissingOption { option: "--db" });
}

#[test]
fn the_two_subcommands_are_distinguished() {
    let a = task_with(&[]);
    assert_eq!(a.base, PathBuf::from("/b"));
    assert_eq!(a.intent, IntentId::new("i1"));
    assert_eq!(a.exec, vec!["true".to_owned()]);
    // 未给出的开关与可选项落在各自的缺省态，不互相顶替。
    assert!(!a.apply);
    assert!(!a.approve);
    assert_eq!(a.sandbox, None);
    assert!(a.effects.is_empty());

    assert_eq!(
        cli::parse(["recover", "--db", "/tmp/x.db"]).unwrap(),
        Command::Recover(RecoverArgs {
            db: PathBuf::from("/tmp/x.db")
        })
    );
}

#[test]
fn an_unknown_sandbox_mechanism_is_rejected_at_parse_time() {
    let err = cli::parse(task_argv(&["--sandbox", "nope"])).unwrap_err();
    assert_eq!(
        err,
        CliError::UnknownSandboxMechanism {
            name: "nope".to_owned()
        }
    );

    // 两个合法取值都要真能过——否则上面那条断言对「拒掉一切」的解析器照样成立。
    for (name, expected) in [
        ("landlock", SandboxMechanism::Landlock),
        ("bubblewrap", SandboxMechanism::Bubblewrap),
    ] {
        let a = task_with(&["--sandbox", name]);
        assert_eq!(a.sandbox, Some(expected), "{name} 应解析为 {expected:?}");
        // 命令行取值与 `as_str` 互逆，机制名字只有这一份表。
        assert_eq!(expected.as_str(), name);
    }
}

#[test]
fn an_effect_without_a_colon_is_rejected() {
    let err = cli::parse(task_argv(&["--effect", "push_branch"])).unwrap_err();
    assert_eq!(
        err,
        CliError::EffectWithoutColon {
            value: "push_branch".to_owned()
        }
    );
}

#[test]
fn an_effect_names_a_known_type() {
    let a = task_with(&["--effect", "push_branch:origin/main"]);
    assert_eq!(
        a.effects,
        vec![EffectSpec {
            effect_type: EffectType::PushBranch,
            target: "origin/main".to_owned(),
        }]
    );

    // 目标按**第一个**冒号切开，故目标本身可以含冒号。
    let a = task_with(&["--effect", "publish:https://example.com"]);
    assert_eq!(
        a.effects[0],
        EffectSpec {
            effect_type: EffectType::Publish,
            target: "https://example.com".to_owned(),
        },
        "目标里的冒号不该被当作分隔符"
    );

    // 类型取 `EffectType` 的**封闭枚举**，每个变体都要认（遍历 `ALL`，不只抽一个）。
    for t in EffectType::ALL {
        let value = format!("{}:目标", t.as_str());
        let a = task_with(&["--effect", value.as_str()]);
        assert_eq!(a.effects[0].effect_type, t, "{} 应解析为 {t:?}", t.as_str());
        assert_eq!(a.effects[0].target, "目标");
    }

    // 未知类型在**解析期**即 Err（设计第 6.8 节：开放类型会让策略表漏判，
    // 故未知类型不该到运行期才失败）。
    let err = cli::parse(task_argv(&["--effect", "nope:x"])).unwrap_err();
    assert_eq!(
        err,
        CliError::UnknownEffectType {
            name: "nope".to_owned()
        }
    );

    // 多个 `--effect` 按出现次序累积（设计第 4.1 节的 `...`）。
    let a = task_with(&["--effect", "charge:acct_1", "--effect", "deploy:prod"]);
    let types: Vec<EffectType> = a.effects.iter().map(|e| e.effect_type).collect();
    assert_eq!(types, vec![EffectType::Charge, EffectType::Deploy]);
    let targets: Vec<&str> = a.effects.iter().map(|e| e.target.as_str()).collect();
    assert_eq!(targets, vec!["acct_1", "prod"]);
}

#[test]
fn exec_takes_every_following_token_verbatim() {
    // 这条是「不切分」与「切分」的分水岭：按空白切分的实现会把 `两个 空格`
    // 拆成两个 token，并把 `--apply` 当成本驱动的开关。
    let a = task_args([
        "task",
        "--base",
        "/b",
        "--intent",
        "i1",
        "--exec",
        "git",
        "commit",
        "-m",
        "两个 空格",
        "--apply",
    ]);
    assert_eq!(
        a.exec,
        vec!["git", "commit", "-m", "两个 空格", "--apply"],
        "`--exec` 之后全部 token 原样进 argv，含空格的参数不被切分"
    );
    assert!(
        !a.apply,
        "`--exec` 之后的 `--apply` 属于命令，不该被解析成驱动自己的开关"
    );

    // 反面：写在 `--exec` **之前**的 `--apply` 才是驱动开关。
    let a = task_args([
        "task", "--base", "/b", "--intent", "i1", "--apply", "--exec", "true",
    ]);
    assert!(a.apply);
    assert_eq!(a.exec, vec!["true".to_owned()]);
}

#[test]
fn a_duplicate_value_option_is_rejected() {
    // 静默取后一个会让 `--base /a --base /b` 看起来像「指定了 /a」，而实际跑 /b。
    //
    // **四个带取值的选项逐项过**，不只抽两个：模块文档「# 重复选项」那节写的是
    // 「`--base` / `--intent` / `--sandbox` / `--db` 只接受一次」，是一项**四点
    // 枚举上的绝对断言**。只钉住其中两点时，没钉住的那两条分支可以被改成任何东西
    // 而没有对照片——这正是纪律 2 说的「绝对措辞藏在『A 或 B』的完备性里」。
    let cases: [(Vec<&str>, &'static str); 4] = [
        (
            vec![
                "task", "--base", "/a", "--base", "/b", "--intent", "i1", "--exec", "true",
            ],
            "--base",
        ),
        (
            vec![
                "task", "--base", "/b", "--intent", "i1", "--intent", "i2", "--exec", "true",
            ],
            "--intent",
        ),
        (
            vec![
                "task", "--base", "/b", "--intent", "i1", "--sandbox", "landlock", "--sandbox",
                "bubblewrap", "--exec", "true",
            ],
            "--sandbox",
        ),
        (vec!["recover", "--db", "/a", "--db", "/b"], "--db"),
    ];
    for (args, option) in cases {
        let err = cli::parse(args).unwrap_err();
        // 断言到**具体变体与具体选项**（纪律 3），并断言信息点名了它。
        assert_eq!(err, CliError::DuplicateOption { option });
        assert!(
            err.to_string().contains(option),
            "错误信息应点名 {option}，实际：{err}"
        );
    }

    // 开关型选项重复是幂等的（重复不改变结果），故照常接受——两个开关各来一次。
    let a = task_args([
        "task",
        "--base",
        "/b",
        "--intent",
        "i1",
        "--apply",
        "--apply",
        "--exec",
        "true",
    ]);
    assert!(a.apply, "`--apply` 重复给出应照常接受，且仍为真");

    let a = task_args([
        "task",
        "--base",
        "/b",
        "--intent",
        "i1",
        "--approve",
        "--approve",
        "--exec",
        "true",
    ]);
    assert!(a.approve, "`--approve` 重复给出应照常接受，且仍为真");

    // 「`--effect` 本就可重复」由 `an_effect_names_a_known_type` 末段覆盖
    // （两条 `--effect` 都留下，且类型与目标各按出现次序对上）。
}

/// `CliError` 的文档写着「每个变体都点名**具体是哪一个**选项/取值出了错」——
/// 这也是一项枚举上的绝对断言，故这里把**全部十个变体**过一遍。
///
/// 唯一没有「出错对象」可点的是 `MissingSubcommand`（没有子命令，就没有出错的那个
/// 东西），它的信息改为列出**可用的**子命令，同样是为了不让人逐个试；它单独断言。
#[test]
fn every_error_variant_names_the_offending_token() {
    let named = [
        (
            CliError::UnknownSubcommand {
                name: "nope".to_owned(),
            },
            "nope",
        ),
        (
            CliError::MissingValue {
                option: "--sandbox",
            },
            "--sandbox",
        ),
        (
            CliError::MissingOption {
                option: "--intent",
            },
            "--intent",
        ),
        (
            CliError::UnknownOption {
                name: "--zzz".to_owned(),
            },
            "--zzz",
        ),
        (
            CliError::DuplicateOption {
                option: "--sandbox",
            },
            "--sandbox",
        ),
        (
            CliError::UnknownSandboxMechanism {
                name: "nope".to_owned(),
            },
            "nope",
        ),
        (
            CliError::EffectWithoutColon {
                value: "push_branch".to_owned(),
            },
            "push_branch",
        ),
        (
            CliError::EffectWithEmptyTarget {
                value: "publish:".to_owned(),
            },
            "publish:",
        ),
        (
            CliError::UnknownEffectType {
                name: "nope".to_owned(),
            },
            "nope",
        ),
    ];
    for (err, token) in named {
        assert!(
            err.to_string().contains(token),
            "{err:?} 的信息应点名 {token}，实际：{err}"
        );
    }

    let msg = CliError::MissingSubcommand.to_string();
    for available in ["task", "recover"] {
        assert!(
            msg.contains(available),
            "缺少子命令时应列出可用的 {available}，实际：{msg}"
        );
    }
}

#[test]
fn an_effect_with_an_empty_target_is_rejected() {
    let err = cli::parse(task_argv(&["--effect", "publish:"])).unwrap_err();
    assert_eq!(
        err,
        CliError::EffectWithEmptyTarget {
            value: "publish:".to_owned()
        }
    );
}

/// `task` 子命令在 Task 12 接入前是一个**声明的**占位：必须返回非零退出码并明说
/// 尚未接线，不得有任何看起来「跑过了」的输出。
///
/// **Task 12 接入 `task_cmd` 时删掉本用例。** 它在这里正是为了挡住「占位悄悄变成
/// 一个返回 0 的空实现」——那种实现看起来在做事，而没有别的用例会因此变红。
#[test]
fn task_is_a_declared_placeholder_until_it_is_wired() {
    // 全路径：`Command` 这个名字在本文件里已被 `cli::Command` 占用。
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_continuum-runtime"))
        .args(["task", "--base", "/b", "--intent", "i1", "--exec", "true"])
        .output()
        .expect("continuum-runtime 无法执行");
    assert!(
        !out.status.success(),
        "尚未接线的子命令必须返回非零退出码"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("尚未接线"),
        "应明说尚未接线，实际 stderr：{stderr}"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.is_empty(),
        "占位不得有 stdout 输出（那会看起来像跑过了），实际：{stdout}"
    );
}

#[test]
fn an_unknown_subcommand_or_option_is_rejected() {
    assert_eq!(
        cli::parse(Vec::<String>::new()).unwrap_err(),
        CliError::MissingSubcommand
    );
    assert_eq!(
        cli::parse(["nope"]).unwrap_err(),
        CliError::UnknownSubcommand {
            name: "nope".to_owned()
        }
    );
    // 两个子命令各有自己的选项表：`recover` 不认识 `task` 的选项，反之亦然。
    assert_eq!(
        cli::parse(["recover", "--db", "/x", "--apply"]).unwrap_err(),
        CliError::UnknownOption {
            name: "--apply".to_owned()
        }
    );
    assert_eq!(
        cli::parse(["task", "--db", "/x"]).unwrap_err(),
        CliError::UnknownOption {
            name: "--db".to_owned()
        }
    );
}
