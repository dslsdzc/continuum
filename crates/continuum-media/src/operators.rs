//! 本领域 17 枚算子的清单与注册入口（设计 §5.1、§5.2）。
//!
//! 逐枚的 `id` / `input_schema` / `output_schema` / `determinism` /
//! `side_effect_class` / backend 候选**逐格取自设计 §5.2 的表**（唯一事实来源），
//! 顺序即该表的行序（§5.1）。`version` 一律为 1（§5.1 的不变量）。

use continuum_artifact::ArtifactType;
use continuum_operator::{
    BackendId, Determinism, Operator, OperatorId, OperatorRegistry, OperatorVersion,
    SideEffectClass,
};

use crate::error::MediaError;

/// 本领域 17 枚算子的全集。顺序即设计 §5.2 表的行序。
/// 不变量：id 逐枚不同、version 均为 1。
/// **这里不是 `const`**，两条理由各自独立成立：本仓带 `ALL` 常量的既有做法
/// （`ArtifactType::ALL`、`PrivacyClass::ALL`）成立的前提是那些类型**无字段**；且
/// `Operator` 含 `OperatorId`（字段私有、`new` 不是 `const fn`）与三枚 `Vec` 字段，
/// 故在 crate 外**构造不出 `Operator` 的常量值**。
/// 数目 17 写在**返回类型**里，使清单个数的改动编译期可见。
///
/// 数组的第 16、17 项取两枚具名私有 helper 的返回值——设计 §5.5 的两条
/// 级别声明挂在它们的文档注释上（详见两处 helper）。
pub fn all_operators() -> [Operator; 17] {
    [
        // 设计 §5.2 第 1 行
        Operator {
            id: OperatorId::new("media-import"),
            version: OperatorVersion::new(1),
            input_schema: vec![],
            output_schema: vec![ArtifactType::Video, ArtifactType::Audio],
            determinism: Determinism::Deterministic,
            side_effect_class: SideEffectClass::Idempotent,
            backend_candidates: vec![BackendId::new("local-fs")],
        },
        // 设计 §5.2 第 2 行
        Operator {
            id: OperatorId::new("proxy-build"),
            version: OperatorVersion::new(1),
            input_schema: vec![ArtifactType::Video],
            output_schema: vec![ArtifactType::Video],
            determinism: Determinism::Deterministic,
            side_effect_class: SideEffectClass::Idempotent,
            backend_candidates: vec![BackendId::new("ffmpeg")],
        },
        // 设计 §5.2 第 3 行
        Operator {
            id: OperatorId::new("shot-detect"),
            version: OperatorVersion::new(1),
            input_schema: vec![ArtifactType::Video],
            output_schema: vec![ArtifactType::ShotSet],
            determinism: Determinism::Deterministic,
            side_effect_class: SideEffectClass::Pure,
            backend_candidates: vec![
                BackendId::new("ffmpeg-scene-detect"),
                BackendId::new("pyscenedetect"),
            ],
        },
        // 设计 §5.2 第 4 行
        Operator {
            id: OperatorId::new("transcribe"),
            version: OperatorVersion::new(1),
            input_schema: vec![ArtifactType::Audio],
            output_schema: vec![ArtifactType::Transcript],
            determinism: Determinism::NonDeterministic,
            side_effect_class: SideEffectClass::Pure,
            backend_candidates: vec![
                BackendId::new("local-whisper"),
                BackendId::new("cloud-model"),
                BackendId::new("specialized-asr"),
            ],
        },
        // 设计 §5.2 第 5 行
        Operator {
            id: OperatorId::new("audio-analysis"),
            version: OperatorVersion::new(1),
            input_schema: vec![ArtifactType::Audio],
            output_schema: vec![ArtifactType::Json],
            determinism: Determinism::Deterministic,
            side_effect_class: SideEffectClass::Pure,
            backend_candidates: vec![BackendId::new("librosa"), BackendId::new("essentia")],
        },
        // 设计 §5.2 第 6 行
        Operator {
            id: OperatorId::new("vision-analysis"),
            version: OperatorVersion::new(1),
            input_schema: vec![ArtifactType::Video, ArtifactType::ShotSet],
            output_schema: vec![ArtifactType::Json],
            determinism: Determinism::NonDeterministic,
            side_effect_class: SideEffectClass::Pure,
            backend_candidates: vec![
                BackendId::new("local-vision-model"),
                BackendId::new("cloud-vision-model"),
            ],
        },
        // 设计 §5.2 第 7 行
        Operator {
            id: OperatorId::new("metadata-analysis"),
            version: OperatorVersion::new(1),
            input_schema: vec![ArtifactType::Video, ArtifactType::Audio],
            output_schema: vec![ArtifactType::Json],
            determinism: Determinism::Deterministic,
            side_effect_class: SideEffectClass::Pure,
            backend_candidates: vec![BackendId::new("ffprobe")],
        },
        // 设计 §5.2 第 8 行
        Operator {
            id: OperatorId::new("narrative-plan"),
            version: OperatorVersion::new(1),
            input_schema: vec![
                ArtifactType::ShotSet,
                ArtifactType::Transcript,
                ArtifactType::Json,
            ],
            output_schema: vec![ArtifactType::Json],
            determinism: Determinism::NonDeterministic,
            side_effect_class: SideEffectClass::Pure,
            backend_candidates: vec![BackendId::new("primary-model")],
        },
        // 设计 §5.2 第 9 行
        Operator {
            id: OperatorId::new("timeline-compose"),
            version: OperatorVersion::new(1),
            input_schema: vec![
                ArtifactType::Json,
                ArtifactType::ShotSet,
                ArtifactType::Audio,
            ],
            output_schema: vec![ArtifactType::Timeline],
            determinism: Determinism::Deterministic,
            side_effect_class: SideEffectClass::Pure,
            backend_candidates: vec![BackendId::new("builtin")],
        },
        // 设计 §5.2 第 10 行
        Operator {
            id: OperatorId::new("effects-subtitle"),
            version: OperatorVersion::new(1),
            input_schema: vec![ArtifactType::Timeline],
            output_schema: vec![ArtifactType::Timeline, ArtifactType::SubtitleTrack],
            determinism: Determinism::Deterministic,
            side_effect_class: SideEffectClass::Pure,
            backend_candidates: vec![BackendId::new("ffmpeg-filtergraph")],
        },
        // 设计 §5.2 第 11 行
        Operator {
            id: OperatorId::new("render"),
            version: OperatorVersion::new(1),
            input_schema: vec![ArtifactType::Timeline],
            output_schema: vec![ArtifactType::Render],
            determinism: Determinism::Deterministic,
            side_effect_class: SideEffectClass::Idempotent,
            backend_candidates: vec![BackendId::new("ffmpeg")],
        },
        // 设计 §5.2 第 12 行
        Operator {
            id: OperatorId::new("generate-broll"),
            version: OperatorVersion::new(1),
            input_schema: vec![ArtifactType::Timeline],
            output_schema: vec![ArtifactType::Video],
            determinism: Determinism::NonDeterministic,
            side_effect_class: SideEffectClass::Pure,
            backend_candidates: vec![BackendId::new("video-gen-backend")],
        },
        // 设计 §5.2 第 13 行
        Operator {
            id: OperatorId::new("generate-voice"),
            version: OperatorVersion::new(1),
            input_schema: vec![ArtifactType::Transcript],
            output_schema: vec![ArtifactType::Audio],
            determinism: Determinism::NonDeterministic,
            side_effect_class: SideEffectClass::Pure,
            backend_candidates: vec![BackendId::new("tts-backend")],
        },
        // 设计 §5.2 第 14 行
        Operator {
            id: OperatorId::new("generate-music"),
            version: OperatorVersion::new(1),
            input_schema: vec![ArtifactType::Json],
            output_schema: vec![ArtifactType::Audio],
            determinism: Determinism::NonDeterministic,
            side_effect_class: SideEffectClass::Pure,
            backend_candidates: vec![BackendId::new("music-gen-backend")],
        },
        // 设计 §5.2 第 15 行
        Operator {
            id: OperatorId::new("frame-interpolation"),
            version: OperatorVersion::new(1),
            input_schema: vec![ArtifactType::Video],
            output_schema: vec![ArtifactType::Video],
            determinism: Determinism::NonDeterministic,
            side_effect_class: SideEffectClass::Pure,
            backend_candidates: vec![BackendId::new("interpolation-backend")],
        },
        verify_deterministic_operator(),
        verify_multimodal_operator(),
    ]
}

