//! `ArtifactType` 的字符串编码（§239）：往返与封闭性。
//!
//! 该编码原先在 `continuum-artifact` 与 `continuum-graph` 的 `persist` 里**各存一张
//! 表**，取值逐字相同。本 crate 的 `PrivacyClass` 已把编码提到枚举自己身上
//! （`as_str` / `parse`），`ArtifactType` 照同一先例收拢（`ArtifactType::as_str` /
//! `parse`），两个 `persist` 改为委托。本文件是那份编码的完整性守卫。

use continuum_artifact::ArtifactType;

/// 遍历 [`ArtifactType::ALL`] 的每一型做往返，并断言名单的数目。
///
/// **本用例守的是解码侧**：`parse` 从 `&str` 出发，编译器点不出漏掉的臂，只有遍历
/// 能发现「某个**已在名单里**的类型解不回去」。编码侧不靠本用例——`as_str` 的 match
/// 穷尽且无通配臂，加型时编译失败。
///
/// **数目断言比它看起来窄**，据实写明：它只在「名单的长度与这里的字面量不一致」时红
/// （有人增删了 `ALL` 的项而没同步本行）。它**发现不了**「加了类型却没加进 `ALL`」
/// ——那种情形下长度不变，往返也走不到新类型，本用例与它一起照过。那一种由 `as_str`
/// 的编译失败拦下：报错点就在 `ArtifactType` 的定义处，而 `ALL` 与 `as_str` 同在一个
/// `impl` 块里紧邻。
#[test]
fn every_artifact_type_round_trips_through_its_encoding() {
    assert_eq!(ArtifactType::ALL.len(), 6, "ArtifactType 的名单与变体数不符");

    for ty in ArtifactType::ALL {
        let encoded = ty.as_str();
        assert_eq!(
            ArtifactType::parse(encoded),
            Some(ty),
            "{encoded} 应解回 {ty:?}"
        );
        assert!(
            !encoded.is_empty() && encoded.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
            "落库编码应为小写、多词以 _ 连接（Debug 表示含大写），实际 {encoded}"
        );
    }

    // 表外取值一律 None：不取默认型——库里读出不认识的串说明有本版本不认得的类型，
    // 取默认会把一份别的东西当成本型处理。
    for bogus in ["", "source tree ", "SourceTree", "source-tree", "nope", "patch_"] {
        assert_eq!(ArtifactType::parse(bogus), None, "{bogus} 不应解出任何类型");
    }
}
