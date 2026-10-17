//! Task 3 §9.1 (f)、(p)：`label()` 的全部映射，逐条写死，不抽代表。
//!
//! 注释：`web_untrusted()` 与 `provider_supplied()` 是同一行的两个入口；
//! `UserInstruction` 的两个 `historical` 分支投影到同一枚标签，角色由 `historical` 位
//! 与数组位置表达，不在标签里。
//!
//! 红条件：`ToolOutput { external: true }` 改成 `TOOL_OUTPUT` → 第 8 条红；
//! `Artifact { Unknown }` 改成 `MODEL_OUTPUT` → 第 15 条与 (p) 红；
//! `UserInstruction` 改成 `TRUSTED_RUNTIME` → 第 1、2 条红。

use continuum_trust::{ArtifactProducer, ContextProvenance, ContextTrust, label};

#[test]
fn every_provenance_maps_to_the_expected_label() {
    // 1.
    assert_eq!(
        label(&ContextProvenance::user_instruction()),
        ContextTrust::UserInstruction
    );
    // 2.
    assert_eq!(
        label(&ContextProvenance::conversation_history()),
        ContextTrust::UserInstruction
    );
    // 3.
    assert_eq!(
        label(&ContextProvenance::trusted_runtime()),
        ContextTrust::TrustedRuntime
    );
    // 4.
    assert_eq!(
        label(&ContextProvenance::project_data()),
        ContextTrust::ProjectData
    );
    // 5.
    assert_eq!(
        label(&ContextProvenance::web_untrusted()),
        ContextTrust::WebUntrusted
    );
    // 6.
    assert_eq!(
        label(&ContextProvenance::provider_supplied()),
        ContextTrust::WebUntrusted
    );
    // 7.
    assert_eq!(
        label(&ContextProvenance::tool_output(false)),
        ContextTrust::ToolOutput
    );
    // 8.
    assert_eq!(
        label(&ContextProvenance::tool_output(true)),
        ContextTrust::WebUntrusted
    );
    // 9.
    assert_eq!(
        label(&ContextProvenance::memory("c1")),
        ContextTrust::Memory
    );
    // 10.
    assert_eq!(
        label(&ContextProvenance::model_output()),
        ContextTrust::ModelOutput
    );
    // 11.
    assert_eq!(
        label(&ContextProvenance::artifact(ArtifactProducer::Model)),
        ContextTrust::ModelOutput
    );
    // 12.
    assert_eq!(
        label(&ContextProvenance::artifact(ArtifactProducer::Tool)),
        ContextTrust::ToolOutput
    );
    // 13.
    assert_eq!(
        label(&ContextProvenance::artifact(ArtifactProducer::External)),
        ContextTrust::WebUntrusted
    );
    // 14.
    assert_eq!(
        label(&ContextProvenance::artifact(ArtifactProducer::Project)),
        ContextTrust::ProjectData
    );
    // 15.
    assert_eq!(
        label(&ContextProvenance::artifact(ArtifactProducer::Unknown)),
        ContextTrust::ProjectData
    );
}

#[test]
fn an_unknown_artifact_producer_defaults_to_project_data() {
    // 这里的「默认」不是 `Default` 实现：`Unknown` 是显式取值。
    // 理由是 `PROJECT_DATA` 是四枚 artifact 投影里唯一不声称产者的 data-only 标签。
    assert_eq!(
        label(&ContextProvenance::artifact(ArtifactProducer::Unknown)),
        ContextTrust::ProjectData
    );
}
