//! `TestValidity` 的形状、聚合与五臂齐（设计 §6.4；计划 Task 3 的 Step 1）。
//!
//! 本文件里标「编译期」的地方**不是运行期红**——判据是「编译不过」或「从编译失败变成
//! 编译通过」，不假称它们会跑红（纪律 1(c)：「编译失败不是变红」）。

use continuum_verify::{
    CheckVerdict, EvidenceId, TestValidity, TestValidityCheck, Validity, VerifyError,
};

/// 按 [`TestValidityCheck::ALL`] 的顺序给五条判定。
fn five_checks(verdicts: [CheckVerdict; 5]) -> Vec<(TestValidityCheck, CheckVerdict)> {
    TestValidityCheck::ALL.into_iter().zip(verdicts).collect()
}

/// 一条五臂齐记录的 `verdict`（形状校验在 `TestValidity::new` 内做，这里只读聚合结果）。
fn verdict_of(verdicts: [CheckVerdict; 5]) -> Validity {
    TestValidity::new(EvidenceId::from_str("ev-1"), five_checks(verdicts))
        .expect("五臂齐的记录应构造成功")
        .verdict
}

/// 任一 `Fail` ⇒ `Invalid`：五条检查**逐臂各一条**照片（不抽代表）。
///
/// **每条臂都可能是唯一发现「测试无效」的那条**，故 Fail 的位置逐臂挪一遍——只拍一条臂
/// 会让「漏掉某臂的 Fail」漂过去。
///
/// 红条件：把聚合改成多数票、或改成「只要有一条 `Pass` 即 `Valid`」（**取反档**）⇒ 红。
#[test]
fn one_fail_makes_the_whole_record_invalid() {
    assert_eq!(
        verdict_of([
            CheckVerdict::Fail,
            CheckVerdict::Pass,
            CheckVerdict::Pass,
            CheckVerdict::Pass,
            CheckVerdict::Pass,
        ]),
        Validity::Invalid,
        "RequirementMapping 一条 Fail、其余 Pass ⇒ Invalid（§6.4：任一 Fail ⇒ Invalid）"
    );
    assert_eq!(
        verdict_of([
            CheckVerdict::Pass,
            CheckVerdict::Fail,
            CheckVerdict::Pass,
            CheckVerdict::Pass,
            CheckVerdict::Pass,
        ]),
        Validity::Invalid,
        "AssertionStrength 一条 Fail、其余 Pass ⇒ Invalid（§6.4：任一 Fail ⇒ Invalid）"
    );
    assert_eq!(
        verdict_of([
            CheckVerdict::Pass,
            CheckVerdict::Pass,
            CheckVerdict::Fail,
            CheckVerdict::Pass,
            CheckVerdict::Pass,
        ]),
        Validity::Invalid,
        "ErrorImplWouldPass 一条 Fail、其余 Pass ⇒ Invalid（§6.4：任一 Fail ⇒ Invalid）"
    );
    assert_eq!(
        verdict_of([
            CheckVerdict::Pass,
            CheckVerdict::Pass,
            CheckVerdict::Pass,
            CheckVerdict::Fail,
            CheckVerdict::Pass,
        ]),
        Validity::Invalid,
        "MockBypassesRealPath 一条 Fail、其余 Pass ⇒ Invalid（§6.4：任一 Fail ⇒ Invalid）"
    );
    assert_eq!(
        verdict_of([
            CheckVerdict::Pass,
            CheckVerdict::Pass,
            CheckVerdict::Pass,
            CheckVerdict::Pass,
            CheckVerdict::Fail,
        ]),
        Validity::Invalid,
        "MissingBoundary 一条 Fail、其余 Pass ⇒ Invalid（§6.4：任一 Fail ⇒ Invalid）"
    );
}

/// 五条全 `Pass` ⇒ `Valid`。
///
/// **这是上一条的对照侧**——只钉 `Invalid` 那侧会让「恒 `Invalid`」漂过去。
#[test]
fn five_passes_give_valid() {
    assert_eq!(
        verdict_of([CheckVerdict::Pass; 5]),
        Validity::Valid,
        "五条全 Pass ⇒ Valid（§6.4 的聚合规则）"
    );
}

