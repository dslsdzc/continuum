//! §34 的生成内容单独授权（设计 §7）的守卫：五枚旗标、四条「算子 → 旗标」映射、
//! fail-closed 的判定两侧。
//!
//! 用例的语料（五个串的字面值、四条映射的算子 id）**手写**、逐条取自设计 §7 与计划：
//! 若从实现侧的 `as_str` 取语料，测的是「它等于它自己」——那是恒真的假照片
//! （设计 §12.2 末对 (a)(f) 的点名同形）。
//!
//! 各条与设计条目的对应：第一条对 §12.1 的臂数与往返；其余对 §12.2 的 (l)(m)(j)(k)；
//! 末一条是 fail-closed 的另一半（计划 `## 遗留` 第三节新增的补件之一，
//! 设计 §12.2 的 (j)(k) 两侧之外的那一侧）。

use continuum_media::{
    GenerativePermission, MediaError, PermittedGenerativeContent, authorize_generative,
    required_permission,
};
use continuum_operator::OperatorId;

/// §12.1 的臂数与往返：`GenerativePermission` 恰是 §34 的五枚，逐臂往返，
/// 且五个串的字面值逐臂手写断言。
///
/// **串的取值是本计划定的**：设计只给了类型名与五个旗标的规范名
/// （`allow_generated_broll` 一类），没给 `as_str` 的串（计划 `## 遗留` 第三节）。
/// 故这里的对照物是**手写字面值**，不是实现侧的 `as_str`。
///
/// **红条件（取反档）**：把 `Music` 与 `Voice` 的串**在 `as_str` 与 `parse` 两处一致地**
/// 互换 ⇒ 逐臂的字面值断言红。**据实记**：这一档**只断往返时不会红**（两处一致地互换
/// 是自洽的重编码），故承重的是字面值那一条；**只在 `as_str` 一处改则
/// `parse(as_str(p)) == Some(p)` 当场为假、往返即红**——两档不是同一枚变异体
/// （口径见计划纪律 1 的第三枚）。
///
/// **`ThreeDGeneration` 这一枚旗标的地位**：§34 的旗标与算子不是一一对应；
/// **本块不给它编算子**（那会落进切分 §八「`3d/` 无对家」那条无解）；
/// **这不是死臂**——一个 Contract 声明 `allow_3d_generation` 而系统里没有 3D 生成算子，
/// 是**系统不提供该能力**。它仍需随其余四枚一起参与往返，并逐臂断字面值。
#[test]
fn the_five_flags_are_exactly_the_five_from_section_34() {
    assert_eq!(
        GenerativePermission::ALL.len(),
        5,
        "GenerativePermission 应为 §34 的五枚"
    );

    // 逐臂往返（遍历 ALL）：`as_str` 的 match 穷尽，加枚时编译不过；解码侧由本循环兜住。
    for p in GenerativePermission::ALL {
        let literal = p.as_str();
        assert_eq!(
            GenerativePermission::parse(literal),
            Some(p),
            "{literal} 的编码应解回自身"
        );
    }

    // 逐臂字面值（手写清单）：串的取值由本计划定（设计未给），故对照物必须手写。
    let expected = [
        ("broll", GenerativePermission::Broll),
        ("voice", GenerativePermission::Voice),
        ("music", GenerativePermission::Music),
        (
            "frame_interpolation",
            GenerativePermission::FrameInterpolation,
        ),
        ("three_d_generation", GenerativePermission::ThreeDGeneration),
    ];
    for (literal, p) in expected {
        assert_eq!(p.as_str(), literal, "{literal} 应是该枚旗标的编码");
        assert_eq!(
            GenerativePermission::parse(literal),
            Some(p),
            "{literal} 应解回手写清单里对应的那一枚"
        );
    }
}

/// 第 (l) 条：四枚生成算子**逐枚断言是哪一枚**旗标（不是断言「是 `Some`」）。
///
/// **红条件（收紧档）**：让四枚**都**返回 `Broll` ⇒ 后三条红。
#[test]
fn each_of_the_four_generative_operators_requires_its_own_flag() {
    let required_of = |id: &str| required_permission(&OperatorId::new(id));

    assert_eq!(
        required_of("generate-broll"),
        Some(GenerativePermission::Broll),
        "generate-broll 应要求 Broll（§7.2 四条映射之一）"
    );
    assert_eq!(
        required_of("generate-voice"),
        Some(GenerativePermission::Voice),
        "generate-voice 应要求 Voice（§7.2 四条映射之一）"
    );
    assert_eq!(
        required_of("generate-music"),
        Some(GenerativePermission::Music),
        "generate-music 应要求 Music（§7.2 四条映射之一）"
    );
    assert_eq!(
        required_of("frame-interpolation"),
        Some(GenerativePermission::FrameInterpolation),
        "frame-interpolation 应要求 FrameInterpolation（§7.2 四条映射之一）"
    );
}

