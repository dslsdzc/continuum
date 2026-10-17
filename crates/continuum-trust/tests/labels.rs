//! §9.1 (a)–(e)：七个标签的编码封闭、往返、别名拒绝、serde 与谓词划分。
//!
//! **期望名单手写**，不从 `ALL` 或 `as_str` 派生——从被测清单里取语料就是恒真的假照片。
//!
//! 红条件（供 Task 6 变异复核）：
//! - 把 `as_str` 的 `WEB_UNTRUSTED` 改成 `WEB` → (a)(b)(d) 红，(g) 正例仍绿；
//! - 加第八枚而不补四处 `match` → 编译失败（E0004）；
//! - 删一枚 → (a) 手写名单红且 `as_str` 少一臂。

use continuum_trust::ContextTrust;

/// §281 的七个串，逐字手写。**不从 `ALL` 派生**。
const SPEC_LABELS: [&str; 7] = [
    "USER_INSTRUCTION",
    "TRUSTED_RUNTIME",
    "PROJECT_DATA",
    "WEB_UNTRUSTED",
    "TOOL_OUTPUT",
    "MEMORY",
    "MODEL_OUTPUT",
];

/// 另一份手写映射：**无通配臂的穷尽 `match`**（crate 内四处 + 测试一处）。
/// 加第八枚时这里也编译不过，实现者必须重新回答它的规范串。
fn expected_str(t: ContextTrust) -> &'static str {
    match t {
        ContextTrust::UserInstruction => "USER_INSTRUCTION",
        ContextTrust::TrustedRuntime => "TRUSTED_RUNTIME",
        ContextTrust::ProjectData => "PROJECT_DATA",
        ContextTrust::WebUntrusted => "WEB_UNTRUSTED",
        ContextTrust::ToolOutput => "TOOL_OUTPUT",
        ContextTrust::Memory => "MEMORY",
        ContextTrust::ModelOutput => "MODEL_OUTPUT",
    }
}

#[test]
fn the_seven_labels_are_exactly_the_specs_seven() {
    assert_eq!(ContextTrust::ALL.len(), 7, "§281 的标签恰有七枚");

    for (i, actual) in ContextTrust::ALL.iter().enumerate() {
        assert_eq!(
            actual.as_str(),
            SPEC_LABELS[i],
            "第 {i} 枚标签的规范串必须逐字等于 §281 的书写"
        );
    }

    for actual in ContextTrust::ALL {
        assert_eq!(
            actual.as_str(),
            expected_str(actual),
            "两处手写名单必须一致"
        );
    }
}

#[test]
fn every_label_round_trips_through_its_encoding() {
    for actual in ContextTrust::ALL {
        assert_eq!(
            ContextTrust::parse(actual.as_str()),
            Some(actual),
            "{} 的编码必须能严格往返",
            actual.as_str()
        );
    }
}

#[test]
fn the_encoding_rejects_aliases_and_lowercase() {
    for rejected in [
        "user_instruction",
        "USER INSTRUCTION",
        " user_instruction ",
        "",
        "UserInstruction",
        "USER-INSTRUCTION",
    ] {
        assert_eq!(
            ContextTrust::parse(rejected),
            None,
            "表外串 {rejected:?} 必须被严格逆拒绝"
        );
    }
}

#[test]
fn serde_uses_the_same_encoding_as_as_str() {
    for actual in ContextTrust::ALL {
        let encoded = serde_json::to_value(actual).expect("标签必须可序列化");
        assert_eq!(
            encoded,
            serde_json::json!(actual.as_str()),
            "serde 编码必须与 as_str 逐字一致"
        );

        let decoded: ContextTrust =
            serde_json::from_value(serde_json::json!(actual.as_str())).expect("标签必须可反序列化");
        assert_eq!(decoded, actual, "serde 解码必须回到原值");
    }

    let outside = serde_json::from_value::<ContextTrust>(serde_json::json!("user_instruction"));
    assert!(outside.is_err(), "表外串反序列化必须报错");
}

#[test]
fn the_three_predicates_partition_the_seven() {
    // 三组真值集逐条手写，不从谓词或 ALL 派生。
    let instruction_side = [ContextTrust::UserInstruction, ContextTrust::TrustedRuntime];
    let data_only = [
        ContextTrust::ProjectData,
        ContextTrust::WebUntrusted,
        ContextTrust::ToolOutput,
        ContextTrust::Memory,
        ContextTrust::ModelOutput,
    ];

    for actual in ContextTrust::ALL {
        assert_eq!(
            actual.may_sit_in_instructions(),
            instruction_side.contains(&actual),
            "may_sit_in_instructions 的真值集恰为 {{UserInstruction, TrustedRuntime}}"
        );
        assert_eq!(
            actual.is_data_only(),
            data_only.contains(&actual),
            "is_data_only 的真值集恰为五枚 data-only"
        );
        assert_eq!(
            actual.is_user_instruction(),
            actual == ContextTrust::UserInstruction,
            "is_user_instruction 的真值集恰为 {{UserInstruction}}"
        );
    }

    // 子集：is_user_instruction ⊆ may_sit_in_instructions。
    for actual in ContextTrust::ALL {
        if actual.is_user_instruction() {
            assert!(
                actual.may_sit_in_instructions(),
                "is_user_instruction 的真值集必须是 may_sit_in_instructions 真值集的子集"
            );
        }
    }

    // 互补：并集覆盖 ALL、交集为空。
    for actual in ContextTrust::ALL {
        assert!(
            actual.is_data_only() != actual.may_sit_in_instructions(),
            "is_data_only 与 may_sit_in_instructions 必须互补"
        );
    }
}

#[test]
fn display_delegates_to_as_str() {
    for actual in ContextTrust::ALL {
        assert_eq!(
            format!("{actual}"),
            actual.as_str(),
            "Display 必须委托 as_str，与编码同源"
        );
    }
}
