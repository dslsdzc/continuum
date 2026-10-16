//! `VerificationPolicy` / `VerificationProfile` / `VerifierLevel` 的形状与解析
//! （设计 §5.1–§5.3、§7.1；计划 Task 4 的 Step 1）。
//!
//! 三条口径（照计划的测试纪律）：
//! - 失败路径的用例断言**是哪一种** `Err`（`UnknownVerificationPolicy` 的四类输入逐条、
//!   `ProfileMissing`），不只「返回了 `Err`」；
//! - 「守卫」两侧成对：`a_well_formed_policy_parses_field_by_field` 是拒绝侧的对照正例，
//!   `a_present_profile_is_returned_field_by_field` 是 `Missing ⇒ Err` 的另一侧；
//! - 标「编译期照片」的地方**不是运行期红**——判据是「编译不过」或「从编译失败变成
//!   编译通过」，不假称它们会跑红（纪律 1(c)：「编译失败不是变红」）。

use continuum_verify::{
    EvidenceType, VerificationPolicy, VerificationProfile, VerificationProfileSource,
    VerifierLevel, VerifyError, profile_for,
};
use serde_json::{Value, json};

/// (j) 的第一子例：`Value::Null` ⇒ `Default`（§5.1 的两侧之一）。
///
/// **这不是「无要求」**：任务级 `VerificationProfile` 仍然适用（§5.1 原文；
/// 下游从覆盖判定的一侧另有用例钉这一点，本文件只钉解析侧）。
///
/// 红条件：`Value::Null` 那一支返回 `Err`（**收紧档**）⇒ 红。
#[test]
fn a_null_policy_is_the_default_policy() {
    let parsed = VerificationPolicy::from_value(&Value::Null)
        .expect("Null 是「无限定」不是「不认识」：应解析为 Default（§5.1 第一侧）");
    assert_eq!(
        parsed,
        VerificationPolicy::default(),
        "Null 应给出 Default 的两个字段值"
    );
    assert_eq!(
        parsed.min_verifier_level, None,
        "Default 的 min_verifier_level 是 None（不限，按 §262 的顺序取最高可用级）"
    );
    assert!(
        parsed.required_evidence_types.is_empty(),
        "Default 的 required_evidence_types 为空（本节点无额外要求）"
    );
}

/// (j) 的第二子例：**非空但不认识**的四种输入逐条 ⇒ 各一条
/// `Err(VerifyError::UnknownVerificationPolicy)`（§5.1 的第二侧）。
///
/// 「缺字段」一侧两条：只拍一侧的话，另一侧「缺省即默认」的宽松实现会漂过
/// （把缺省当默认，正是「一个写着内容的策略被当成空策略」的另一条路）。
///
/// 红条件：未知分支返回 `Default` 而不是 `Err`（**放宽档**）⇒ 五条全红。
/// **这一档若不红，说明只拍了空策略那一侧**（设计 §12.2 的预告）。
#[test]
fn a_non_empty_unrecognized_policy_is_rejected() {
    // 1) 缺字段：缺 min_verifier_level（只写了 required_evidence_types）。
    assert_eq!(
        VerificationPolicy::from_value(&json!({"required_evidence_types": []})),
        Err(VerifyError::UnknownVerificationPolicy),
        "缺 min_verifier_level 应被拒（§5.1 第 2 条的「缺字段」）"
    );
    // 2) 缺字段：缺 required_evidence_types（只写了 min_verifier_level）。
    assert_eq!(
        VerificationPolicy::from_value(&json!({"min_verifier_level": null})),
        Err(VerifyError::UnknownVerificationPolicy),
        "缺 required_evidence_types 应被拒（§5.1 第 2 条的「缺字段」）"
    );
    // 3) 多出未定义字段。
    assert_eq!(
        VerificationPolicy::from_value(&json!({
            "min_verifier_level": null,
            "required_evidence_types": [],
            "extra": true,
        })),
        Err(VerifyError::UnknownVerificationPolicy),
        "多出未定义字段应被拒（§5.1 第 2 条）"
    );
    // 4) min_verifier_level 是五级序之外的串。
    assert_eq!(
        VerificationPolicy::from_value(&json!({
            "min_verifier_level": "gpt_4o_reviewer",
            "required_evidence_types": [],
        })),
        Err(VerifyError::UnknownVerificationPolicy),
        "五级序之外的级别串应被拒（§5.1 第 2 条）"
    );
    // 5) required_evidence_types 含未知类型名。
    assert_eq!(
        VerificationPolicy::from_value(&json!({
            "min_verifier_level": null,
            "required_evidence_types": ["smoke_test"],
        })),
        Err(VerifyError::UnknownVerificationPolicy),
        "未知证据类型名应被拒（§5.1 第 2 条）"
    );
}

