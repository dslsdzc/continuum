//! 环境变量凭据源（设计 §5.3 的真实现之二）。
//!
//! 全部用例只用合成值：**不在仓库中写入任何凭据**。
//!
//! 进程环境是共享状态，故所有 `set_var` 都在 [`ENV_LOCK`] 之下——`std::env::set_var`
//! 在 edition 2024 是 `unsafe`，其条件正是「不得与其他线程的 getenv/setenv 并发」。

use std::sync::Mutex;

use continuum_capability::{Capability, CapabilityKind, GithubAction, mint};
use continuum_secrets::{EnvCredentialSource, SecretsError, SecretsRuntime, env_var_name};

const FAR: i64 = i64::MAX / 2;
const PREFIX: &str = "CONTINUUM_TEST_SECRET_";

/// 串行化本文件内所有接触进程环境的用例。
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn lock() -> std::sync::MutexGuard<'static, ()> {
    // 前一个用例 panic 过会让锁中毒，但环境本身没有不变量被破坏，继续即可。
    ENV_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// 设置环境变量。调用方必须已持有 [`ENV_LOCK`]。
fn set_var(name: &str, value: &str) {
    // SAFETY: 本文件内所有 set_var/getenv 都在 ENV_LOCK 之下，无并发。
    unsafe { std::env::set_var(name, value) };
}

fn capability(scope: &str, expiry: i64) -> Capability {
    mint(
        CapabilityKind::Github(GithubAction::CreatePr),
        scope.to_owned(),
        expiry,
    )
    .expect("测试能力应铸得出")
}

#[test]
fn the_variable_name_is_the_prefix_plus_the_escaped_scope() {
    // 非字母数字（含字面 `_`）逐字节写成 `_` + 两位大写十六进制；字母转大写。
    assert_eq!(
        env_var_name("CONTINUUM_SECRET_", "repo/X"),
        "CONTINUUM_SECRET_REPO_2FX"
    );
    assert_eq!(env_var_name("P_", "a.b-c/d"), "P_A_2EB_2DC_2FD");
    assert_eq!(env_var_name("P_", "repo_a"), "P_REPO_5FA");

    // 编码是单射：四个只差一个标点的作用域给出四个互不相同的变量名——它们不会
    // 共用一份材料（文件源同样视它们为四个不同的作用域）。
    let names = [
        env_var_name("P_", "repo/a-b"),
        env_var_name("P_", "repo/a_b"),
        env_var_name("P_", "repo/a.b"),
        env_var_name("P_", "repo/a/b"),
    ];
    let mut unique = names.to_vec();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), 4, "编码塌缩了：{names:?}");

    // 限制（拍下来，不只是写在注释里）：编码是**单射 modulo ASCII 大小写**——
    // 字母统一转大写，故只差大小写的两个作用域共用一个变量名、共用一份材料。
    // 这是本 crate 选的大写惯例带来的（Linux 上环境变量名本身区分大小写，
    // Windows 上不区分），不是媒介换不回来的东西；凡以大小写区分作用域的场景用文件源。
    assert_eq!(
        env_var_name("P_", "repo/X"),
        env_var_name("P_", "REPO/X")
    );
}

#[test]
fn a_set_variable_is_issued_and_read() {
    let _guard = lock();
    let scope = "repo/X";
    let name = env_var_name(PREFIX, scope);
    set_var(&name, "ENV-SECRET-1a2b");

    let rt = SecretsRuntime::new(Box::new(EnvCredentialSource::new(PREFIX, FAR)));
    let cap = capability(scope, FAR);
    let credential = rt.issue(&cap, 0).expect("应能签发");

    assert_eq!(credential.scope(), scope);
    assert_eq!(
        rt.material(&credential, 0).expect("应取得到").expose(),
        b"ENV-SECRET-1a2b"
    );
}

#[test]
fn a_longer_ttl_is_truncated_to_the_capabilitys_expiry() {
    let _guard = lock();
    let scope = "repo/长ttl";
    set_var(&env_var_name(PREFIX, scope), "ENV-SECRET-2c3d");

    let rt = SecretsRuntime::new(Box::new(EnvCredentialSource::new(PREFIX, 1_000_000)));
    let credential = rt
        .issue(&capability(scope, 5_000), 0)
        .expect("应能签发");

    assert_eq!(credential.expiry(), 5_000);
}

#[test]
fn a_shorter_ttl_is_kept_and_not_widened_to_the_capabilitys() {
    let _guard = lock();
    let scope = "repo/短ttl";
    set_var(&env_var_name(PREFIX, scope), "ENV-SECRET-4e5f");

    let rt = SecretsRuntime::new(Box::new(EnvCredentialSource::new(PREFIX, 3_000)));
    let credential = rt.issue(&capability(scope, FAR), 0).expect("应能签发");

    assert_eq!(credential.expiry(), 3_000);
}

#[test]
fn an_overflowing_ttl_saturates_instead_of_wrapping() {
    let _guard = lock();
    let scope = "repo/溢出";
    set_var(&env_var_name(PREFIX, scope), "ENV-SECRET-9z8y");

    let rt = SecretsRuntime::new(Box::new(EnvCredentialSource::new(PREFIX, i64::MAX)));
    // `now + ttl` 会溢出：饱和加法钉在 `i64::MAX`，回绕则会得到一个负数（过去的
    // 时刻），凭据一签出即失效——本条即钉住这一条注释里的说法。
    let credential = rt
        .issue(&capability(scope, i64::MAX), 1)
        .expect("应能签发");

    assert_eq!(credential.expiry(), i64::MAX);
}

#[test]
fn a_scope_with_no_variable_is_rejected() {
    let _guard = lock();
    let scope = "repo/从未设置过";

    let rt = SecretsRuntime::new(Box::new(EnvCredentialSource::new(PREFIX, FAR)));
    let cap = capability(scope, FAR);

    match rt.issue(&cap, 0) {
        Err(SecretsError::ScopeNotCovered { scope: reported, .. }) => {
            assert_eq!(reported, scope);
        }
        other => panic!("变量不存在时应拒签，实得 {other:?}"),
    }
}

#[test]
fn an_empty_variable_value_is_rejected() {
    let _guard = lock();
    let scope = "repo/空值";
    set_var(&env_var_name(PREFIX, scope), "");

    let rt = SecretsRuntime::new(Box::new(EnvCredentialSource::new(PREFIX, FAR)));
    let cap = capability(scope, FAR);

    match rt.issue(&cap, 0) {
        Err(SecretsError::EmptyMaterial { scope: reported, .. }) => {
            assert_eq!(reported, scope);
        }
        other => panic!("变量为空时应拒签，实得 {other:?}"),
    }
}
