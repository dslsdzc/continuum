//! 凭据签发的运行期判据（设计 §5.1、§5.2、§8）。
//!
//! 「窄能力要不到宽凭据」是**类型层**命题（签名里没有指定另一个作用域的位置），
//! 由 `tests/type_level.rs` 的两份编译失败样例钉住；本文件的用例只钉运行期能观察
//! 的那一半：签出的凭据作用域**等于**所据能力的作用域（不是调用方给的），到期不晚
//! 于能力，且材料的取用只在访问点发生。

use continuum_capability::{Capability, CapabilityError, CapabilityKind, GithubAction, mint};
use continuum_secrets::{
    CredentialSource, RotationEvent, SecretMaterial, SecretsError, SecretsRuntime,
};

/// 测试用的合成材料。**不是任何真实凭据**（计划 §「不在仓库中写入任何凭据」）。
const SECRET: &str = "SUPER-SECRET-VALUE-9f3c";

/// 一个足够远的到期时刻：`i64::MAX` 会让 `min` 的比较失去意义（任何被比较的一方都
/// 等于它），故取一半。
const FAR: i64 = i64::MAX / 2;

/// 测试用凭据源：`expiry` 为 `None` 表示本源不覆盖任何作用域。
struct FakeSource {
    name: &'static str,
    expiry: Option<i64>,
    material: &'static str,
}

impl FakeSource {
    fn covering(name: &'static str, expiry: i64, material: &'static str) -> Self {
        Self {
            name,
            expiry: Some(expiry),
            material,
        }
    }

    fn covering_nothing(name: &'static str) -> Self {
        Self {
            name,
            expiry: None,
            material: SECRET,
        }
    }
}

impl CredentialSource for FakeSource {
    fn name(&self) -> &str {
        self.name
    }

    fn claimed_expiry(&self, scope: &str, _now: i64) -> Result<i64, SecretsError> {
        self.expiry.ok_or_else(|| SecretsError::ScopeNotCovered {
            origin: self.name.to_owned(),
            scope: scope.to_owned(),
        })
    }

    fn fetch(&self, scope: &str, _now: i64) -> Result<SecretMaterial, SecretsError> {
        match self.expiry {
            Some(_) => Ok(SecretMaterial::new(self.material)),
            None => Err(SecretsError::ScopeNotCovered {
                origin: self.name.to_owned(),
                scope: scope.to_owned(),
            }),
        }
    }
}

fn capability(scope: &str, expiry: i64) -> Capability {
    mint(
        CapabilityKind::Github(GithubAction::CreatePr),
        scope.to_owned(),
        expiry,
    )
    .expect("测试能力应铸得出")
}

fn runtime(source: FakeSource) -> SecretsRuntime {
    SecretsRuntime::new(Box::new(source))
}

#[test]
fn a_credential_carries_the_capabilitys_scope_and_expiry() {
    let rt = runtime(FakeSource::covering("测试源", FAR, SECRET));
    let cap = capability("repo/X", FAR);

    let credential = rt.issue(&cap, 0).expect("应能签发");

    // 作用域来自**能力**，不是调用方——`issue` 的签名里没有可以另指一个作用域的位置
    // （那一半由 `tests/compile_fail/issue_has_no_scope_parameter.rs` 钉）。
    assert_eq!(credential.scope(), "repo/X");
    assert_eq!(credential.expiry(), cap.expiry());
}

#[test]
fn a_longer_source_validity_is_truncated_to_the_capabilitys_expiry() {
    let rt = runtime(FakeSource::covering("测试源", 9_000, SECRET));
    let cap = capability("repo/X", 5_000);

    let credential = rt.issue(&cap, 0).expect("应能签发");

    // 源声称更长 → 截到能力的到期时刻（若实现只取源的到期时刻，本条变红）。
    assert_eq!(credential.expiry(), 5_000);
}

#[test]
fn a_shorter_source_validity_is_kept_and_not_widened_to_the_capabilitys() {
    let rt = runtime(FakeSource::covering("测试源", 3_000, SECRET));
    let cap = capability("repo/X", 5_000);

    let credential = rt.issue(&cap, 0).expect("应能签发");

    // 源声称更短 → 取源的那一个（若实现只取能力的到期时刻，本条变红）。
    assert_eq!(credential.expiry(), 3_000);
}

#[test]
fn an_expired_capability_cannot_issue() {
    let rt = runtime(FakeSource::covering("测试源", FAR, SECRET));
    let cap = capability("repo/X", 1_000);

    // 到期时刻本身即失效（`expiry` 是失效时刻，不是「最后有效的时刻」）。
    // 比较仍只有 `Capability::is_valid_at` 一个产生点，本 crate 转出它的错误。
    match rt.issue(&cap, 1_000) {
        Err(SecretsError::Capability(CapabilityError::Expired {
            expiry: 1_000,
            now: 1_000,
        })) => {}
        other => panic!("到期后应拒签且报能力已失效，实得 {other:?}"),
    }

    // 对照臂：到期前一毫秒签得出。
    assert!(rt.issue(&cap, 999).is_ok());
}

#[test]
fn a_scope_the_source_does_not_cover_is_rejected() {
    let rt = runtime(FakeSource::covering_nothing("测试源"));
    let cap = capability("repo/X", FAR);

    match rt.issue(&cap, 0) {
        Err(SecretsError::ScopeNotCovered { origin, scope }) => {
            assert_eq!(origin, "测试源");
            assert_eq!(scope, "repo/X");
        }
        other => panic!("源不覆盖该作用域时应拒签，实得 {other:?}"),
    }
}

