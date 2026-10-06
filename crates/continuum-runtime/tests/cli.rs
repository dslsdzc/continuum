//! 驱动的子命令与参数解析（设计下篇第 4.1 节）。
//!
//! 这些用例经 `continuum_runtime::cli` 直接看**解析结果**——各字段解出的是什么、错误是
//! 哪一个变体、信息里点名了哪个 token。驱动二进制只能看到退出码与 stderr 的文本，
//! 拿不到解析结果本身。
//!
//! `task` 的行为判据（建区、沙箱执行、效应声明、清理）在 `task_cli.rs` 里经二进制观察；
//! 本文件只放解析这一层。解析期的拒绝（如 `--backend` 是未知选项、`--effect` 目标为空）
//! 同样经二进制可见，那几条落在 `task_cli.rs`。

use continuum_core::tool::ToolId;
use continuum_effect::EffectType;
use continuum_runtime::cli::{
    self, CliError, Command, EffectSpec, RecoverArgs, SandboxMechanism,
};
use continuum_workspace::IntentId;
use std::path::PathBuf;

/// 一条最小合法 `task` 命令的**前置部分**，即 `--exec` 之前的那些 token。
fn task_prefix() -> Vec<&'static str> {
    vec!["task", "--base", "/b", "--intent", "i1", "--db", "/db"]
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
fn task_requires_base_intent_db_and_exec() {
    // 四个必填项各缺一次。断言的是**具体哪一个**错误变体，不只是「返回了 Err」：
    // 只报「参数有误」的解析器同样能让「返回了 Err」通过。
    //
    // 四项逐项过，不只抽一两个：四个缺项检查是**各自手写的分支**（`parse_task` 末尾的
    // 四行），能各自漂移——`--db` 那一行正是本 task 新加的。
    let cases: [(Vec<&str>, &'static str); 4] = [
        (
            vec!["task", "--intent", "i1", "--db", "/db", "--exec", "true"],
            "--base",
        ),
        (
            vec!["task", "--base", "/b", "--db", "/db", "--exec", "true"],
            "--intent",
        ),
        (
            vec!["task", "--base", "/b", "--intent", "i1", "--db", "/db"],
            "--exec",
        ),
        (
            vec!["task", "--base", "/b", "--intent", "i1", "--exec", "true"],
            "--db",
        ),
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
    let err = cli::parse(["task", "--base", "/b", "--intent", "i1", "--db", "/db", "--exec"])
        .unwrap_err();
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
    assert_eq!(a.db, PathBuf::from("/db"));
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
        "--db",
        "/db",
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
        "task", "--base", "/b", "--intent", "i1", "--db", "/db", "--apply", "--exec", "true",
    ]);
    assert!(a.apply);
    assert_eq!(a.exec, vec!["true".to_owned()]);
}

