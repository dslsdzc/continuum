//! §10 的具名入口 `bind_media_methods`：把四条 `video/` 方法的 `realized_by`
//! 填进方法库——**这是本块调用 `MethodRegistry::bind` 的唯一入口**。
//!
//! # 实现只调 `bind`
//!
//! **不 `register` 方法条目、不构造 `MethodEntry`、不给 §187 补条目**（设计 §10）：
//! `video/` 的四条 id 由 P5b 的 `MethodRegistry::seeded()` 给出，本块只把这四条
//! 「每一条已 seed 的 id」逐条 `bind` 一次（P5b 设计第六节：四块**不得**自造
//! `MethodEntry`）。
//!
//! # §33 与 §187 的 `video/` 不是同一张表
//!
//! §33 的七枚预处理算子（`proxy-build` / `audio-analysis` / `metadata-analysis` 等）
//! **不在任何方法的 `realized_by` 里，这不是漏 bind**：§187 的 `video/` 只有四条目录名
//! （方法名），§33 是链上的步骤——两张表各管一件事，前者是「这一类任务怎样做得可靠」
//! 的索引，后者是剪辑链的环节序。故本函数只填四条，不替 §33 的算子发明目录
//! （设计 §10；给 §187 补规范无来源的方法名会动到 P5b 的裁决）。
//!
//! # 错误类型直接透出
//!
//! 本函数的失败是「方法库拒绝了这次 `bind`」（[`MethodError::NotFound`]），
//! **不并进 `MediaError`**：判定、收件人与处置都不同，故直接透出——这与 §5.1 把
//! `OperatorError::Duplicate` 包进 `MediaError::Registry` 是同一条判据的两面：
//! **判定是同一件事则合并，是两件事则分开**（设计 §10）。
//!
//! `realized_by` 是文本弱引用（P5b 设计 §3.2 末、§4.3）：本 crate 无法在编译期核对
//! 这些串与 `all_operators()` 的 id 一致——那条核对落在消费块
//! （`tests/methods.rs` 的第 (s) 条，本块同时持有 `all_operators()` 与 `MethodRegistry`）。

use continuum_method::{MethodError, MethodId, MethodRegistry};

/// 把四条 `video/` 方法的 `realized_by` 填进方法库（P5b 设计第六节：每块对其领域目录里的**每一条已 seed 的 id**
/// 调用一次 `bind(id, &[...])`）。`video/` 四条逐条非空。
/// 这是本块调用 `MethodRegistry::bind` 的**唯一入口**。
///
/// 四条映射**逐条照设计 §10 的表**（`realized_by` 是本块的算子 id 文本）：
/// 每一行的理由见各调用的行内注释。按 §10 表的行序逐条 `bind`；任一条失败即
/// `?` 短路透出（空表上首条失败即 `shot-analysis`——`tests/methods.rs` 的
/// `binding_an_unseeded_id_returns_not_found` 钉住这一侧）。
pub fn bind_media_methods(registry: &mut MethodRegistry) -> Result<(), MethodError> {
    // §10 表第 1 行：§33 的 shot detection 与 face/object indexing、visual embedding
    // 都是「对镜头内容作出判定」的步骤。
    registry.bind(
        &MethodId::new("shot-analysis"),
        &["shot-detect", "vision-analysis"],
    )?;

    // §10 表第 2 行：§32 的 Narrative Planning，与 §187 的 narrative-plan 同名同物。
    registry.bind(&MethodId::new("narrative-plan"), &["narrative-plan"])?;

    // §10 表第 3 行：§32 的 Timeline Artifact；产物是 `Timeline`。
    registry.bind(&MethodId::new("timeline-compose"), &["timeline-compose"])?;

    // §10 表第 4 行：§32 链尾两步（verify-deterministic / verify-multimodal）
    // 是「审看渲染结果」。
    registry.bind(
        &MethodId::new("render-review"),
        &["verify-deterministic", "verify-multimodal"],
    )?;

    Ok(())
}
