//! `EvidenceType` 的落库编码（设计 §4.2）：十四臂的名单、往返与封闭性。
//!
//! 本文件是那份编码的完整性守卫：`ALL` 的十四项**逐字**钉住（含数目断言），
//! `parse` 从 `&str` 出发的往返**逐型**遍历，三枚由领域算子块报出的臂**逐枚**钉串名，
//! 表外取值一律 `None`。编码本身写在 `src/evidence_type.rs` 上，与该类型同址。

use continuum_verify::EvidenceType;

/// 十四臂的名单与数目：`ALL` 的十四项**逐字列出**（一项不抽代表）。
///
/// **数目断言比它看起来窄**，据实写明：它只在「名单的长度与这里的字面量不一致」时红
/// （有人增删了 `ALL` 的项而没同步本行）。它**发现不了**「加了臂却没加进 `ALL`」——
/// 那种情形下长度不变，逐字比对也走不到新臂，本用例与它一起照过。那一种由 `as_str`
/// 的编译失败拦下：加臂时报错点就在 `EvidenceType` 的定义处，而 `ALL` 与 `as_str`
/// 同在一个 `impl` 块里紧邻。
///
/// **不做**「`ALL.len()` 与另一个从 `ALL` 派生的量比较」——那是恒真的假照片
/// （设计 §12.2 的「一条预防」）。
#[test]
fn the_type_list_names_the_fourteen_arms_in_one_place() {
    assert_eq!(EvidenceType::ALL.len(), 14, "EvidenceType 的名单与变体数不符");

    assert_eq!(
        EvidenceType::ALL,
        [
            EvidenceType::Test,
            EvidenceType::Fuzz,
            EvidenceType::Property,
            EvidenceType::FormalProof,
            EvidenceType::StaticAnalysis,
            EvidenceType::Benchmark,
            EvidenceType::VisualCheck,
            EvidenceType::MetadataCheck,
            EvidenceType::HumanConfirmation,
            EvidenceType::Differential,
            EvidenceType::Metamorphic,
            EvidenceType::ModelReview,
            EvidenceType::ContradictionCheck,
            EvidenceType::CitationVerification,
        ],
        "ALL 的十四项与设计 §4.2 的表不符"
    );
}

/// 遍历 [`EvidenceType::ALL`] 的每一型做往返。
///
/// **本用例守的是解码侧**：`parse` 从 `&str` 出发，编译器点不出漏掉的臂，只有遍历
/// 能发现「某个**已在名单里**的类型解不回去」。编码侧不靠本用例——`as_str` 的
/// match 穷尽且无通配臂，加臂时编译失败。
///
/// 红条件（取反档）：改 `as_str` 里那一型的那一行，**或**改 `parse` 里那一型的
/// 那个字面量——两处各自是唯一锚点，不共用。
#[test]
fn every_evidence_type_round_trips_through_its_encoding() {
    for ty in EvidenceType::ALL {
        let encoded = ty.as_str();
        assert_eq!(
            EvidenceType::parse(encoded),
            Some(ty),
            "{encoded} 应解回 {ty:?}"
        );
        assert!(
            !encoded.is_empty() && encoded.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
            "落库编码应为小写、多词以 _ 连接（Debug 表示含大写），实际 {encoded}"
        );
    }
}

/// 三枚由领域算子块报出的臂（设计 §4.2 末三行）的**逐枚**串名：一条一条，不抽代表。
///
/// **这一条单独存在**是因为这三臂的出处与前十一臂不是同一种：`ModelReview` 由 P5c
/// 报出，`ContradictionCheck` / `CitationVerification` 由 P5d 报出；串名是**本设计的
/// 命名**，不是规范逐字（§262 只给级别名、§330 只给步骤名）。红条件：改其中一枚的串
/// （取反档）⇒ 该条红。
#[test]
fn the_three_arms_reported_by_the_domain_blocks_carry_their_designated_encodings() {
    assert_eq!(
        EvidenceType::ModelReview.as_str(),
        "model_review",
        "ModelReview 的串名是设计 §4.2 末三行的命名"
    );
    assert_eq!(
        EvidenceType::ContradictionCheck.as_str(),
        "contradiction_check",
        "ContradictionCheck 的串名是设计 §4.2 末三行的命名"
    );
    assert_eq!(
        EvidenceType::CitationVerification.as_str(),
        "citation_verification",
        "CitationVerification 的串名是设计 §4.2 末三行的命名"
    );
}

/// 表外取值一律 `None`：不取默认型——库里读出不认识的串说明有本版本不认得的证据
/// 类型，取默认会把一份别的东西当成本型处理。
///
/// 三条各自钉一侧：未列入的词、大写（编码一律小写）、空串。红条件（放宽档）：
/// `parse` 末支返回某个默认型而不是 `None` ⇒ 三条全红。
#[test]
fn an_out_of_table_string_is_rejected() {
    assert_eq!(
        EvidenceType::parse("unknown_evidence"),
        None,
        "表外取值不应解出任何类型"
    );
    assert_eq!(
        EvidenceType::parse("Test"),
        None,
        "编码一律小写，Test 不应解出任何类型"
    );
    assert_eq!(EvidenceType::parse(""), None, "空串不应解出任何类型");
}
