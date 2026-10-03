//! 驱动的子命令与参数解析（设计下篇第 4.1 节）。
//!
//! 用法：
//!
//! ```text
//! continuum task    --base <目录> --intent <id> --exec <命令> [参数...]
//!                   [--apply] [--approve]
//!                   [--sandbox <机制>]
//!                   [--effect <类型>:<目标>]...
//! continuum recover --db <路径>
//! ```
//!
//! 手写解析，不引第三方 CLI 库：本 crate 的参数表很小，且现有依赖里没有这类库。
//! 口径照 `continuum-policy` 的 `Condition::parse`——**任何一项不符即 `Err`**，
//! 不错解、不取默认值。理由与那里相同：一个被静默忽略的选项会让驱动按它未被告知
//! 的行为跑，而调用方以为自己的意图已被采纳；本 crate 是外部意图进入系统的第一个
//! 关口，静默的入口无法在下游补救。
//!
//! # `--exec` 的约定
//!
//! **`--exec` 消费其后的全部参数，拼成命令的 argv。** 故 `--exec` 之后的任何 token
//! 都是命令的一部分，**即使它看起来像本驱动自己的选项**（如 `--apply`）——想做
//! `--apply`，把它写在 `--exec` **之前**。
//!
//! 理由有三条：
//! - [`continuum_sandbox::Sandbox::spawn`] 收的是 [`std::process::Command`]，要的就是 argv；
//! - 自己按空白切分、或把整串交给 shell，都会把**带空格的参数**悄悄弄坏：
//!   `--exec git commit -m "两个 空格"` 里的引号与空格在两种做法下都还原不回来；
//! - 「`--exec` 之后全是命令」是 `env` / `timeout` / `nice` 的既有约定，不必另立新规。
//!
//! `--exec` 之后**至少**要有一个参数（命令本身），缺则 `Err`。
//!
//! # 重复选项
//!
//! 带取值的选项（`--base` / `--intent` / `--sandbox` / `--db`）**只接受一次**，
//! 第二次出现即 `Err`。
//!
//! **为什么不取「后者胜」**：`--base /a --base /b` 在后者胜下会按 `/b` 跑，而写的人
//! 若本意是 `/a`（例如把两条命令拼在一起、或复制粘贴时忘了删），他看到的是一次
//! **成功**的调用，事后没有任何痕迹指向 `/a` 被丢掉了。这与 `save_policy` 那条
//! 「`OR REPLACE` 会静默覆盖用户策略」是同一种毛病——**写错的东西要拒绝，不要静默
//! 退化**（设计下篇第 5.1 节对条件解析立的就是这条）。拒绝的代价是重打一次命令。
//!
//! 开关型选项（`--apply` / `--approve`）重复给出是幂等的——重复不会改变结果，
//! 故照常接受；`--effect` 按设计第 4.1 节的 `...` 本就可重复。

use continuum_effect::EffectType;
use continuum_workspace::IntentId;
use std::path::PathBuf;
use thiserror::Error;

/// 本驱动的用法串。解析失败时由 `main` 打印。
///
/// 首行的程序名取设计下篇第 4.1 节的写法（`continuum`），不是本 crate 的 bin
/// 名（`continuum-runtime`）——名字以设计为准，是否改名/加别名不在本 task 内。
pub const USAGE: &str = "\
用法：
  continuum task    --base <目录> --intent <id> --exec <命令> [参数...]
                    [--apply] [--approve]
                    [--sandbox <机制>]
                    [--effect <类型>:<目标>]...
  continuum recover --db <路径>

说明：
  --exec 消费其后的全部参数作为命令的 argv（不切分、不经 shell），
         故 --exec 之后的任何 token 都属于该命令，即使它看起来像本驱动的选项。
         想在命令里用 --apply，请把它写在 --exec 之前。
  --sandbox 取 landlock 或 bubblewrap；不给则由装配点按能力自动选。
  --effect 形如 <类型>:<目标>，类型取 EffectType 的封闭枚举，目标按**第一个**冒号切开。";

/// 一次调用的子命令（设计下篇第 4.1 节）。
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Task(TaskArgs),
    Recover(RecoverArgs),
}

/// `task` 子命令的参数（设计下篇第 4.1、4.2 节）。
///
/// 本类型只承载**解析结果**，不做任何探测与构造：沙箱句柄、工作区、数据库
/// 都不在这里取（见 [`SandboxMechanism`] 的说明）。
#[derive(Debug, Clone, PartialEq)]
pub struct TaskArgs {
    /// Base Workspace 的根目录。
    pub base: PathBuf,
    /// Intent 的身份。此处只承载字符串，路径逃逸的判定在
    /// `continuum_workspace` 的创建路径上（那是唯一能判的地方）。
    pub intent: IntentId,
    /// 命令的 argv，**已按调用方给的边界切好**（见模块文档的 `--exec` 约定）。
    /// 长度至少为 1，第 0 项即命令本身。
    pub exec: Vec<String>,
    /// 是否请求集成（设计第 4.2 节第 7 步）。
    pub apply: bool,
    /// 是否给出第 2 级批准（设计第 5.5 节）。
    pub approve: bool,
    /// 显式指定的沙箱机制；`None` 表示命令行未指定。
    pub sandbox: Option<SandboxMechanism>,
    /// 本条命令计划施加的效应，按出现次序。可为零条。
    pub effects: Vec<EffectSpec>,
}

