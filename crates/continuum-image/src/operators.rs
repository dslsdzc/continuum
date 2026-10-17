//! 五枚图像域算子（设计 §3.2）与 §343 的批次边界（设计 §9.2、§3.1）。
//!
//! 逐列取值一律照设计 §3.2 的表（本文件不重抄那张表，只留逐行出处）；表中
//! `determinism` / `side_effect_class` / backend 候选三列在规范未给处**由设计定**，
//! 表里逐格以「定」字标出。

use continuum_artifact::ArtifactType;
use continuum_operator::{
    BackendId, Determinism, Operator, OperatorId, OperatorVersion, SideEffectClass,
};

/// 本领域五枚算子的全集。**顺序即设计 §3.2 表的行序**，用例按 id 取行（不按下标）。
/// 不变量：id 逐枚不同、version 均为 1。
/// **这里不是 `const`**，两条理由各自独立成立（§3.1）：
/// （1）`OperatorId(String)` 的字段私有、`OperatorId::new` 不是 `const fn`；
/// （2）`Operator` 另含三枚 `Vec` 字段（`input_schema` / `output_schema` / `backend_candidates`）。
/// 本仓带 `ALL` 常量的既有做法成立的前提是**那些类型无字段**——援引一条既有约定时
/// 要连它的成立条件一起搬。
/// 数目 5 改由**返回类型**承载，故清单个数的改动**编译期可见**（写少一枚而不改返回类型，直接编不过）。
///
/// 端口用到的型都是**既有枚举里的型**（`Image` 由 P5e 落地、`Blob` / `Json` 是 P1 既有），
/// 故本块**不向 P5e 报出任何变体**（设计 §3.4 末）。**「只用到两枚型」那一句与表相抵**
/// （表第 5 行的输出是 `[Json]`，共三枚）——处置见 `tests/operators.rs` 的
/// `the_port_types_in_use_are_exactly_the_three_types_of_the_table` 与 Task 2 报告。
pub fn all_operators() -> [Operator; 5] {
    [
        // 第 1 行：`generate-image`。出处：§323 `:2316-2331`；§162 `:596-612`；§161 `:581`。
        // `input_schema` 是 `[Image]` 而不是 `[]`：`reference_artifacts` 是**制品**，
        // 「从零生成」的那一次只是没有接参考制品（设计 §3.2 第 1 条）。
        // `side_effect_class = Pure` 是 2026-10-10 的订正（此前取 `NonIdempotent`）：
        // 它的产物是**一个值**、它**不改变本仓之外的状态**；消耗的算力记在预算维度，
        // 不是副作用（设计 §3.2 第 6 条）。
        Operator {
            id: OperatorId::new("generate-image"),
            version: OperatorVersion::new(1),
            input_schema: vec![ArtifactType::Image],
            output_schema: vec![ArtifactType::Image],
            determinism: Determinism::NonDeterministic,
            side_effect_class: SideEffectClass::Pure,
            backend_candidates: vec![BackendId::new("image-gen-backend")],
        },
        // 第 2 行：`detect-edit-region`。出处：§164 `:684`（Object / Region Detection）；
        // §325 `:2360-2366`；§166 `:745-776`。
        // `output_schema` 是 `[Blob]` 而不是 `[Image]`：掩码是**单通道位图**，不是彩色图像
        //——记成 `Image` 会让读它的人按图像的色彩空间 / Alpha 约定去读（设计 §3.2 第 2 条）。
        Operator {
            id: OperatorId::new("detect-edit-region"),
            version: OperatorVersion::new(1),
            input_schema: vec![ArtifactType::Image],
            output_schema: vec![ArtifactType::Blob],
            determinism: Determinism::NonDeterministic,
            side_effect_class: SideEffectClass::Pure,
            backend_candidates: vec![BackendId::new("object-detector-backend")],
        },
        // 第 3 行：`local-generative-edit`。出处：§164 `:688`（Local Generative Edit）；
        // §165 `:724`（Generative Inpainting）；§324 `:2335-2345`；§163 `:626-665`。
        // `side_effect_class = Pure` 同第 1 行（2026-10-10 订正；理由见设计 §3.2 第 6 条）。
        Operator {
            id: OperatorId::new("local-generative-edit"),
            version: OperatorVersion::new(1),
            input_schema: vec![ArtifactType::Image, ArtifactType::Blob],
            output_schema: vec![ArtifactType::Image],
            determinism: Determinism::NonDeterministic,
            side_effect_class: SideEffectClass::Pure,
            backend_candidates: vec![BackendId::new("inpainting-backend")],
        },
        // 第 4 行：`hard-composite`。出处：§165 `:728`（Hard Composite）；§326 `:2378-2388`；
        // §164 `:690`（Composite）。
        // `side_effect_class = Idempotent` 而**不是** `Pure`：它的执行体**自己有落盘动作**
        //（在本地存储里落下一整张图），故它**改变本仓之外的状态**，而重复执行得到同一状态。
        // **「它产生一个大制品 / 重算一次要落一个新 Artifact 与新事件」那句已废**——
        //「消耗资源」与「改变状态」是两件事（设计 §3.2 第 7 条）。
        Operator {
            id: OperatorId::new("hard-composite"),
            version: OperatorVersion::new(1),
            input_schema: vec![ArtifactType::Image, ArtifactType::Blob],
            output_schema: vec![ArtifactType::Image],
            determinism: Determinism::Deterministic,
            side_effect_class: SideEffectClass::Idempotent,
            backend_candidates: vec![BackendId::new("builtin")],
        },
        // 第 5 行：`verify-outside-mask`。出处：§168 `:818-835`；§326 `:2372`；《工程》`:512`。
        // 它只承担 §168 的**三项确定性检查**——语义那一项不注册（判据不存在，设计 §5.5）。
        Operator {
            id: OperatorId::new("verify-outside-mask"),
            version: OperatorVersion::new(1),
            input_schema: vec![ArtifactType::Image, ArtifactType::Blob],
            output_schema: vec![ArtifactType::Json],
            determinism: Determinism::Deterministic,
            side_effect_class: SideEffectClass::Pure,
            backend_candidates: vec![BackendId::new("builtin")],
        },
    ]
}

/// §343 的 v0.1 批：本域在 v0.1 内要注册的那些算子的 id（设计 §9.2）。
/// **这里的 `const` 成立**——元素型是 `&str`，与 `all_operators()` 那一处不同。
/// 判据：每一项都能在 `all_operators()` 里找到；且补集恰为其余四枚（**逐枚断言，不是只看长度**）。
pub const V01_OPERATOR_IDS: [&str; 1] = ["generate-image"];