/// 正例：两个字段各给一个合法值 ⇒ `Ok`，且**逐字段**断言读回值。
///
/// **没有这一条，上一条可以由「一律 `Err`」满足**（拒绝侧的守卫必须成对）。
///
/// 红条件：任一字段的解码取反（**取反档**，改 `as_str`/`parse` 或 `from_value` 的
/// 对应分支）⇒ 该字段的断言红。
#[test]
fn a_well_formed_policy_parses_field_by_field() {
    let parsed = VerificationPolicy::from_value(&json!({
        "min_verifier_level": "cross_family_verifier",
        "required_evidence_types": ["property", "fuzz"],
    }))
    .expect("两个字段各给一个合法值时应解析成功（§5.1 的第二侧之外：认识即解析）");

    assert_eq!(
        parsed.min_verifier_level,
        Some(VerifierLevel::CrossFamilyVerifier),
        "min_verifier_level 应解回五级序里的那一级"
    );
    assert_eq!(
        parsed.required_evidence_types,
        vec![EvidenceType::Property, EvidenceType::Fuzz],
        "required_evidence_types 应逐项解回证据类型（顺序照原值）"
    );
}

/// (i)：无档案 ⇒ `Err(ProfileMissing)`，**不是**「无要求」的档案（§5.3 第 3 条）。
///
/// 这一条钉的是 **fail-open 的那一侧**：无档案 ⇒ 无要求 ⇒ 不阻断。
///
/// 红条件：`Missing` 返回「无要求的档案」（`required_evidence_types: vec![]`）
/// 而不是 `Err`（**放宽档**）⇒ 红。
#[test]
fn an_absent_profile_is_an_error_not_an_empty_profile() {
    assert_eq!(
        profile_for(&VerificationProfileSource::Missing),
        Err(VerifyError::ProfileMissing),
        "无档案应返回 Err(ProfileMissing)：把它当「无要求」会让每项 REQUIRED 的 required 集合为空、覆盖判定无缺项，即「无档案 ⇒ 无要求 ⇒ 不阻断」（§5.3 第 3 条）"
    );
}

/// **另一侧**：`Provided(p)` ⇒ `Ok`，六个字段逐字段相等
/// （否则可由「一律 `Err`」满足）。
///
/// 红条件：任一字段在取得点上被改写或丢失（**取反档**）⇒ 该字段的断言红。
#[test]
fn a_present_profile_is_returned_field_by_field() {
    let profile = VerificationProfile {
        required_evidence_types: vec![EvidenceType::Test, EvidenceType::Fuzz],
        optional_evidence_types: vec![EvidenceType::Benchmark],
        fuzz_budget: Some(json!({"seconds": 30})),
        mutation_threshold: Some(json!(0.75)),
        property_requirements: vec![json!({"cases": 100})],
        independent_verifier_required: true,
    };
    let returned = profile_for(&VerificationProfileSource::Provided(profile))
        .expect("有档案时应原样返回（§5.3 第 3 条的对照侧）");

    assert_eq!(
        returned.required_evidence_types,
        vec![EvidenceType::Test, EvidenceType::Fuzz],
        "required_evidence_types 应逐项相等"
    );
    assert_eq!(
        returned.optional_evidence_types,
        vec![EvidenceType::Benchmark],
        "optional_evidence_types 应逐项相等（只存不判，但不得在取得点上丢失）"
    );
    assert_eq!(
        returned.fuzz_budget,
        Some(json!({"seconds": 30})),
        "fuzz_budget 应逐值相等（只存不判）"
    );
    assert_eq!(
        returned.mutation_threshold,
        Some(json!(0.75)),
        "mutation_threshold 应逐值相等（只存不判）"
    );
    assert_eq!(
        returned.property_requirements,
        vec![json!({"cases": 100})],
        "property_requirements 应逐项相等（只存不判）"
    );
    assert!(
        returned.independent_verifier_required,
        "independent_verifier_required 应相等（进判定）"
    );
}

/// §261 的六个字段**按名穷尽解构**，**不带 `..`**。
///
/// **这是编译期照片，不是运行期红**：红条件（**移除档**）是给 `VerificationProfile`
/// 加第七个字段（或改名）⇒ 本函数的解构与结构体字面量**编译不过**。观测方式：
/// 「本段从编译通过变成编译不过」——本用例不会跑红（纪律 1(c)）。
///
/// 各绑定都要被用掉：未用变量会让本块的门（0 warning）红了，故顺带让六个字段
/// 「存在且类型对」在运行期也各有一处读点。
#[test]
fn the_six_fields_of_261_are_all_present() {
    let profile = VerificationProfile {
        required_evidence_types: vec![EvidenceType::Test],
        optional_evidence_types: vec![EvidenceType::Benchmark],
        fuzz_budget: Some(json!({"seconds": 30})),
        mutation_threshold: Some(json!(0.75)),
        property_requirements: vec![json!({"cases": 100})],
        independent_verifier_required: true,
    };
    let VerificationProfile {
        required_evidence_types,
        optional_evidence_types,
        fuzz_budget,
        mutation_threshold,
        property_requirements,
        independent_verifier_required,
    } = profile;

    assert_eq!(required_evidence_types, vec![EvidenceType::Test], "字段一");
    assert_eq!(
        optional_evidence_types,
        vec![EvidenceType::Benchmark],
        "字段二"
    );
    assert_eq!(fuzz_budget, Some(json!({"seconds": 30})), "字段三");
    assert_eq!(mutation_threshold, Some(json!(0.75)), "字段四");
    assert_eq!(property_requirements, vec![json!({"cases": 100})], "字段五");
    assert!(independent_verifier_required, "字段六");
}

