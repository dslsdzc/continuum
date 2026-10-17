//! 本领域九枚算子的清单（设计 §3.2）。
//!
//! 逐枚的 `id` / `input_schema` / `output_schema` / `determinism` /
//! `side_effect_class` / backend 候选**逐格取自设计 §3.2 的表**（唯一事实来源），
//! 顺序即该表的行序（§3.2 表头口径：算子名与端口类型在规范给了的地方照录，
//! `determinism` / `side_effect_class` / backend 候选三列由本设计定、表内标「定」）。
//! `version` 一律为 1（§3.2 的不变量）。

use continuum_artifact::ArtifactType;
use continuum_operator::{
    BackendId, Determinism, Operator, OperatorId, OperatorVersion, SideEffectClass,
};

/// 本领域九枚算子的全集。**顺序即设计 §3.2 表的行序**，用例按行序取。
/// 不变量：id 逐枚不同、version 均为 1。
/// **这里不是 `const`**：`Operator` 含 `OperatorId`（字段私有、`new` 非 `const fn`）
/// 与三枚 `Vec` 字段，在 crate 外构造不出常量值；本仓带 `ALL` 常量的两处
/// （`ArtifactType::ALL` / `AuditKind::ALL`）成立的前提是那些类型**无字段**。
/// 数目 9 写在**返回类型**里，使清单个数的改动**编译期可见**——这是本行要保住的那条性质。
pub fn all_operators() -> [Operator; 9] {
    [
        // 设计 §3.2 第 1 行（出处：§16 `:790-794`；§255 `:1023-1037`）。
        // §3.3 第 1 条：本行**不在《总纲》§8.3 `:1448` 的六步上**，它的来源是两处，
        // 各自独立成立——①§16 `:790-794` 与 §255 `:1023-1027` 把 L2 工作区定为代码
        // 任务的基础（《总纲》`:1443` 逐字重复了这一点）；②P5b 设计第六节把 §187 的
        // `worktree` 条目交给本块。**若不给这一枚算子，「L2 隔离工作区」在这条链上
        // 就没有任何可注册的落点**——它就只能是一条注释，而注释不是登记。
        Operator {
            id: OperatorId::new("code-workspace"),
            version: OperatorVersion::new(1),
            input_schema: vec![],
            output_schema: vec![ArtifactType::SourceTree],
            determinism: Determinism::NonDeterministic,
            side_effect_class: SideEffectClass::Idempotent,
            backend_candidates: vec![BackendId::new("worktree"), BackendId::new("overlay")],
        },
        // 设计 §3.2 第 2 行（出处：《总纲》§8.3 `:1448`；§186 `:62`）。
        Operator {
            id: OperatorId::new("code-plan"),
            version: OperatorVersion::new(1),
            input_schema: vec![ArtifactType::Json],
            output_schema: vec![ArtifactType::Json],
            determinism: Determinism::NonDeterministic,
            side_effect_class: SideEffectClass::Pure,
            backend_candidates: vec![BackendId::new("primary-model")],
        },
        // 设计 §3.2 第 3 行（出处：《总纲》§8.3 `:1448`；§186 `:64`）。
        Operator {
            id: OperatorId::new("code-decompose"),
            version: OperatorVersion::new(1),
            input_schema: vec![ArtifactType::Json],
            output_schema: vec![ArtifactType::Json],
            determinism: Determinism::NonDeterministic,
            side_effect_class: SideEffectClass::Pure,
            backend_candidates: vec![BackendId::new("primary-model")],
        },
        // 设计 §3.2 第 4 行（出处：《总纲》§8.3 `:1448`；§186 `:66-68`）。
        Operator {
            id: OperatorId::new("code-implement"),
            version: OperatorVersion::new(1),
            input_schema: vec![ArtifactType::Json],
            output_schema: vec![ArtifactType::Patch],
            determinism: Determinism::NonDeterministic,
            side_effect_class: SideEffectClass::Idempotent,
            backend_candidates: vec![
                BackendId::new("coding-agent"),
                BackendId::new("local-agent-cli"),
            ],
        },
        // 设计 §3.2 第 5 行（出处：§186 `:70`；`:85`）。
        Operator {
            id: OperatorId::new("code-spec-review"),
            version: OperatorVersion::new(1),
            input_schema: vec![ArtifactType::Patch, ArtifactType::Json],
            output_schema: vec![ArtifactType::Json],
            determinism: Determinism::NonDeterministic,
            side_effect_class: SideEffectClass::Pure,
            backend_candidates: vec![BackendId::new("independent-reviewer-model")],
        },
        // 设计 §3.2 第 6 行（出处：§186 `:72`；`:85`）。
        Operator {
            id: OperatorId::new("code-quality-review"),
            version: OperatorVersion::new(1),
            input_schema: vec![ArtifactType::Patch, ArtifactType::Json],
            output_schema: vec![ArtifactType::Json],
            determinism: Determinism::NonDeterministic,
            side_effect_class: SideEffectClass::Pure,
            backend_candidates: vec![BackendId::new("independent-reviewer-model")],
        },
        // 设计 §3.2 第 7 行（出处：§186 `:74`；§259 `:1109-1119`）。
        Operator {
            id: OperatorId::new("code-test-run"),
            version: OperatorVersion::new(1),
            input_schema: vec![ArtifactType::SourceTree, ArtifactType::Patch],
            output_schema: vec![ArtifactType::TestResult],
            determinism: Determinism::Deterministic,
            side_effect_class: SideEffectClass::Idempotent,
            backend_candidates: vec![
                BackendId::new("cargo-test"),
                BackendId::new("local-test-runner"),
            ],
        },
        // 设计 §3.2 第 8 行（出处：§193 `:361-374`；P5b 设计第六节的 `fuzz` 行）。
        // §3.3 第 5 条：本行与第 7 行**分作两枚，不合成一枚多 backend 的算子**——
        // 分的原因是 `determinism`（Fuzzing 与 Property-based Testing 以熵源生成
        // 输入，同一棵树两次运行可能给出不同的覆盖与不同的发现）。
        // **若合成一枚并把 fuzz 放进它的候选里，那一枚的 `Deterministic` 声明当场
        // 为假**——本条声明是复用的前提，故不合成。
        Operator {
            id: OperatorId::new("code-fuzz-run"),
            version: OperatorVersion::new(1),
            input_schema: vec![ArtifactType::SourceTree, ArtifactType::Patch],
            output_schema: vec![ArtifactType::TestResult],
            determinism: Determinism::NonDeterministic,
            side_effect_class: SideEffectClass::Idempotent,
            backend_candidates: vec![
                BackendId::new("cargo-fuzz"),
                BackendId::new("proptest"),
            ],
        },
        // 设计 §3.2 第 9 行（出处：§186 `:74`；§262 `:1173-1179`）。
        // §3.3 第 11 条：本行是「§262 第 1 级的确定性判定器」，**不是 P5a 的判定**
        // （§262 的五级序里第 1 级是 `deterministic checker`）。比较器的实现在本块
        // 的交付物里不在本 task（本 task 只声明算子）；**级别序与候选过滤属 P5a**
        // （其设计 §7）。
        Operator {
            id: OperatorId::new("code-verify"),
            version: OperatorVersion::new(1),
            input_schema: vec![ArtifactType::TestResult, ArtifactType::Patch],
            output_schema: vec![ArtifactType::Json],
            determinism: Determinism::Deterministic,
            side_effect_class: SideEffectClass::Pure,
            backend_candidates: vec![BackendId::new("builtin")],
        },
    ]
}
