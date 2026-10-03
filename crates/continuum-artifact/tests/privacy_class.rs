//! `PrivacyClass` 的字符串编码（§243）：往返与封闭性。
//!
//! 该编码原先只在 `src/persist.rs` 内有一份私有的编解码表。策略条件
//! （`continuum-policy` 的 `privacy_class` 事实）也要读同一个串，故编码提到了
//! 枚举自己身上（`PrivacyClass::as_str` / `parse`），库列与策略条件共用一份。
//! 本文件是那份编码的完整性守卫。

use continuum_artifact::PrivacyClass;

/// 遍历**全部**五档做往返，并断言名单的数目。
///
/// 数目断言是必需的：漏加新档时名单长度不变，只遍历名单的断言照过。
///
/// 解码侧没有穷尽 match 的保护（来源是 `&str` 而非枚举），所以这条往返断言不是
/// 补充，而是那一侧唯一的守卫；编码侧由 `as_str` 的穷尽 match 在编译期守住。
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