/// §262 的五臂：数目、逐臂点名与**四条相邻 `<`**（不抽代表）。
///
/// `Ord` 由声明序派生，**声明序即 §262 的优先序**（设计 §7.1）：改一次声明序就改一次
/// 选择结果。红条件（**取反档**）：把枚举声明序改一处 ⇒ **该对** `assert!` 红——
/// 四条里哪一对被改就哪一对红，其余断言对具体次序不敏感的那一半不红。
#[test]
fn the_five_levels_are_declared_in_the_262_priority_order() {
    assert_eq!(
        VerifierLevel::ALL.len(),
        5,
        "VerifierLevel 的名单与臂数不符"
    );
    assert_eq!(
        VerifierLevel::ALL.as_slice(),
        [
            VerifierLevel::DeterministicChecker,
            VerifierLevel::FormalStaticChecker,
            VerifierLevel::IndependentModelVerifier,
            VerifierLevel::CrossFamilyVerifier,
            VerifierLevel::HumanReview,
        ]
        .as_slice(),
        "ALL 的五项与 §262 的优先级序不符"
    );

    // 四条相邻序逐对断言：每一条都可能是唯一决定「选谁」的那条。
    assert!(
        VerifierLevel::DeterministicChecker < VerifierLevel::FormalStaticChecker,
        "§262 第 1 级（deterministic checker）应优先于第 2 级（formal/static checker）"
    );
    assert!(
        VerifierLevel::FormalStaticChecker < VerifierLevel::IndependentModelVerifier,
        "§262 第 2 级应优先于第 3 级（independent model verifier）"
    );
    assert!(
        VerifierLevel::IndependentModelVerifier < VerifierLevel::CrossFamilyVerifier,
        "§262 第 3 级应优先于第 4 级（cross-family verifier）"
    );
    assert!(
        VerifierLevel::CrossFamilyVerifier < VerifierLevel::HumanReview,
        "§262 第 4 级应优先于第 5 级（human review）"
    );
}

/// [`VerifierLevel::ALL`] 的**逐臂往返**与串名（设计 §12.1 的「每臂一条往返 ＋ 数目
/// 断言」，编码纪律照 `EvidenceType` 先例）。
///
/// **解码侧**（`parse` 从 `&str` 出发，编译器点不出漏掉的臂）由遍历兜住；
/// **编码侧**不靠本用例——`as_str` 的 match 穷尽且无通配臂，加臂时编译失败。
///
/// 串名没有规范逐字出处（§262 只给级别名）：五枚串名是本块的命名，照 §4.2 的编码口径
/// （小写、多词以 `_` 连接）。红条件（**取反档**）：改 `as_str` 或 `parse` 里某一臂的
/// 那一行 ⇒ 该臂的串名/往返断言红（两处各自是唯一锚点，不共用）。
#[test]
fn every_level_round_trips_through_its_encoding() {
    // 逐臂串名：一条一条，不抽代表。
    assert_eq!(
        VerifierLevel::DeterministicChecker.as_str(),
        "deterministic_checker",
        "§262 第 1 级的编码"
    );
    assert_eq!(
        VerifierLevel::FormalStaticChecker.as_str(),
        "formal_static_checker",
        "§262 第 2 级的编码"
    );
    assert_eq!(
        VerifierLevel::IndependentModelVerifier.as_str(),
        "independent_model_verifier",
        "§262 第 3 级的编码"
    );
    assert_eq!(
        VerifierLevel::CrossFamilyVerifier.as_str(),
        "cross_family_verifier",
        "§262 第 4 级的编码"
    );
    assert_eq!(
        VerifierLevel::HumanReview.as_str(),
        "human_review",
        "§262 第 5 级的编码"
    );

    for level in VerifierLevel::ALL {
        let encoded = level.as_str();
        assert_eq!(
            VerifierLevel::parse(encoded),
            Some(level),
            "{encoded} 应解回 {level:?}"
        );
        assert!(
            !encoded.is_empty() && encoded.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
            "编码应为小写、多词以 _ 连接（Debug 表示含大写），实际 {encoded}"
        );
    }
}