/// `recover` 子命令的参数（设计下篇第 4.1 节）。
#[derive(Debug, Clone, PartialEq)]
pub struct RecoverArgs {
    /// 数据库路径。
    pub db: PathBuf,
}

/// 一条 `--effect <类型>:<目标>`（设计第 6.1 节的 `effect_type` 与 `target`）。
#[derive(Debug, Clone, PartialEq)]
pub struct EffectSpec {
    pub effect_type: EffectType,
    /// 作用对象（URL / 分支名 / 收件人 / 资源标识），文本。
    pub target: String,
}

impl EffectSpec {
    /// 从 `<类型>:<目标>` 解析。
    ///
    /// 按**第一个**冒号切开，故目标里可以有冒号（`publish:https://example.com`
    /// 的目标是 `https://example.com`）。类型侧不可能含冒号，故第一个冒号即分界。
    ///
    /// 类型经 [`EffectType::parse`] 收窄到封闭枚举，未知类型**在解析期**即 `Err`
    /// ——设计下篇第 6.8 节的理由是「开放类型会让策略表漏判」，故未知类型不该
    /// 到运行期才失败。
    ///
    /// **目标为空（`publish:`）同样是 `Err`，不当作「空目标」收下**：效应记录的作用
    /// 对象是策略与对账都要读的字段，一条 target 为空的记录落进 `effect` 表之后，
    /// 没有任何下游能判断它指的是什么——幂等键算得出、状态机推得动，唯独无法回答
    /// 「这是对哪个东西做的」。这类记录既不能被执行，也不能被安全地忽略，故在
    /// 入口处拒掉（同一条「写错的要拒绝，不要静默退化」）。
    pub fn parse(value: &str) -> Result<Self, CliError> {
        let Some((type_name, target)) = value.split_once(':') else {
            return Err(CliError::EffectWithoutColon {
                value: value.to_owned(),
            });
        };
        if target.is_empty() {
            return Err(CliError::EffectWithEmptyTarget {
                value: value.to_owned(),
            });
        }
        let effect_type = EffectType::parse(type_name).ok_or_else(|| CliError::UnknownEffectType {
            name: type_name.to_owned(),
        })?;
        Ok(Self {
            effect_type,
            target: target.to_owned(),
        })
    }
}

/// 沙箱机制的**选择**，只有名字，没有句柄。
///
/// 本类型刻意不持有 [`continuum_sandbox::Sandbox`]：那个枚举的各变体带着句柄，
/// **构造期**就要做探测（Landlock 的内核 ABI、bubblewrap 的 `bwrap` 定位），
/// 而解析期只该表达「调用方点了哪一个」。构造发生在装配点（Task 9/10），
/// 那时才按设计第 4.3 节决定是显式指定还是按能力自动选。
///
/// 故 `None`（未指定）与 `Some(..)`（指定了）是两种状态，本类型不表达「自动选」
/// 的结果——那是装配点的事。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxMechanism {
    Landlock,
    Bubblewrap,
}

impl SandboxMechanism {
    /// 命令行取值。与 [`SandboxMechanism::parse`] 互逆。
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Landlock => "landlock",
            Self::Bubblewrap => "bubblewrap",
        }
    }

    /// [`SandboxMechanism::as_str`] 的严格逆；表外字符串一律 `None`，不取默认机制
    /// ——取默认会让「我指定了 bubblewrap」与「我没指定」变成同一件事。
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "landlock" => Self::Landlock,
            "bubblewrap" => Self::Bubblewrap,
            _ => return None,
        })
    }
}

