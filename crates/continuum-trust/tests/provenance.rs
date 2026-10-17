//! Task 2：`ContextProvenance` 与 `ArtifactProducer` 的构造入口判别位与 serde 往返。
//!
//! 注释：`provider_supplied()` 不是 `web_untrusted()` 的别名，而是「第三方 provider 文本」
//! 这个来源的具名入口；两者投影到同一枚标签是设计 §3.3 的映射，不是本类型层面的合并。
//!
//! 红条件：把 `provider_supplied()` 改成 `TrustedRuntime` → 本文件的显式 `assert_ne!` 红；
//! 把 `conversation_history()` 的 `historical` 写成 `false` → 构造入口测试红；
//! 把 `memory` 的 `claim_id` 丢掉 → `Memory` 断言红。

use continuum_trust::{ArtifactProducer, ContextProvenance};

#[test]
fn constructors_carry_their_discriminators() {
    assert_eq!(
        ContextProvenance::user_instruction(),
        ContextProvenance::UserInstruction { historical: false }
    );
    assert_eq!(
        ContextProvenance::conversation_history(),
        ContextProvenance::UserInstruction { historical: true }
    );
    assert_eq!(
        ContextProvenance::trusted_runtime(),
        ContextProvenance::TrustedRuntime
    );

    // Review Focus 2：第三方 provider 文本默认 fail-safe 到 WebUntrusted，
    // 不得与 Runtime 本层 authored 的文本混为一谈。
    assert_eq!(
        ContextProvenance::provider_supplied(),
        ContextProvenance::WebUntrusted
    );
    assert_ne!(
        ContextProvenance::provider_supplied(),
        ContextProvenance::trusted_runtime(),
        "provider 提供的文本不得被标成 TRUSTED_RUNTIME"
    );

    assert_eq!(
        ContextProvenance::project_data(),
        ContextProvenance::ProjectData
    );
    assert_eq!(
        ContextProvenance::web_untrusted(),
        ContextProvenance::WebUntrusted
    );
    assert_eq!(
        ContextProvenance::tool_output(true),
        ContextProvenance::ToolOutput { external: true }
    );
    assert_eq!(
        ContextProvenance::tool_output(false),
        ContextProvenance::ToolOutput { external: false }
    );
    assert_eq!(
        ContextProvenance::memory("c1"),
        ContextProvenance::Memory {
            claim_id: "c1".into()
        }
    );
    assert_eq!(
        ContextProvenance::memory(String::from("c1")),
        ContextProvenance::Memory {
            claim_id: "c1".into()
        }
    );
    assert_eq!(
        ContextProvenance::model_output(),
        ContextProvenance::ModelOutput
    );

    for producer in [
        ArtifactProducer::Model,
        ArtifactProducer::Tool,
        ArtifactProducer::External,
        ArtifactProducer::Project,
        ArtifactProducer::Unknown,
    ] {
        assert_eq!(
            ContextProvenance::artifact(producer),
            ContextProvenance::Artifact { producer },
            "artifact 构造入口必须原样带上生产者判别位"
        );
    }
}

#[test]
fn provenance_and_producer_round_trip_through_serde() {
    // 只测往返，不写死派生 JSON 的具体形状：这两枚没有规范字符串编码。
    let provenances = [
        ContextProvenance::UserInstruction { historical: false },
        ContextProvenance::UserInstruction { historical: true },
        ContextProvenance::TrustedRuntime,
        ContextProvenance::ProjectData,
        ContextProvenance::WebUntrusted,
        ContextProvenance::ToolOutput { external: true },
        ContextProvenance::ToolOutput { external: false },
        ContextProvenance::Memory {
            claim_id: "c1".into(),
        },
        ContextProvenance::ModelOutput,
        ContextProvenance::Artifact {
            producer: ArtifactProducer::Unknown,
        },
    ];

    for provenance in provenances {
        let encoded = serde_json::to_string(&provenance).expect("provenance 必须可序列化");
        let decoded: ContextProvenance =
            serde_json::from_str(&encoded).expect("provenance 必须可反序列化");
        assert_eq!(decoded, provenance, "provenance 的 serde 往返必须回到原值");
    }

    for producer in [
        ArtifactProducer::Model,
        ArtifactProducer::Tool,
        ArtifactProducer::External,
        ArtifactProducer::Project,
        ArtifactProducer::Unknown,
    ] {
        let encoded = serde_json::to_string(&producer).expect("producer 必须可序列化");
        let decoded: ArtifactProducer =
            serde_json::from_str(&encoded).expect("producer 必须可反序列化");
        assert_eq!(decoded, producer, "producer 的 serde 往返必须回到原值");
    }
}