#[test]
fn a_duplicate_value_option_is_rejected() {
    // 静默取后一个会让 `--base /a --base /b` 看起来像「指定了 /a」，而实际跑 /b。
    //
    // **五个带取值的分支逐项过**，不只抽两个：模块文档「# 重复选项」那节把两处 `--db`
    // 也算在内，那是一项**五点枚举上的绝对断言**。只钉住其中几处时，没钉住的那些分支
    // 可以被改成任何东西而没有对照片——这正是纪律 2 说的「绝对措辞藏在『A 或 B』的
    // 完备性里」。两处 `--db` 分属两个子命令、各自手写，故各要一张照片。
    let cases: [(Vec<&str>, &'static str); 5] = [
        (
            vec![
                "task", "--base", "/a", "--base", "/b", "--intent", "i1", "--db", "/db",
                "--exec", "true",
            ],
            "--base",
        ),
        (
            vec![
                "task", "--base", "/b", "--intent", "i1", "--intent", "i2", "--db", "/db",
                "--exec", "true",
            ],
            "--intent",
        ),
        (
            vec![
                "task", "--base", "/b", "--intent", "i1", "--db", "/a", "--db", "/b", "--exec",
                "true",
            ],
            "--db",
        ),
        (
            vec![
                "task", "--base", "/b", "--intent", "i1", "--db", "/db", "--sandbox", "landlock",
                "--sandbox", "bubblewrap", "--exec", "true",
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
        "--db",
        "/db",
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
        "--db",
        "/db",
        "--approve",
        "--approve",
        "--exec",
        "true",
    ]);
    assert!(a.approve, "`--approve` 重复给出应照常接受，且仍为真");

    // 「`--effect` 本就可重复」由 `an_effect_names_a_known_type` 末段覆盖
    // （两条 `--effect` 都留下，且类型与目标各按出现次序对上）。
}

/// 带取值的选项在**参数末尾**缺取值时，各自报各自的 `MissingValue`。
///
/// 与 `a_duplicate_value_option_is_rejected` 是同一形态：六处都是各自手写的分支
/// （`parse_task` 的五个臂 + `parse_recover` 的一个臂），都能各自漂移，故逐项钉住，
/// 不只抽一个。`--exec` 那一处由 `task_requires_base_intent_db_and_exec` 覆盖——它缺的
/// 是整条命令，走的是同一个变体。
#[test]
fn a_value_option_at_the_end_of_argv_is_rejected() {
    let cases: [(Vec<&str>, &'static str); 6] = [
        (vec!["task", "--base"], "--base"),
        (vec!["task", "--base", "/b", "--intent"], "--intent"),
        (
            vec!["task", "--base", "/b", "--intent", "i1", "--db"],
            "--db",
        ),
        (
            vec!["task", "--base", "/b", "--intent", "i1", "--db", "/db", "--sandbox"],
            "--sandbox",
        ),
        (
            vec![
                "task", "--base", "/b", "--intent", "i1", "--db", "/db", "--effect",
            ],
            "--effect",
        ),
        (vec!["recover", "--db"], "--db"),
    ];
    for (args, option) in cases {
        let err = cli::parse(args).unwrap_err();
        assert_eq!(err, CliError::MissingValue { option });
        assert!(
            err.to_string().contains(option),
            "错误信息应点名 {option}，实际：{err}"
        );
    }
}

/// `USAGE` 是打到 stderr 给用户看的**纯文本**，不是 rustdoc，故不得带 markdown 标记。
///
/// 它曾写作 `目标按**第一个**冒号切开。`——用户看到的是带星号的那串字符。本用例挡住
/// 这一类回归：星号强调与反引号在终端里都是原样显示的，不会变成任何样式。
#[test]
fn usage_is_plain_text_without_markdown_markers() {
    assert!(
        !cli::USAGE.contains("**"),
        "USAGE 不得含 markdown 的星号强调：\n{}",
        cli::USAGE
    );
    assert!(
        !cli::USAGE.contains('`'),
        "USAGE 不得含 markdown 的反引号：\n{}",
        cli::USAGE
    );
}

/// `CliError` 的文档写着「每个变体都点名**具体是哪一个**选项/取值出了错」——
/// 这也是一项枚举上的绝对断言，故这里把**全部十二个变体**过一遍。
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
        (
            CliError::InvalidToolInput {
                value: "{".to_owned(),
                reason: "EOF while parsing an object".to_owned(),
            },
            "{",
        ),
        (
            CliError::OptionRequiresEffect { option: "--approve" },
            "--approve",
        ),
    ];
    for (err, token) in named {
        assert!(
            err.to_string().contains(token),
            "{err:?} 的信息应点名 {token}，实际：{err}"
        );
    }

    let msg = CliError::MissingSubcommand.to_string();
    for available in ["task", "tool", "recover"] {
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
        cli::parse(["recover", "--db", "/x", "--base", "/b"]).unwrap_err(),
        CliError::UnknownOption {
            name: "--base".to_owned()
        }
    );
}

// ── `tool` 子命令的解析面（F 设计 §2.1、§2.2） ───────────────────────────────
//
// 与 `task` 同法：本文件只放解析这一层，路径行为在 `tool_call.rs`（库级）里验。
// 四条 `UnknownOption` 各一条、不抽代表——它们是四条独立分支（`--base` / `--exec` /
// `--apply` / `--sandbox` 走的是同一个兜底臂，但四条选项名各自能漂移）。

/// `tool` 子命令的最小合法前置部分：两个必填项。
fn tool_prefix() -> Vec<&'static str> {
    vec!["tool", "--db", "/db", "--tool", "t1"]
}

