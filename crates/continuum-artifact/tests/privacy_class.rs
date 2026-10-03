//! `PrivacyClass` 的字符串编码（§243）：往返与封闭性。
//!
//! 该编码原先只在 `src/persist.rs` 内有一份私有的编解码表。策略条件
//! （`continuum-policy` 的 `privacy_class` 事实）也要读同一个串，故编码提到了
//! 枚举自己身上（`PrivacyClass::as_str` / `parse`），库列与策略条件共用一份。
//! 本文件是那份编码的完整性守卫。

use continuum_artifact::PrivacyClass;

/// 遍历 [`PrivacyClass::ALL`] 的每一档做往返，并断言名单的数目。
///
/// **本用例守的是解码侧**：`parse` 从 `&str` 出发，编译器点不出漏掉的臂，只有遍历
/// 能发现「某个**已在名单里**的档次解不回去」。编码侧不靠本用例——`as_str` 的 match
/// 穷尽且无通配臂，加档时编译失败。
///
/// **数目断言比它看起来窄**，据实写明：它只在「名单的长度与这里的字面量不一致」时红
/// （有人增删了 `ALL` 的项而没同步本行）。它**发现不了**「加了档次却没加进 `ALL`」
/// ——那种情形下长度不变，往返也走不到新档，本用例与它一起照过。那一种由 `as_str`
/// 的编译失败拦下：报错点就在 `PrivacyClass` 的定义处，而 `ALL` 与 `as_str` 同在一个
/// `impl` 块里紧邻。
#[test]
fn every_privacy_class_round_trips_through_its_encoding() {
    assert_eq!(PrivacyClass::ALL.len(), 5, "PrivacyClass 的名单与变体数不符");

    for privacy in PrivacyClass::ALL {
        let encoded = privacy.as_str();
        assert_eq!(
            PrivacyClass::parse(encoded),
            Some(privacy),
            "{encoded} 应解回 {privacy:?}"
        );
        assert!(
            !encoded.is_empty() && encoded.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
            "落库编码应为小写、多词以 _ 连接（Debug 表示含大写），实际 {encoded}"
        );
    }

    // 表外取值一律 None：不取默认档次——隐私等级是策略裁决的依据，取默认会让
    // 策略表漏判。
    for bogus in ["", "personal ", "Personal", "local only", "nope", "public_"] {
        assert_eq!(PrivacyClass::parse(bogus), None, "{bogus} 不应解出任何档次");
    }
}
