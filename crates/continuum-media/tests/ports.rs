//! §5.3 的 17 行端口边表的守卫（设计 §12.2 的 (t)）。
//!
//! **语料手写**：设计 §5.3 的 17 行边表逐行照录为 `DECLARED_EDGES`
//!（下游算子 ← 输入的来源（交集中的型）），逐行断言「下游 `input_schema` 的
//! **每一个型**都能在该行列出的上游算子里找到，且所引的那一枚的 `output_schema`
//! 含该型」。
//!
//! **断言的对象是「交集中的型」，不是「两侧相等」**（设计 §5.3 改判后的口径：
//! 多数相邻算子的 schema 只相交、不相等——如 `media-import` 出 `[Video, Audio]`，
//! 而 `proxy-build` 只入 `[Video]`）。
//!
//! **这张表的射程（据实记，不许读成全称）**：它钉的是**设计声明的那 17 行关系**，
//! **不是「所有可能的连接都合法」**；§5.3 末已写「本表不是全图」——两枚生成算子的
//! 产物回流进 `timeline-compose` 之类的**其他**边属装配方的图构造
//!（本块只注册算子、不建执行路径，设计 §1.2）。
//!
//! **逐行、不抽代表**：17 行全部进断言（表驱动），每一行都有会红的改法——
//! 用例内两条点名的红条件与一般机理逐处写明。

use continuum_artifact::ArtifactType;
use continuum_media::all_operators;
use continuum_operator::Operator;

/// 设计 §5.3 的 17 行边表，**手写照录**：行序即设计 §5.2 表的行序；
/// 每行 = (下游算子 id, 该行声明的输入来源列表)；来源列表的元素 =
/// (上游算子 id, 该上游的 `output_schema` 与下游 `input_schema` 交集中的型)。
///
/// 第 1 行 `media-import` 的来源列表**为空**：它的输入是外部素材，不是上游制品
///（§5.2 第 1 行）——「零个输入」在类型里可表达。
const DECLARED_EDGES: [(&str, &[(&str, ArtifactType)]); 17] = [
    ("media-import", &[]),
    ("proxy-build", &[("media-import", ArtifactType::Video)]),
    ("shot-detect", &[("media-import", ArtifactType::Video)]),
    ("transcribe", &[("media-import", ArtifactType::Audio)]),
    ("audio-analysis", &[("media-import", ArtifactType::Audio)]),
    (
        "vision-analysis",
        &[
            ("media-import", ArtifactType::Video),
            ("shot-detect", ArtifactType::ShotSet),
        ],
    ),
    (
        "metadata-analysis",
        &[
            ("media-import", ArtifactType::Video),
            ("media-import", ArtifactType::Audio),
        ],
    ),
    (
        "narrative-plan",
        &[
            ("shot-detect", ArtifactType::ShotSet),
            ("transcribe", ArtifactType::Transcript),
            ("vision-analysis", ArtifactType::Json),
        ],
    ),
    (
        "timeline-compose",
        &[
            ("narrative-plan", ArtifactType::Json),
            ("shot-detect", ArtifactType::ShotSet),
            ("media-import", ArtifactType::Audio),
        ],
    ),
    (
        "effects-subtitle",
        &[("timeline-compose", ArtifactType::Timeline)],
    ),
    ("render", &[("effects-subtitle", ArtifactType::Timeline)]),
    (
        "generate-broll",
        &[("timeline-compose", ArtifactType::Timeline)],
    ),
    (
        "generate-voice",
        &[("transcribe", ArtifactType::Transcript)],
    ),
    ("generate-music", &[("narrative-plan", ArtifactType::Json)]),
    (
        "frame-interpolation",
        &[("media-import", ArtifactType::Video)],
    ),
    (
        "verify-deterministic",
        &[
            ("render", ArtifactType::Render),
            ("timeline-compose", ArtifactType::Timeline),
        ],
    ),
    (
        "verify-multimodal",
        &[
            ("render", ArtifactType::Render),
            ("transcribe", ArtifactType::Transcript),
            ("effects-subtitle", ArtifactType::SubtitleTrack),
        ],
    ),
];

/// 在清单里按 id 线性查找；**找不到即 `panic!`**——不许写成「找不到就跳过」：
/// 静默跳过会让「某行的上游 id 被改坏」这一档在本用例里变绿。
fn operator_by_id<'a>(operators: &'a [Operator], id: &str) -> &'a Operator {
    operators
        .iter()
        .find(|operator| operator.id.as_str() == id)
        .unwrap_or_else(|| panic!("all_operators() 里应有 {id}"))
}

/// 第 (t) 条：§5.3 的 17 行边表**逐行**成立（设计 §12.2 的 (t)）。
///
/// 断言分两半，**每一行都被两半覆盖**：
/// - 半一（声明侧）：该行声明的每一枚 `(上游, 型)`——该型必须出现在下游的
///   `input_schema` 里（它确是「交集中的型」，不是凭空写的），且必须出现在
///   该上游的 `output_schema` 里（所引的那一枚确实供得上）；
/// - 半二（覆盖侧）：下游 `input_schema` 的每一个型都必须在声明的来源里出现
///   （有孤儿型即红）。
///
/// **红条件（取反档，点名的两条）**：
/// 1. 把 `transcribe` 的 `input_schema` 从 `[Audio]` 改成 `[Video]` ⇒ 第 4 行红
///    （声明的交集中的型 `Audio` 不再在下游的输入里）；
/// 2. 把 `verify-multimodal` 的 `SubtitleTrack` 去掉 ⇒ 第 17 行的
///    `effects-subtitle` 那一支红（同上）。
///
/// **一般机理（其余行的红路径）**：任一行里任一枚 `(上游, 型)` 被改坏——
/// 上游 id 改错 ⇒ 找不到即 `panic!`；型改错 ⇒ 半一的两条断言之一红；
/// 任一行的 `input_schema` 多出一枚声明外的型 ⇒ 半二红；声明里删掉一枚 ⇒ 半二红。
/// 故 17 行**每一行**都有会红的改法；表的行数自身由返回类型 `[..; 17]` 在编译期守护。
#[test]
fn every_downstream_input_type_has_a_declared_upstream_source() {
    let operators = all_operators();

    for (downstream_id, sources) in DECLARED_EDGES {
        let downstream = operator_by_id(&operators, downstream_id);

        // 半一：声明侧。
        for &(upstream_id, ty) in sources {
            let upstream = operator_by_id(&operators, upstream_id);
            assert!(
                downstream.input_schema.contains(&ty),
                "{downstream_id} 的行里声明 {upstream_id} 供 {ty:?}，但 {ty:?} 不在 {downstream_id} 的 input_schema 里"
            );
            assert!(
                upstream.output_schema.contains(&ty),
                "{downstream_id} 的行里声明上游 {upstream_id} 供 {ty:?}，但 {upstream_id} 的 output_schema 不含该型"
            );
        }

        // 半二：覆盖侧。
        for ty in &downstream.input_schema {
            assert!(
                sources.iter().any(|&(_, declared)| declared == *ty),
                "{downstream_id} 的 input_schema 含 {ty:?}，但该行声明的来源里没有任何一枚供上它"
            );
        }
    }
}
