//! Runtime 入口：解析子命令并分派（设计下篇第 4.1 节）。
//!
//! 迁移集合为 P0 内置迁移加 P1、P2 各层迁移；恢复钩子由各层注册。

use continuum_persist::Migration;
use continuum_runtime::cli::{self, Command};
use std::ffi::OsString;
use std::process::ExitCode;

mod recover_cmd;
mod recovery;
mod sandbox_select;
mod task_cmd;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // 原始参数另取一份（`OsString`，不经 UTF-8 转换）：`task` 第 2 步要把自身经
    // `unshare -Urm` 重新执行，转交的必须是**调用方实际写的那串参数**，从解析结果
    // 重建会丢掉形状（同一个值可以有多种写法）。
    let raw: Vec<OsString> = std::env::args_os().skip(1).collect();
    match cli::parse(args) {
        Ok(Command::Recover(a)) => match recover_cmd::run(&a.db) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("启动失败: {e}");
                ExitCode::FAILURE
            }
        },
        Ok(Command::Task(a)) => match task_cmd::run(&a, &raw) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("任务失败: {e}");
                ExitCode::FAILURE
            }
        },
        Err(e) => {
            eprintln!("参数错误：{e}");
            eprintln!("{}", cli::USAGE);
            ExitCode::FAILURE
        }
    }
}

/// 本驱动全量注册的迁移集合：P0 内建 + P1（artifact、graph）+ P2（workspace、policy、effect）。
///
/// `recover` 与 `task` 共用同一份，两处各写一份清单会让「注册的集合」有两个来源：
/// `task` 要 `workspace` 表（设计第 4.2 节第 3 步落库），若它那份少一条，症状要到
/// 落库时以「表不存在」的形式出现，而不是在装配处。
///
/// **每张表由「用它的那个 task」注册**（计划 Task 10/11）：`policy` 由 Task 10 加上
/// （第 7 步的 `load_policies` 要读它），`effect` 由 Task 11 加上（第 4 步的
/// `record_planned` 要写它）。把它们一次排在装配收尾的 Task 12，会让 Task 10 与
/// Task 11 各要一张还没建的表——**表要在用它之前建**，这条在端到端测试与真实调用上
/// 是同一件事（用户跑 `task --effect …` 同样会撞上「no such table」），不只是测试夹具
/// 的问题。
pub(crate) fn runtime_migrations() -> Vec<Migration> {
    let mut migrations = continuum_persist::builtin_migrations();
    migrations.extend(continuum_artifact::p1_artifact_migrations());
    migrations.extend(continuum_graph::p1_graph_migrations());
    migrations.extend(continuum_workspace::p2_workspace_migrations());
    migrations.extend(continuum_policy::p2_policy_migrations());
    migrations.extend(continuum_effect::p2_effect_migrations());
    migrations
}

// `recover` 的实现已移进 [`recover_cmd`]（与 `task_cmd` 对称）：本模块只做分发与
// 装配清单。恢复钩子的注册（P1 的 `MarkRunningNodesLost` 与本子项目的
// `MarkExecutingAsUnknown`）在那边的 [`recover_cmd::run`] 里。
