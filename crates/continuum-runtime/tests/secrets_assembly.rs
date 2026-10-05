//! Task 7：驱动侧对 `SecretsRuntime` 的装配（设计 §4.1「驱动装配好传进来」）。
//!
//! 判据的对象是**真实二进制的启动行为**：装配写在 `main.rs` 里，import 本 crate 的函数
//! 看不到它。故两条用例都经 [`env!("CARGO_BIN_EXE_continuum-runtime")`] 驱动真实二进制。
//!
//! **环境变量是进程级的**：两条用例各自 `Command::new` 起独立进程，**不用**
//! `std::env::set_var` 改本进程环境（Rust 2024 里它已是 `unsafe`，且会污染同 crate
//! 别的二进制用例——那些用例与本文件并行跑在同一个测试进程里）。
//!
//! 两条用例互为一对：只留前一条，「驱动本来就起不来」也会让它绿；只留后一条，
//! 「配了坏凭据源却静默照跑」也会让它绿。

use std::path::Path;
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_continuum-runtime");

/// 凭据源位置的环境变量。**设计未规定这个位置从哪来，是本实现自定的**
/// （见 `crates/continuum-runtime/src/secrets.rs` 的模块文档）。
const CREDENTIALS_ENV: &str = "CONTINUUM_CREDENTIALS";

/// 跑一次 `recover`，`env` 是给子进程加的环境（`None` 表示显式移除该变量）。
fn recover_with(db: &Path, credentials: Option<&str>) -> std::process::Output {
    let mut cmd = Command::new(BIN);
    match credentials {
        Some(path) => {
            cmd.env(CREDENTIALS_ENV, path);
        }
        // 显式移除而不是「不管」：父测试进程的环境里若恰好有这个变量，
        // 「不设该变量」这条对照臂就会跑成另一件事。
        None => {
            cmd.env_remove(CREDENTIALS_ENV);
        }
    }
    cmd.args(["recover", "--db"])
        .arg(db)
        .output()
        .expect("continuum-runtime 无法执行")
}

/// 配了读不出来的凭据源，启动必须**失败**（fail-closed），且报的是文件源读失败
/// 那一种 `Err`（`SecretsError::SourceIo`），不是泛泛的「失败了」。
///
/// 判据：退出码非零 + stderr 含 `启动失败: 凭据源 … 读取失败: …`（`SourceIo` 的
/// `Display`，见 `continuum-secrets/src/error.rs`），且含那个不存在的路径。
#[test]
fn a_broken_credential_source_fails_the_startup() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("t.db");
    let missing = dir.path().join("不存在的凭据文件");
    assert!(!missing.exists(), "夹具前提：这个路径本就不存在");

    let out = recover_with(&db, Some(&missing.display().to_string()));

    assert!(
        !out.status.success(),
        "配了读不出来的凭据源，启动却成功了（退出码 {:?}）\n--- stdout ---\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("启动失败"),
        "stderr 缺少启动失败的标记:\n{stderr}"
    );
    assert!(
        stderr.contains("凭据源") && stderr.contains("读取失败"),
        "stderr 不是 `SecretsError::SourceIo` 的消息（期望含「凭据源 … 读取失败」）:\n{stderr}"
    );
    assert!(
        stderr.contains(&missing.display().to_string()),
        "消息里没带上那个读不出来的路径:\n{stderr}"
    );
}

/// 对照臂：**不**设置那个变量时，启动照常成功。
///
/// 没有这一条，上一条在「驱动本来就起不来」（例如版本不匹配、迁移失败）时也会绿
/// ——两条一起才把「是那处装配把它拦下的」钉住。
#[test]
fn an_absent_configuration_leaves_the_startup_working() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("t.db");

    let out = recover_with(&db, None);

    assert!(
        out.status.success(),
        "没配凭据源时启动本应照常成功，实际退出码 {:?}\n--- stderr ---\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
}
