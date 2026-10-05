//! 文件凭据源（设计 §5.3 的真实现之一）。
//!
//! 全部用例只用 `tempfile` 与合成值：**不在仓库中写入任何凭据**。

use continuum_capability::{Capability, CapabilityKind, GithubAction, mint};
use continuum_secrets::{
    CredentialSource, FileCredentialSource, SecretsError, SecretsRuntime,
};

const SECRET_A: &str = "FILE-SECRET-A-0a1b";
const SECRET_B: &str = "FILE-SECRET-B-2c3d";
const FAR: i64 = i64::MAX / 2;

fn capability(scope: &str, expiry: i64) -> Capability {
    mint(
        CapabilityKind::Github(GithubAction::CreatePr),
        scope.to_owned(),
        expiry,
    )
    .expect("测试能力应铸得出")
}

/// 写一个临时凭据文件并打开它。`TempDir` 必须随返回值一起活着。
fn source_with(contents: &str) -> (tempfile::TempDir, FileCredentialSource) {
    let dir = tempfile::tempdir().expect("临时目录应建得出");
    let path = dir.path().join("secrets.tsv");
    std::fs::write(&path, contents).expect("临时凭据文件应写得成");
    let source = FileCredentialSource::open(&path).expect("应能打开");
    (dir, source)
}

fn runtime(source: FileCredentialSource) -> SecretsRuntime {
    SecretsRuntime::new(Box::new(source))
}

#[test]
fn a_listed_scope_is_issued() {
    let (_dir, source) = source_with(&format!("# 注释行\nrepo/X\t{SECRET_A}\t{FAR}\n"));
    let rt = runtime(source);
    let cap = capability("repo/X", FAR);

    let credential = rt.issue(&cap, 0).expect("应能签发");

    assert_eq!(credential.scope(), "repo/X");
    assert_eq!(
        rt.material(&credential, 0).expect("应取得到").expose(),
        SECRET_A.as_bytes()
    );
}

#[test]
fn a_scope_the_file_does_not_list_is_rejected() {
    let (_dir, source) = source_with(&format!("repo/X\t{SECRET_A}\t{FAR}\n"));
    let rt = runtime(source);
    let cap = capability("repo/Z", FAR);

    match rt.issue(&cap, 0) {
        Err(SecretsError::ScopeNotCovered { scope, .. }) => assert_eq!(scope, "repo/Z"),
        other => panic!("文件未列该作用域时应拒签，实得 {other:?}"),
    }
}

#[test]
fn a_files_longer_validity_is_truncated_to_the_capabilitys_expiry() {
    let (_dir, source) = source_with(&format!("repo/X\t{SECRET_A}\t9000\n"));
    let rt = runtime(source);

    let credential = rt
        .issue(&capability("repo/X", 5_000), 0)
        .expect("应能签发");

    assert_eq!(credential.expiry(), 5_000);
}

#[test]
fn a_files_shorter_validity_is_kept_and_not_widened_to_the_capabilitys() {
    let (_dir, source) = source_with(&format!("repo/X\t{SECRET_A}\t3000\n"));
    let rt = runtime(source);

    let credential = rt
        .issue(&capability("repo/X", 5_000), 0)
        .expect("应能签发");

    assert_eq!(credential.expiry(), 3_000);
}

#[test]
fn reloading_the_file_swaps_both_material_and_claimed_expiry() {
    let dir = tempfile::tempdir().expect("临时目录应建得出");
    let path = dir.path().join("secrets.tsv");
    std::fs::write(&path, format!("repo/X\t{SECRET_A}\t9000\n")).expect("应写得成");
    let mut source = FileCredentialSource::open(&path).expect("应能打开");

    assert_eq!(
        source.fetch("repo/X", 0).expect("应取得到").expose(),
        SECRET_A.as_bytes()
    );
    assert_eq!(source.claimed_expiry("repo/X", 0).expect("应给得出"), 9_000);

    // 运维重写了文件后重载：材料与声称的到期时刻都换新的。
    std::fs::write(&path, format!("repo/X\t{SECRET_B}\t3000\n")).expect("应写得成");
    source.reload().expect("应能重载");

    assert_eq!(
        source.fetch("repo/X", 0).expect("应取得到").expose(),
        SECRET_B.as_bytes()
    );
    assert_eq!(source.claimed_expiry("repo/X", 0).expect("应给得出"), 3_000);
}

