//! Runtime 入口：解析子命令并分派（设计下篇第 4.1 节）。
//!
//! 迁移集合为 P0 内置迁移加 P1、P2、P3 各层迁移；恢复钩子由各层注册。

use continuum_persist::Migration;
use continuum_runtime::cli::{self, Command};
use std::ffi::OsString;
use std::process::ExitCode;

mod recover_cmd;
mod recovery;
mod secrets;
mod task_cmd;
mod tool_cmd;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // 原始参数另取一份（`OsString`，不经 UTF-8 转换）：`task` 第 2 步要把自身经
    // `unshare -Urm` 重新执行，转交的必须是**调用方实际写的那串参数**，从解析结果
    // 重建会丢掉形状（同一个值可以有多种写法）。
    let raw: Vec<OsString> = std::env::args_os().skip(1).collect();
    // 参数解析在装配之前：命令行写错了要先说命令行的事，不去碰凭据源的 IO。
    let command = match cli::parse(args) {
        Ok(command) => command,
        Err(e) => {
            eprintln!("参数错误：{e}");
            eprintln!("{}", cli::USAGE);
            return ExitCode::FAILURE;
        }
    };

    // 密钥运行时的装配（设计 §4.1：驱动装配好传进来）。放在**分派之前**：
    // 凭据源配错时任何子命令都不启动，失败点落在启动、而不是落到某条路径的深处。
    //
    // 这个绑定今天**没有消费者**（唯一消费者是 B 的连接器入口，那条边按裁决 §六.3
    // 不接）——它不是「暂时没用到的管道」，是 §11 第 13b 条那条未接线的现场。
    // 详见 `secrets` 模块的文档；不要把它读成「已经接上了」。
    let _secrets_runtime = match secrets::assemble() {
        Ok(runtime) => runtime,
        Err(e) => {
            eprintln!("启动失败: {e}");
            return ExitCode::FAILURE;
        }
    };

    match command {
        Command::Recover(a) => match recover_cmd::run(&a.db) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("启动失败: {e}");
                ExitCode::FAILURE
            }
        },
        Command::Task(a) => match task_cmd::run(&a, &raw) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("任务失败: {e}");
                ExitCode::FAILURE
            }
        },
        // 与 `Task` 臂同形。**这一臂与 `cli::Command::Tool` 必须同批**：少了它就是
        // `error[E0004]`（非穷尽），而 `cargo test --test cli` 单独能过（它不构 bin），
        // 故自检里那条 `cargo build --workspace --all-targets` 才是这一臂的守卫。
        Command::Tool(a) => match tool_cmd::run(&a) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("工具调用失败: {e}");
                ExitCode::FAILURE
            }
        },
    }
}

/// 本驱动全量注册的迁移集合：P0 内建 + P1（artifact、graph）+ P2（workspace、policy、
/// effect）+ P3（capability 的 `tool` 表；子项目 D 的 `model_registry` / `model_profile` /
/// `model_skill_score` 三张表）。
///
/// `recover` 与 `task` 共用同一份，两处各写一份清单会让「注册的集合」有两个来源：
/// `task` 要 `workspace` 表（设计第 4.2 节第 3 步落库），若它那份少一条，症状要到
/// 落库时以「表不存在」的形式出现，而不是在装配处。
///
/// **每张表由「用它的那个 task」注册**（计划 Task 10/11，P3 同）：`policy` 由
/// Task 10 加上（第 7 步的 `load_policies` 要读它），`effect` 由 Task 11 加上
/// （第 4 步的 `record_planned` 要写它），`tool` 由 P3 的 Task 4 加上（Task 5 的
/// `authorize` 要读它）。把它们一次排在装配收尾会让前面的 task 各要一张还没建的表
/// ——**表要在用它之前建**，这条在端到端测试与真实调用上是同一件事（用户跑
/// `task --effect …` 同样会撞上「no such table」），不只是测试夹具的问题。
///
/// **本子项目（P3 子项目 D）的三张表由 D 自己注册**（「谁的表谁注册」，协调者裁决）：
/// `model_registry` / `model_profile` / `model_skill_score` 由 §3.1 的一条迁移建出，
/// 注册在下面这一行里。**它的消费方（子项目 G 的模型调用路径）本轮尚不存在**——
/// 这一句是给后来者的：这张清单里那三张表没有本轮的使用者，不是漏接了，是使用者还没建。
pub(crate) fn runtime_migrations() -> Vec<Migration> {
    let mut migrations = continuum_persist::builtin_migrations();
    migrations.extend(continuum_artifact::p1_artifact_migrations());
    migrations.extend(continuum_graph::p1_graph_migrations());
    migrations.extend(continuum_workspace::p2_workspace_migrations());
    migrations.extend(continuum_policy::p2_policy_migrations());
    migrations.extend(continuum_effect::p2_effect_migrations());
    migrations.extend(continuum_capability::p3_capability_migrations());
    migrations.extend(continuum_model_registry::p3d_model_migrations());
    migrations
}

// `recover` 的实现已移进 [`recover_cmd`]（与 `task_cmd` 对称）：本模块只做分发与
// 装配清单。恢复钩子的注册（P1 的 `MarkRunningNodesLost` 与本子项目的
// `MarkExecutingAsUnknown`）在那边的 [`recover_cmd::run`] 里。
