//! 本领域七枚算子的清单（设计 §4.2 的表是**唯一事实来源**）。
//!
//! 逐枚的 `id` / `input_schema` / `output_schema` / `determinism` /
//! `side_effect_class` / backend 候选**逐格取自设计 §4.2 的表**，顺序即该表的行序
//! （§4.1）；`version` 一律为 1（§4.2 末）。
//!
//! **`NonDeterministic` 与 `Pure` 同现不是笔误**（设计 §4.4 第 3 条）：`determinism`
//! 与 `side_effect_class` 是**两个正交字段**——「经模型/检索」不等于「有外部副作用」。
//! `side_effect_class` 判的是**算子自身的执行有没有改变本仓之外的状态**：本块七枚
//! 都只读、只产出本仓制品（模型调用与检索调用属这一类，网络传输与计费记在预算维度），
//! 故第 3 行 `Deterministic + Pure` 与其余六行 `NonDeterministic + Pure` 各自自洽。

use continuum_artifact::ArtifactType;
use continuum_operator::{
    BackendId, Determinism, Operator, OperatorId, OperatorVersion, SideEffectClass,
};

/// 本领域七枚算子的全集。顺序即设计 §4.2 表的行序。
/// **返回值是数组不是 `&'static [Operator]`**：`Operator` 含 `Vec` 与 `String`，
/// 不可 const 构造——`OperatorId::new` / `BackendId::new` 都不是 `const fn`，
/// 且非空 `Vec` 不是 const 操作，而七枚逐一至少有一个非空 `Vec`（第 1 枚的
/// `input_schema` 为空，其 `output_schema` 与 `backend_candidates` 仍非空）——
/// 两条实测理由见设计 §4.1.1，故只能每次构造。
/// 不变量：七枚的 id 逐枚不同、version 均为 1——由 `tests/catalog.rs` 的两条用例把守。
pub fn all_operators() -> [Operator; 7] {
    [
        // 设计 §4.2 第 1 行
        Operator {
            id: OperatorId::new("research-question"),
            version: OperatorVersion::new(1),
            input_schema: vec![],
            output_schema: vec![ArtifactType::Json],
            determinism: Determinism::NonDeterministic,
            side_effect_class: SideEffectClass::Pure,
            backend_candidates: vec![BackendId::new("primary-model")],
        },
        // 设计 §4.2 第 2 行
        Operator {
            id: OperatorId::new("research-search"),
            version: OperatorVersion::new(1),
            input_schema: vec![ArtifactType::Json],
            output_schema: vec![ArtifactType::Json],
            determinism: Determinism::NonDeterministic,
            side_effect_class: SideEffectClass::Pure,
            backend_candidates: vec![BackendId::new("retrieval-backend")],
        },
        // 设计 §4.2 第 3 行
        Operator {
            id: OperatorId::new("research-source-set"),
            version: OperatorVersion::new(1),
            input_schema: vec![ArtifactType::Json],
            output_schema: vec![ArtifactType::Json],
            determinism: Determinism::Deterministic,
            side_effect_class: SideEffectClass::Pure,
            backend_candidates: vec![BackendId::new("builtin")],
        },
        // 设计 §4.2 第 4 行
        Operator {
            id: OperatorId::new("research-evidence-extraction"),
            version: OperatorVersion::new(1),
            input_schema: vec![ArtifactType::Json],
            output_schema: vec![ArtifactType::Json],
            determinism: Determinism::NonDeterministic,
            side_effect_class: SideEffectClass::Pure,
            backend_candidates: vec![BackendId::new("primary-model")],
        },
        // 设计 §4.2 第 5 行
        Operator {
            id: OperatorId::new("research-contradiction-check"),
            version: OperatorVersion::new(1),
            input_schema: vec![ArtifactType::Json],
            output_schema: vec![ArtifactType::Json],
            determinism: Determinism::NonDeterministic,
            side_effect_class: SideEffectClass::Pure,
            backend_candidates: vec![BackendId::new("primary-model")],
        },
        // 设计 §4.2 第 6 行
        Operator {
            id: OperatorId::new("research-synthesis"),
            version: OperatorVersion::new(1),
            input_schema: vec![ArtifactType::Json],
            output_schema: vec![ArtifactType::Report],
            determinism: Determinism::NonDeterministic,
            side_effect_class: SideEffectClass::Pure,
            backend_candidates: vec![BackendId::new("primary-model")],
        },
        // 设计 §4.2 第 7 行
        Operator {
            id: OperatorId::new("research-citation-verification"),
            version: OperatorVersion::new(1),
            input_schema: vec![ArtifactType::Report, ArtifactType::Json],
            output_schema: vec![ArtifactType::Json],
            determinism: Determinism::NonDeterministic,
            side_effect_class: SideEffectClass::Pure,
            backend_candidates: vec![
                BackendId::new("primary-model"),
                BackendId::new("retrieval-backend"),
            ],
        },
    ]
}