#[test]
fn the_material_is_readable_before_the_credential_expires_and_not_at_it() {
    let rt = runtime(FakeSource::covering("测试源", 1_000, SECRET));
    let cap = capability("repo/X", FAR);
    let credential = rt.issue(&cap, 0).expect("应能签发");
    assert_eq!(credential.expiry(), 1_000);

    // 对照臂：到期前一毫秒取得到材料。
    assert_eq!(
        rt.material(&credential, 999).expect("应取得到").expose(),
        SECRET.as_bytes()
    );

    for now in [1_000, 1_001] {
        match rt.material(&credential, now) {
            Err(SecretsError::CredentialExpired {
                expiry: 1_000,
                now: reported,
            }) => assert_eq!(reported, now),
            other => panic!("now={now} 时凭据应已失效，实得 {other:?}"),
        }
    }
}

#[test]
fn every_rotation_event_class_invalidates_old_credentials() {
    for event in RotationEvent::ALL {
        let mut rt = runtime(FakeSource::covering("测试源", FAR, SECRET));
        let cap = capability("repo/X", FAR);
        let before = rt.issue(&cap, 0).expect("应能签发");
        assert!(rt.material(&before, 0).is_ok(), "{event:?}: 轮换前应可用");

        rt.rotate(event);

        assert_eq!(rt.last_rotation(), Some(event), "{event:?} 未记为最近一次轮换");
        match rt.material(&before, 0) {
            Err(SecretsError::Superseded { .. }) => {}
            other => panic!("{event:?} 轮换后旧凭据应被取代，实得 {other:?}"),
        }

        // 重发：新签的凭据可用。
        let after = rt.issue(&cap, 0).expect("应能重发");
        assert_eq!(
            rt.material(&after, 0).expect("重发后应可用").expose(),
            SECRET.as_bytes(),
            "{event:?}: 重发的凭据应可用"
        );
    }
}

/// 每次 `fetch` 都换一份材料的源：用来观察「材料是在访问点取的，不是签在凭据上的」。
struct CountingSource {
    calls: std::cell::Cell<u32>,
}

impl CredentialSource for CountingSource {
    fn name(&self) -> &str {
        "计数源"
    }

    fn claimed_expiry(&self, _scope: &str, _now: i64) -> Result<i64, SecretsError> {
        Ok(FAR)
    }

    fn fetch(&self, _scope: &str, _now: i64) -> Result<SecretMaterial, SecretsError> {
        let n = self.calls.get();
        self.calls.set(n + 1);
        Ok(SecretMaterial::new(format!("第{n}次取用")))
    }
}

#[test]
fn the_material_is_fetched_at_every_access_and_not_kept_on_the_credential() {
    let rt = SecretsRuntime::new(Box::new(CountingSource {
        calls: std::cell::Cell::new(0),
    }));
    let cap = capability("repo/X", FAR);
    let credential = rt.issue(&cap, 0).expect("应能签发");

    // 同一枚凭据两次取用拿到不同的材料 → 材料在访问点从源取，凭据上只有句柄。
    assert_eq!(
        rt.material(&credential, 0).expect("应取得到").expose(),
        "第0次取用".as_bytes()
    );
    assert_eq!(
        rt.material(&credential, 0).expect("应取得到").expose(),
        "第1次取用".as_bytes()
    );
}

#[test]
fn the_four_rotation_event_classes_are_listed_by_name() {
    // §103 列的四类触发事件，逐项按名字钉住。`RotationEvent::as_str` 是无通配臂的
    // 穷尽 match，故新增变体会编译不过——但「新增变体后忘了加进 ALL」编译器抓不到，
    // 本条只能钉住 ALL 现有的项。
    assert_eq!(RotationEvent::ALL.len(), 4);
    let names: Vec<&str> = RotationEvent::ALL.iter().map(|e| e.as_str()).collect();
    assert_eq!(
        names,
        vec![
            "authority_compromised",
            "permanent_failover",
            "device_revoked",
            "credential_leaked",
        ]
    );
}

#[test]
fn no_error_message_embeds_the_material() {
    let cap = capability("repo/X", FAR);

    // 被取代的凭据：材料此刻仍在运行时手里，错误消息不许把它带出去。
    let mut superseding = runtime(FakeSource::covering("测试源", FAR, SECRET));
    let credential = superseding.issue(&cap, 0).expect("应能签发");
    superseding.rotate(RotationEvent::CredentialLeaked);
    let superseded = superseding.material(&credential, 0).unwrap_err();

    // 已到期的凭据（另起一个运行时，否则轮换那条判定先命中）：同理。
    let expired_rt = runtime(FakeSource::covering("测试源", 1_000, SECRET));
    let credential = expired_rt.issue(&cap, 0).expect("应能签发");
    let expired = expired_rt.material(&credential, 1_000).unwrap_err();

    for err in [superseded, expired] {
        let rendered = err.to_string();
        let debugged = format!("{err:?}");
        assert!(
            !rendered.contains(SECRET) && !debugged.contains(SECRET),
            "错误消息里出现了材料: {rendered} / {debugged}"
        );
    }
}

#[test]
fn the_material_is_redacted_in_debug_output() {
    let material = SecretMaterial::new(SECRET);
    let rendered = format!("{material:?}");
    assert!(
        !rendered.contains(SECRET),
        "SecretMaterial 的 Debug 打出了材料: {rendered}"
    );
}