/// 在最小合法 `tool` 命令上追加若干 token，返回整条 argv。
fn tool_argv(extra: &[&str]) -> Vec<String> {
    let mut args: Vec<String> = tool_prefix().iter().map(|s| (*s).to_owned()).collect();
    args.extend(extra.iter().map(|s| (*s).to_owned()));
    args
}

/// 解析并断言落在 `Tool` 变体上。
fn tool_args<I, S>(args: I) -> cli::ToolArgs
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    match cli::parse(args).expect("应解析成功") {
        Command::Tool(a) => a,
        other => panic!("应解析为 Tool，实际 {other:?}"),
    }
}

/// 同上，但用最小合法命令加追加项。
fn tool_with(extra: &[&str]) -> cli::ToolArgs {
    tool_args(tool_argv(extra))
}

/// 各字段与命令行逐项相同。**工具 id 就是 `continuum_core::tool::ToolId`**（不另建
/// 第二个）——本用例正是按那个类型断言，故「是不是同一个类型」这件事在此有照片。
#[test]
fn tool_parses_its_own_options() {
    let a = tool_args([
        "tool",
        "--db",
        "d",
        "--tool",
        "t1",
        "--input",
        r#"{"a":1}"#,
        "--effect",
        "charge:x",
        "--intent",
        "i1",
    ]);

    assert_eq!(a.db, PathBuf::from("d"));
    assert_eq!(a.tool, ToolId::new("t1"));
    assert_eq!(a.input, serde_json::json!({"a": 1}));
    assert_eq!(a.intent, Some(IntentId::new("i1")));
    assert!(!a.approve, "未给出 --approve 时为假");
    assert_eq!(
        a.effects,
        vec![EffectSpec {
            effect_type: EffectType::Charge,
            target: "x".to_owned(),
        }]
    );

    // `--approve` 是开关，与 `--effect` 同进同出时照常解析。
    let a = tool_with(&["--effect", "charge:x", "--intent", "i1", "--approve"]);
    assert!(a.approve);
}

/// 省略 `--input` → `json!({})`，**不是 `null`**。
///
/// 两者都是合法 JSON，故「返回了某个值」这条断言分不开它们——必须断言到具体的值。
#[test]
fn tool_input_defaults_to_an_empty_object() {
    let a = tool_with(&[]);
    assert_eq!(
        a.input,
        serde_json::json!({}),
        "省略 --input 时输入是空对象（「没有参数」），不是 null（「没有输入」）"
    );
    assert!(!a.input.is_null(), "空对象与 null 是两回事");
}

/// `--input` 不是合法 JSON → **解析期**即 `Err`，断言到变体。
#[test]
fn tool_rejects_an_unparsable_input() {
    let err = cli::parse(tool_argv(&["--input", "{"])).unwrap_err();
    match err {
        CliError::InvalidToolInput { value, reason } => {
            assert_eq!(value, "{", "变体里应带上那段不可解析的原文");
            assert!(!reason.is_empty(), "reason 取 serde_json 的消息，不该为空");
        }
        other => panic!("应报 InvalidToolInput，实际 {other:?}"),
    }

    // 合法的 JSON 不受影响——否则上面那条对「一律拒绝 --input」的实现照样成立。
    let a = tool_with(&["--input", "[1, 2]"]);
    assert_eq!(a.input, serde_json::json!([1, 2]));
}

/// 给出 `--effect` 而**未**给出 `--intent` → `Err`（效应没有幂等键的来源）。
#[test]
fn an_effect_without_an_intent_is_rejected() {
    let err = cli::parse(tool_argv(&["--effect", "charge:x"])).unwrap_err();
    assert_eq!(err, CliError::OptionRequiresEffect { option: "--intent" });
}

/// 反向：给出 `--intent` 或 `--approve` 而零 `--effect` → 各自 `Err`。
///
/// 两条断言各只给一个选项，故它们**钉不住次序**（两个选项不会同时出现）；次序由
/// [`both_options_without_an_effect_report_the_intent`] 单独钉。
#[test]
fn an_intent_or_an_approval_without_an_effect_is_rejected() {
    let err = cli::parse(tool_argv(&["--intent", "i1"])).unwrap_err();
    assert_eq!(err, CliError::OptionRequiresEffect { option: "--intent" });

    let err = cli::parse(tool_argv(&["--approve"])).unwrap_err();
    assert_eq!(err, CliError::OptionRequiresEffect { option: "--approve" });
}