/// 第 (m) 条（另一侧）：非生成算子不触 §34——`required_permission` 返回 `None`。
///
/// 红条件（放宽档）：让 `required_permission` 一律返回 `Some(Broll)` ⇒ 红。
#[test]
fn a_non_generative_operator_requires_no_flag() {
    for id in ["render", "transcribe", "verify-deterministic"] {
        assert_eq!(
            required_permission(&OperatorId::new(id)),
            None,
            "{id} 不是生成类，不应要求任何生成旗标"
        );
    }
}

/// 第 (j) 条（反例侧）：未声明 `Broll` 时 `generate-broll` 被拒——
/// `Err(MediaError::NotPermitted { .. })`，**且 `required` 正是 `Broll`**
/// （不是「返回了那一臂」）。
///
/// 红条件（取反档）：把判定改成「未声明 ⇒ 放行」⇒ 红。
#[test]
fn an_ungranted_generative_operator_is_rejected() {
    let granted = PermittedGenerativeContent::none();
    let operator = OperatorId::new("generate-broll");

    let err = authorize_generative(&operator, &granted)
        .expect_err("未声明 Broll 时 generate-broll 必须被拒（§34「不得擅自」）");

    match err {
        MediaError::NotPermitted {
            operator: echoed,
            required,
        } => {
            assert_eq!(echoed, operator, "operator 字段应是判定时的那一枚算子");
            assert_eq!(required, GenerativePermission::Broll, "required 应是 Broll");
        }
        other => panic!("应为 MediaError::NotPermitted，实际 {other:?}"),
    }
}

/// 第 (k) 条（另一侧）：声明为真时放行 `Ok`。
///
/// **没有这一条，第 (j) 条可以由「一律 `Err`」满足。**
#[test]
fn a_granted_generative_operator_is_allowed() {
    let granted = PermittedGenerativeContent::from_permissions(&[GenerativePermission::Broll]);
    let result = authorize_generative(&OperatorId::new("generate-broll"), &granted);
    assert!(
        result.is_ok(),
        "声明 Broll 后 generate-broll 应放行，实际 {result:?}"
    );
}

/// fail-closed 的那一侧：**用本 crate 提供的「空集」构造**得到的
/// `PermittedGenerativeContent`，对四枚生成算子**逐枚** `Err`。
///
/// **这一条钉的是「缺省不放行」**：设计 §7.2 逐字「**这里不是「用 `unwrap_or(true)`
/// 让缺省放行」**」：§34 `:1431` 逐字是「不得擅自」，而 `PermittedGenerativeContent`
/// 的**缺省构造**若是「全放行」，那么一个没写旗标的 Contract 就自动获得全部生成权限
/// ——那正是「擅自」的形态。
///
/// **红条件（取反档）**：把「空集」构造实现成全放行（或给它一个全放行的 `Default`）
/// ⇒ 四条全红。
#[test]
fn the_empty_grant_set_permits_nothing() {
    let empty = PermittedGenerativeContent::none();
    let cases = [
        ("generate-broll", GenerativePermission::Broll),
        ("generate-voice", GenerativePermission::Voice),
        ("generate-music", GenerativePermission::Music),
        (
            "frame-interpolation",
            GenerativePermission::FrameInterpolation,
        ),
    ];

    for (id, expected) in cases {
        match authorize_generative(&OperatorId::new(id), &empty) {
            Err(MediaError::NotPermitted { required, .. }) => {
                assert_eq!(required, expected, "{id} 被拒时 required 应是 {expected:?}");
            }
            Ok(()) => panic!(
                "{id} 在空集下必须被拒——「没写旗标的 Contract」不得自动获得生成权限（§34「不得擅自」）"
            ),
            Err(other) => panic!("{id} 应返回 NotPermitted，实际 {other:?}"),
        }
    }
}