/// 无 `Fail`、一条 `Unknown` ⇒ `Unknown`（**不是 `Invalid`**）。
///
/// 红条件：把 `Unknown` 也算作 `Invalid`（**收紧档**）⇒ 红。**这一条是「更严的读法」
/// 被明确否掉的落点**：§6.4 说那会把 §123 的正式未知降格成「判为无效」，与 §123 允许
/// 正式表达未知的立意相抵。
#[test]
fn no_fail_with_an_unknown_gives_unknown() {
    assert_eq!(
        verdict_of([
            CheckVerdict::Pass,
            CheckVerdict::Pass,
            CheckVerdict::Pass,
            CheckVerdict::Pass,
            CheckVerdict::Unknown,
        ]),
        Validity::Unknown,
        "无 Fail 而有 Unknown ⇒ Unknown，不得降格为 Invalid（§6.4 的聚合规则）"
    );
}

/// 只给四条（缺 `MissingBoundary`）⇒ `Err(VerifyError::IncompleteTestValidityChecks { .. })`。
///
/// 红条件：构造点不检查臂数（**移除档**）⇒ 红。缺臂名单随错误带回（可辨别是哪一条）。
#[test]
fn a_record_missing_a_check_is_rejected() {
    let four = vec![
        (TestValidityCheck::RequirementMapping, CheckVerdict::Pass),
        (TestValidityCheck::AssertionStrength, CheckVerdict::Pass),
        (TestValidityCheck::ErrorImplWouldPass, CheckVerdict::Pass),
        (TestValidityCheck::MockBypassesRealPath, CheckVerdict::Pass),
    ];
    assert_eq!(
        TestValidity::new(EvidenceId::from_str("ev-1"), four),
        Err(VerifyError::IncompleteTestValidityChecks {
            missing: vec![TestValidityCheck::MissingBoundary],
            repeated: Vec::new(),
        }),
        "缺 MissingBoundary 一条应在构造点被拒（§6.4：五臂各一条，缺即错——这些检查里任何一条都可能是唯一发现「测试无效」的那条，静默缺一条等于静默放宽）"
    );
}

/// **另一侧**——给五条但某臂重复（另一臂缺席）⇒ 同样 `Err`。
///
/// 理由：只钉「少于五条」会让「重复补齐五条」漂过去，而那条路把缺臂静默填平。
/// 重复臂与缺臂都随错误带回（本条与上一条的载荷形态可辨别）。
#[test]
fn a_record_with_a_repeated_check_is_rejected() {
    let mut repeated = five_checks([CheckVerdict::Pass; 5]);
    // 第 5 条本应是 MissingBoundary，改成 RequirementMapping——后者出现两次、前者缺席。
    repeated[4] = (TestValidityCheck::RequirementMapping, CheckVerdict::Pass);
    assert_eq!(
        TestValidity::new(EvidenceId::from_str("ev-1"), repeated),
        Err(VerifyError::IncompleteTestValidityChecks {
            missing: vec![TestValidityCheck::MissingBoundary],
            repeated: vec![TestValidityCheck::RequirementMapping],
        }),
        "某臂重复、另一臂缺席时同样拒绝（五条不等于五臂齐：重复补齐会把缺臂静默填平）"
    );
}