/// **次序的那条照片**：零效应下两个选项**同时**给出时，报 `--intent`。
///
/// 前一条的两条断言各只给一个选项，换成「先判 `--approve`」的实现它们照样全过——
/// 故次序承诺没有那一条就没有照片。
#[test]
fn both_options_without_an_effect_report_the_intent() {
    let err = cli::parse(tool_argv(&["--intent", "i1", "--approve"])).unwrap_err();
    assert_eq!(
        err,
        CliError::OptionRequiresEffect { option: "--intent" },
        "两个选项同时给出时报选项表次序靠前的 --intent"
    );
}

/// 零效应的**正常侧**：一个必填项齐全、零 `--effect` 的命令照常解析。
#[test]
fn a_tool_call_without_any_effect_parses() {
    let a = tool_with(&[]);
    assert!(a.effects.is_empty());
    assert_eq!(a.intent, None);
    assert!(!a.approve);
}

/// `--base` / `--exec` / `--apply` / `--sandbox` 都不被 `tool` 接受（设计 §2.1）。
///
/// 四条各一条、不抽代表：它们落到**同一个兜底臂**上，但四条选项名是四个各自可漂移的
/// 取值，而本用例断言的正是在信息里点名了哪一个。
#[test]
fn tool_rejects_the_task_options() {
    let cases: [(Vec<String>, &'static str); 4] = [
        (tool_argv(&["--base", "/b"]), "--base"),
        (tool_argv(&["--exec", "true"]), "--exec"),
        (tool_argv(&["--apply"]), "--apply"),
        (tool_argv(&["--sandbox", "landlock"]), "--sandbox"),
    ];
    for (args, name) in cases {
        let err = cli::parse(args).unwrap_err();
        assert_eq!(err, CliError::UnknownOption { name: name.to_owned() });
        assert!(
            err.to_string().contains(name),
            "错误信息应点名 {name}，实际：{err}"
        );
    }
}

/// 两个必填项缺任一 → `MissingOption`，**两条各一**（与 `task` 的四个缺项同法）。
#[test]
fn tool_requires_db_and_tool() {
    let cases: [(Vec<&str>, &'static str); 2] = [
        (vec!["tool", "--tool", "t1"], "--db"),
        (vec!["tool", "--db", "/db"], "--tool"),
    ];
    for (args, missing) in cases {
        let err = cli::parse(args).unwrap_err();
        assert_eq!(err, CliError::MissingOption { option: missing });
        assert!(
            err.to_string().contains(missing),
            "错误信息应点名 {missing}，实际：{err}"
        );
    }
}

/// `tool` 的四个带取值的选项各自只接受一次，第二次出现即 `Err`。
///
/// 与 `task` 的同一取向（模块文档「# 重复选项」）：静默取后者会让 `--tool a --tool b`
/// 看起来像「指定了 a」而实际跑 b。**四条逐项过**，它们是四个各自手写的分支。
#[test]
fn a_duplicate_tool_option_is_rejected() {
    let cases: [(Vec<&str>, &'static str); 4] = [
        (
            vec!["tool", "--db", "/a", "--db", "/b", "--tool", "t1"],
            "--db",
        ),
        (
            vec!["tool", "--db", "/db", "--tool", "a", "--tool", "b"],
            "--tool",
        ),
        (
            vec!["tool", "--db", "/db", "--tool", "t1", "--input", "{}", "--input", "{}"],
            "--input",
        ),
        (
            vec![
                "tool", "--db", "/db", "--tool", "t1", "--intent", "i1", "--intent", "i2",
                "--effect", "charge:x",
            ],
            "--intent",
        ),
    ];
    for (args, option) in cases {
        let err = cli::parse(args).unwrap_err();
        assert_eq!(err, CliError::DuplicateOption { option });
        assert!(
            err.to_string().contains(option),
            "错误信息应点名 {option}，实际：{err}"
        );
    }

    // 开关型选项重复是幂等的（与 `task` 同一条口径）。
    let a = tool_with(&["--effect", "charge:x", "--intent", "i1", "--approve", "--approve"]);
    assert!(a.approve);
}