/// 解析失败的原因。每个变体都点名**具体是哪一个**选项/取值出了错：
/// 解析错误是给人看的，只报「参数有误」会让人逐个试。
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CliError {
    /// 一个参数都没有。
    #[error("缺少子命令，可用：task、recover")]
    MissingSubcommand,
    /// 首参数不是 `task` 也不是 `recover`。
    #[error("未知子命令：{name}，可用：task、recover")]
    UnknownSubcommand { name: String },
    /// 选项后面没有取值。
    #[error("选项 {option} 缺少取值")]
    MissingValue { option: &'static str },
    /// 必填选项没给。
    #[error("缺少必填选项：{option}")]
    MissingOption { option: &'static str },
    /// 本子命令不认识的选项。
    #[error("未知选项：{name}")]
    UnknownOption { name: String },
    /// 带取值的选项出现了多次。
    #[error("选项 {option} 出现了多次；本选项只接受一次")]
    DuplicateOption { option: &'static str },
    /// `--sandbox` 的取值不在封闭集合内。
    #[error("未知沙箱机制：{name}，可用：landlock、bubblewrap")]
    UnknownSandboxMechanism { name: String },
    /// `--effect` 的值里没有冒号。
    #[error("效应缺少冒号：{value}，应为 <类型>:<目标>")]
    EffectWithoutColon { value: String },
    /// `--effect` 的目标部分为空。
    #[error("效应目标为空：{value}，应为 <类型>:<目标>")]
    EffectWithEmptyTarget { value: String },
    /// `--effect` 的类型不在 [`EffectType`] 的封闭枚举内。
    #[error("未知效应类型：{name}")]
    UnknownEffectType { name: String },
}

/// 解析一整条命令行（**不含** argv[0]）。
///
/// 收 `IntoIterator<Item = Into<String>>` 而非 `&[String]`：调用方给 `env::args()`
/// 的余项与给字面量数组都能直接用，不必为其中一种多写一次转换。
///
/// 只做解析，不做任何 I/O 与探测——所有取值原样装进结果，判定留给各自的归属层
/// （路径逃逸给 `continuum_workspace`，沙箱可用性给装配点）。
pub fn parse<I, S>(args: I) -> Result<Command, CliError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut args = args.into_iter().map(Into::into);
    let Some(subcommand) = args.next() else {
        return Err(CliError::MissingSubcommand);
    };
    let rest: Vec<String> = args.collect();

    match subcommand.as_str() {
        "task" => parse_task(&rest).map(Command::Task),
        "recover" => parse_recover(&rest).map(Command::Recover),
        other => Err(CliError::UnknownSubcommand {
            name: other.to_owned(),
        }),
    }
}

/// 取 `args[*i]` 之后的那一个 token 作取值，并把 `*i` 推进到取值之后。
fn take_value(args: &[String], i: &mut usize, option: &'static str) -> Result<String, CliError> {
    let value = args
        .get(*i + 1)
        .ok_or(CliError::MissingValue { option })?
        .clone();
    *i += 2;
    Ok(value)
}

fn parse_task(args: &[String]) -> Result<TaskArgs, CliError> {
    let mut base: Option<PathBuf> = None;
    let mut intent: Option<IntentId> = None;
    let mut exec: Option<Vec<String>> = None;
    let mut apply = false;
    let mut approve = false;
    let mut sandbox: Option<SandboxMechanism> = None;
    let mut effects = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--base" => {
                if base.is_some() {
                    return Err(CliError::DuplicateOption { option: "--base" });
                }
                base = Some(PathBuf::from(take_value(args, &mut i, "--base")?));
            }
            "--intent" => {
                if intent.is_some() {
                    return Err(CliError::DuplicateOption { option: "--intent" });
                }
                intent = Some(IntentId::new(take_value(args, &mut i, "--intent")?));
            }
            "--apply" => {
                apply = true;
                i += 1;
            }
            "--approve" => {
                approve = true;
                i += 1;
            }
            "--sandbox" => {
                if sandbox.is_some() {
                    return Err(CliError::DuplicateOption { option: "--sandbox" });
                }
                let name = take_value(args, &mut i, "--sandbox")?;
                sandbox = Some(
                    SandboxMechanism::parse(&name)
                        .ok_or(CliError::UnknownSandboxMechanism { name })?,
                );
            }
            "--effect" => {
                let value = take_value(args, &mut i, "--effect")?;
                effects.push(EffectSpec::parse(&value)?);
            }
            "--exec" => {
                // 消费其后**全部**参数（模块文档的 `--exec` 约定）。此处不求值、
                // 不切分：argv 的边界由调用方用参数边界给出，不由本函数猜。
                let argv = &args[i + 1..];
                if argv.is_empty() {
                    // 缺的不是「取值」而是整条命令，但报错仍须点名是 --exec 少了东西。
                    return Err(CliError::MissingValue { option: "--exec" });
                }
                exec = Some(argv.to_vec());
                i = args.len();
            }
            other => {
                return Err(CliError::UnknownOption {
                    name: other.to_owned(),
                });
            }
        }
    }

    Ok(TaskArgs {
        base: base.ok_or(CliError::MissingOption { option: "--base" })?,
        intent: intent.ok_or(CliError::MissingOption { option: "--intent" })?,
        exec: exec.ok_or(CliError::MissingOption { option: "--exec" })?,
        apply,
        approve,
        sandbox,
        effects,
    })
}

fn parse_recover(args: &[String]) -> Result<RecoverArgs, CliError> {
    let mut db: Option<PathBuf> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--db" => {
                if db.is_some() {
                    return Err(CliError::DuplicateOption { option: "--db" });
                }
                db = Some(PathBuf::from(take_value(args, &mut i, "--db")?));
            }
            other => {
                return Err(CliError::UnknownOption {
                    name: other.to_owned(),
                });
            }
        }
    }

    Ok(RecoverArgs {
        db: db.ok_or(CliError::MissingOption { option: "--db" })?,
    })
}