#[test]
fn a_malformed_entry_is_rejected_without_echoing_the_line() {
    // 第三字段不是整数；这一行里的材料不许出现在错误消息中。
    let dir = tempfile::tempdir().expect("临时目录应建得出");
    let path = dir.path().join("secrets.tsv");
    std::fs::write(&path, "repo/X\tLEAKME-9f3c\t不是数字\n").expect("应写得成");

    match FileCredentialSource::open(&path) {
        Err(SecretsError::SourceFormat { line, .. }) => assert_eq!(line, 1),
        other => panic!("非法条目应在打开时报格式错，实得 {other:?}"),
    }

    let err = FileCredentialSource::open(&path).expect_err("非法文件应开不了");
    assert!(
        !err.to_string().contains("LEAKME-9f3c"),
        "格式错误消息里出现了材料: {err}"
    );
}

#[test]
fn every_rejection_reason_of_the_parser_has_a_photo() {
    // 逐项覆盖 `parse` 的五条拒收分支（字段数、空作用域、空材料、到期时刻、重复），
    // 并断言消息里不含材料——材料用的是合成值。
    let cases = [
        ("repo/X\t甲\t1000\nrepo/X\t乙\t2000\n", 2, "作用域重复"),
        (
            "repo/X\t甲\n",
            1,
            "应为 作用域<TAB>材料<TAB>到期时刻 三个字段",
        ),
        ("\t甲\t1000\n", 1, "作用域为空"),
        ("repo/X\t\t1000\n", 1, "材料为空"),
        ("repo/X\t甲\t不是数字\n", 1, "到期时刻不是 Unix 毫秒整数"),
    ];

    for (contents, expected_line, expected_reason) in cases {
        let dir = tempfile::tempdir().expect("临时目录应建得出");
        let path = dir.path().join("secrets.tsv");
        std::fs::write(&path, contents).expect("应写得成");

        match FileCredentialSource::open(&path) {
            Err(SecretsError::SourceFormat { line, reason, .. }) => {
                assert_eq!(line, expected_line, "行号不符（{contents:?}）");
                assert_eq!(reason, expected_reason, "拒收理由不符（{contents:?}）");
            }
            other => panic!("应报格式错（{contents:?}），实得 {other:?}"),
        }

        let err = FileCredentialSource::open(&path).expect_err("非法文件应开不了");
        assert!(
            !err.to_string().contains('甲') && !err.to_string().contains('乙'),
            "格式错误消息里出现了材料: {err}"
        );
    }
}

#[test]
fn a_failed_reload_keeps_the_previous_entries() {
    let dir = tempfile::tempdir().expect("临时目录应建得出");
    let path = dir.path().join("secrets.tsv");
    std::fs::write(&path, format!("repo/X\t{SECRET_A}\t{FAR}\n")).expect("应写得成");
    let mut source = FileCredentialSource::open(&path).expect("应能打开");

    // 重载到一个非法文件：失败，但旧条目仍在（失败不该让可用的源变空）。
    std::fs::write(&path, "坏文件\n").expect("应写得成");
    match source.reload() {
        Err(SecretsError::SourceFormat { line: 1, .. }) => {}
        other => panic!("非法文件重载应报格式错，实得 {other:?}"),
    }

    assert_eq!(
        source.fetch("repo/X", 0).expect("旧条目应还在").expose(),
        SECRET_A.as_bytes()
    );
}

#[test]
fn a_missing_file_is_reported_as_a_source_io_error() {
    let dir = tempfile::tempdir().expect("临时目录应建得出");
    let path = dir.path().join("不存在.tsv");

    match FileCredentialSource::open(&path) {
        Err(SecretsError::SourceIo { .. }) => {}
        other => panic!("文件不存在时应报 IO 错，实得 {other:?}"),
    }
}

#[test]
fn the_source_debug_output_does_not_carry_the_material() {
    let (_dir, source) = source_with(&format!("repo/X\t{SECRET_A}\t{FAR}\n"));

    let rendered = format!("{source:?}");

    assert!(
        !rendered.contains(SECRET_A),
        "FileCredentialSource 的 Debug 打出了材料: {rendered}"
    );
}