/// §5.5 的声明：`verify-deterministic` 在 §262 的序里是第 1 级（`DeterministicChecker`）。
/// **这是本设计的声明**（§32 只说这两步在链尾，没说级别）；
/// **「这一对应由谁持有」未定**（设计 §13 第 5 条 (ii)：装配方以值构造 `AvailableVerifier`，
/// `level` 取本设计声明的那一级）。
///
/// 载体为什么是具名 helper：`all_operators()` 是函数，这两枚只能是**数组里的表达式**；
/// `///` 挂在数组元素上不生成文档，rustc 1.95.0 实测为 `unused_doc_comments`
/// （warn-by-default），与 0 warning 门相抵。**具名 helper 是 item，声明挂得住、
/// 可编译、无 warning。**
/// **据实记**：`Operator` 的七字段里没有级别，故这是**纯数据声明、无运行期照片**；
/// 级别序、候选过滤与独立性判定属 P5a（设计 §5.5 的归属表）。
fn verify_deterministic_operator() -> Operator {
    // 设计 §5.2 第 16 行
    Operator {
        id: OperatorId::new("verify-deterministic"),
        version: OperatorVersion::new(1),
        input_schema: vec![ArtifactType::Render, ArtifactType::Timeline],
        output_schema: vec![ArtifactType::Json],
        determinism: Determinism::Deterministic,
        side_effect_class: SideEffectClass::Pure,
        backend_candidates: vec![BackendId::new("builtin")],
    }
}

/// §5.5 的声明：`verify-multimodal` 在 §262 的序里是第 3 级（`IndependentModelVerifier`）。其余同上。
fn verify_multimodal_operator() -> Operator {
    // 设计 §5.2 第 17 行
    Operator {
        id: OperatorId::new("verify-multimodal"),
        version: OperatorVersion::new(1),
        input_schema: vec![
            ArtifactType::Render,
            ArtifactType::Transcript,
            ArtifactType::SubtitleTrack,
        ],
        output_schema: vec![ArtifactType::Json],
        determinism: Determinism::NonDeterministic,
        side_effect_class: SideEffectClass::Pure,
        backend_candidates: vec![BackendId::new("independent-model")],
    }
}

/// 把给定的一批算子注册进给定注册表（`&all_operators()` 是常规实参）。
/// 一枚媒体算子与既有算子同 (id, version) 是注册期的错误，**不静默跳过**。
/// **参数带一批算子**（不是只吃 `all_operators()`）：§5.4 那条规则要能对**任意** `Operator` 触发。
pub fn register_media_operators(
    registry: &mut OperatorRegistry,
    operators: &[Operator],
) -> Result<(), MediaError> {
    // 本 task 只是「逐枚 `register`、任一失败即返回」；「先核后写」的注册期规则
    //（§5.4）由后续 task 落地。
    for operator in operators {
        registry.register(operator.clone())?;
    }
    Ok(())
}