/// 三枚枚举各把它的臂点名列齐：`ALL` 的数目与逐项、编码串逐臂、`parse(as_str(t))` 往返。
///
/// 红条件：改某一枚的串（**取反档**，锚点取 `as_str` 里那一型的那一行）⇒ 该型那一条红；
/// 删该枚举 **`ALL` 常量数组**里的一臂（**移除档**，锚点只取 `ALL` 的数组元素与它的长度标注
/// 两处——只删元素不改标注会因长度标注不符直接编译失败）⇒ 数目断言与逐项比对红（运行期）。
/// **删枚举定义里那一臂不是变红**：逐臂点名引用了它，是用例侧的编译失败（纪律 1(a)、(c)）。
#[test]
fn the_three_enums_each_name_their_arms() {
    // TestValidityCheck：五臂，§191 逐条。
    assert_eq!(
        TestValidityCheck::ALL.len(),
        5,
        "TestValidityCheck 的名单与臂数不符"
    );
    assert_eq!(
        TestValidityCheck::ALL.as_slice(),
        [
            TestValidityCheck::RequirementMapping,
            TestValidityCheck::AssertionStrength,
            TestValidityCheck::ErrorImplWouldPass,
            TestValidityCheck::MockBypassesRealPath,
            TestValidityCheck::MissingBoundary,
        ]
        .as_slice(),
        "ALL 的五项与 §191 的五个检查不符"
    );
    assert_eq!(
        TestValidityCheck::RequirementMapping.as_str(),
        "requirement_mapping",
        "§191 第 1 条（测试对应哪条 Requirement）的落库编码"
    );
    assert_eq!(
        TestValidityCheck::AssertionStrength.as_str(),
        "assertion_strength",
        "§191 第 2 条（断言是否足够强）的落库编码"
    );
    assert_eq!(
        TestValidityCheck::ErrorImplWouldPass.as_str(),
        "error_impl_would_pass",
        "§191 第 3 条（错误实现是否可能仍通过）的落库编码"
    );
    assert_eq!(
        TestValidityCheck::MockBypassesRealPath.as_str(),
        "mock_bypasses_real_path",
        "§191 第 4 条（Mock 是否绕过真实路径）的落库编码"
    );
    assert_eq!(
        TestValidityCheck::MissingBoundary.as_str(),
        "missing_boundary",
        "§191 第 5 条（边界条件是否缺失）的落库编码"
    );

    // CheckVerdict：三臂。
    assert_eq!(CheckVerdict::ALL.len(), 3, "CheckVerdict 的名单与臂数不符");
    assert_eq!(
        CheckVerdict::ALL.as_slice(),
        [
            CheckVerdict::Pass,
            CheckVerdict::Fail,
            CheckVerdict::Unknown,
        ]
        .as_slice(),
        "ALL 的三项与三档判定不符"
    );
    assert_eq!(
        CheckVerdict::Pass.as_str(),
        "pass",
        "CheckVerdict::Pass 的落库编码"
    );
    assert_eq!(
        CheckVerdict::Fail.as_str(),
        "fail",
        "CheckVerdict::Fail 的落库编码"
    );
    assert_eq!(
        CheckVerdict::Unknown.as_str(),
        "unknown",
        "CheckVerdict::Unknown 的落库编码"
    );

    // Validity：三臂。
    assert_eq!(Validity::ALL.len(), 3, "Validity 的名单与臂数不符");
    assert_eq!(
        Validity::ALL.as_slice(),
        [Validity::Valid, Validity::Invalid, Validity::Unknown].as_slice(),
        "ALL 的三项与三档判定不符"
    );
    assert_eq!(
        Validity::Valid.as_str(),
        "valid",
        "Validity::Valid 的落库编码"
    );
    assert_eq!(
        Validity::Invalid.as_str(),
        "invalid",
        "Validity::Invalid 的落库编码"
    );
    assert_eq!(
        Validity::Unknown.as_str(),
        "unknown",
        "Validity::Unknown 的落库编码"
    );

    // 三枚各做一次全臂往返（解码侧没有穷尽 match 的保护，只有遍历能兜住）。
    for check in TestValidityCheck::ALL {
        let encoded = check.as_str();
        assert_eq!(
            TestValidityCheck::parse(encoded),
            Some(check),
            "{encoded} 应解回 {check:?}"
        );
        assert!(
            !encoded.is_empty() && encoded.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
            "落库编码应为小写、多词以 _ 连接（Debug 表示含大写），实际 {encoded}"
        );
    }
    for verdict in CheckVerdict::ALL {
        let encoded = verdict.as_str();
        assert_eq!(
            CheckVerdict::parse(encoded),
            Some(verdict),
            "{encoded} 应解回 {verdict:?}"
        );
        assert!(
            !encoded.is_empty() && encoded.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
            "落库编码应为小写、多词以 _ 连接（Debug 表示含大写），实际 {encoded}"
        );
    }
    for validity in Validity::ALL {
        let encoded = validity.as_str();
        assert_eq!(
            Validity::parse(encoded),
            Some(validity),
            "{encoded} 应解回 {validity:?}"
        );
        assert!(
            !encoded.is_empty() && encoded.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
            "落库编码应为小写、多词以 _ 连接（Debug 表示含大写），实际 {encoded}"
        );
    }
}
