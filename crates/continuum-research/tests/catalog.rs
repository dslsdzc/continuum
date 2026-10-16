//! 七枚研究算子（设计 §4.2）的清单与枚举守卫：§9.1 的两条 + §9.2 的 (a)(b)(j)。
//!
//! 本文件的全部语料（七个 id 串与七行期望表）**手写**，逐位/逐格取自设计 §4.2：
//! 若把期望值写成按 `all_operators()` 自己遍历再自比，测的是 Rust 的相等性——
//! 那是恒真的假照片（设计 §9.2 末的「一条预防」逐条点名 (a)(b)）。

use continuum_artifact::ArtifactType;
use continuum_operator::{BackendId, Determinism, OperatorVersion, SideEffectClass};
use continuum_research::all_operators;

/// §9.2 第 (a) 条：七枚的 id 按设计 §4.2 的行序**逐位**相等。
///
/// **顺序是这条用例的一半**：若只断集合相等，「调换两行」（如把第 1、2 枚
/// 交换）会成为等价变异体而无人发现。
/// 红条件（**取反档**）：改 `src/operators.rs` 中第 3 枚的 id 字面量即
/// `OperatorId::new("research-source-set")` 那一处的串（七枚的字面量两两不同，
/// 故锚点唯一）⇒ 本用例的第 3 位红。
/// 红条件（**取反档**）：把第 1、2 枚交换顺序（锚点：`src/operators.rs`
/// 里那两枚的**行序**）⇒ 逐位相等在第 1、2 位红。
#[test]
fn the_seven_ids_are_written_out_by_hand() {
    // 设计 §4.2 表的行序，**手写照录**——对照物是设计原文的表，不是 `all_operators()` 自身。
    const DESIGNED_IDS: [&str; 7] = [
        "research-question",
        "research-search",
        "research-source-set",
        "research-evidence-extraction",
        "research-contradiction-check",
        "research-synthesis",
        "research-citation-verification",
    ];

    let operators = all_operators();
    let got: Vec<&str> = operators.iter().map(|op| op.id.as_str()).collect();
    assert_eq!(
        got, DESIGNED_IDS,
        "all_operators() 的 id 次序应为设计 §4.2 表的行序"
    );
}

/// §9.2 第 (b) 条：七行**逐格**断言——`input_schema` / `output_schema` /
/// `determinism` / `side_effect_class` / `backend_candidates` 与 `version`。
///
/// **七行逐行，不是抽样**；期望表手写（设计 §4.2 逐格照录），不由
/// `all_operators()` 自身推出。
/// 红条件（**取反档**）：改第 5 枚 `research-contradiction-check` 的
/// `side_effect_class` 由 `Pure` 改成 `Idempotent`（锚点：**第 5 枚那个结构体
/// 字面量里**的 `side_effect_class:` 行——`side_effect_class: SideEffectClass::Pure`
/// 这个字段行在七枚里出现七次，故定位靠它的毗邻串
/// `OperatorId::new("research-contradiction-check")`，不靠这一行本身）⇒ 第 5 行红。
/// 红条件（**取反档**）：改第 3 行的 `determinism` 由 `Deterministic` 改成
/// `NonDeterministic` ⇒ 第 3 行红，同时 Task 5 的 (k) 也红（两条独立红点，
/// 设计 §9.3 第 1 行）。
/// **`backend_candidates` 那一格逐元素断言**：`["builtin"]` 与 `["Builtin"]`
/// 在 `BackendId` 上是两个值（`definition.rs:66`），故这一格不是等价变异区。
#[test]
fn the_seven_rows_are_asserted_cell_by_cell() {
    // 设计 §4.2 的七行，逐格手写照录：
    // (id, input_schema, output_schema, determinism, side_effect_class, backend_candidates, version)
    let expected: [(
        &str,
        Vec<ArtifactType>,
        Vec<ArtifactType>,
        Determinism,
        SideEffectClass,
        Vec<BackendId>,
        OperatorVersion,
    ); 7] = [
        (
            "research-question",
            vec![],
            vec![ArtifactType::Json],
            Determinism::NonDeterministic,
            SideEffectClass::Pure,
            vec![BackendId::new("primary-model")],
            OperatorVersion::new(1),
        ),
        (
            "research-search",
            vec![ArtifactType::Json],
            vec![ArtifactType::Json],
            Determinism::NonDeterministic,
            SideEffectClass::Pure,
            vec![BackendId::new("retrieval-backend")],
            OperatorVersion::new(1),
        ),
        (
            "research-source-set",
            vec![ArtifactType::Json],
            vec![ArtifactType::Json],
            Determinism::Deterministic,
            SideEffectClass::Pure,
            vec![BackendId::new("builtin")],
            OperatorVersion::new(1),
        ),
        (
            "research-evidence-extraction",
            vec![ArtifactType::Json],
            vec![ArtifactType::Json],
            Determinism::NonDeterministic,
            SideEffectClass::Pure,
            vec![BackendId::new("primary-model")],
            OperatorVersion::new(1),
        ),
        (
            "research-contradiction-check",
            vec![ArtifactType::Json],
            vec![ArtifactType::Json],
            Determinism::NonDeterministic,
            SideEffectClass::Pure,
            vec![BackendId::new("primary-model")],
            OperatorVersion::new(1),
        ),
        (
            "research-synthesis",
            vec![ArtifactType::Json],
            vec![ArtifactType::Report],
            Determinism::NonDeterministic,
            SideEffectClass::Pure,
            vec![BackendId::new("primary-model")],
            OperatorVersion::new(1),
        ),
        (
            "research-citation-verification",
            vec![ArtifactType::Report, ArtifactType::Json],
            vec![ArtifactType::Json],
            Determinism::NonDeterministic,
            SideEffectClass::Pure,
            vec![
                BackendId::new("primary-model"),
                BackendId::new("retrieval-backend"),
            ],
            OperatorVersion::new(1),
        ),
    ];

    let operators = all_operators();
    for (i, (op, want)) in operators.iter().zip(expected.iter()).enumerate() {
        let row = i + 1;
        let (id, input, output, determinism, side_effect_class, backends, version) = want;
        assert_eq!(op.id.as_str(), *id, "第 {row} 行 id");
        assert_eq!(&op.input_schema, input, "第 {row} 行 input_schema");
        assert_eq!(&op.output_schema, output, "第 {row} 行 output_schema");
        assert_eq!(op.determinism, *determinism, "第 {row} 行 determinism");
        assert_eq!(
            op.side_effect_class, *side_effect_class,
            "第 {row} 行 side_effect_class"
        );
        assert_eq!(
            &op.backend_candidates, backends,
            "第 {row} 行 backend_candidates（逐元素）"
        );
        assert_eq!(op.version, *version, "第 {row} 行 version");
    }
}

/// §9.2 第 (j) 条与设计 §6.2 第 1 条：`research-source-set` 的
/// `backend_candidates` **恰等于** `[BackendId::new("builtin")]`（**逐元素**，
/// 不是「非空」），且七枚里 `determinism == Deterministic` 的**恰一枚**、
/// 其 id 是 `research-source-set`。
///
/// **本条的作用域（照实写明）**：它钉的是**本块的声明**，**不是一条被强制的
/// 注册期规则**——`register_research_operators` 只查重，将来第二枚
/// `Deterministic` 算子带外部后端候选出现时**今天没有任何东西会拦它**
/// （设计 §6.2 第 2 条：本块没有 P5e 式的注册期名单检查）。
/// 红条件（**放宽档**）：把它改成 `["builtin", "primary-model"]`（锚点：
/// `src/operators.rs` 中 `research-source-set` 那一枚的 `backend_candidates`
/// 字段值——`["builtin"]` 这个字面量在七枚里只出现**一次**，故唯一）
/// ⇒ 长度与逐元素两半各红。
#[test]
fn the_one_deterministic_operator_is_the_source_set() {
    let operators = all_operators();

    let deterministic: Vec<&str> = operators
        .iter()
        .filter(|op| op.determinism == Determinism::Deterministic)
        .map(|op| op.id.as_str())
        .collect();
    assert_eq!(
        deterministic,
        ["research-source-set"],
        "七枚里 determinism == Deterministic 的应恰一枚，且是 research-source-set（设计 §6.2 第 1 条）"
    );

    let source_set = operators
        .iter()
        .find(|op| op.id.as_str() == "research-source-set")
        .expect("清单里应有 research-source-set");
    assert_eq!(
        source_set.backend_candidates,
        vec![BackendId::new("builtin")],
        "research-source-set 的 backend_candidates 应逐元素恰为 [\"builtin\"]（设计 §6.2 第 1 条）"
    );
}

/// §9.1：`len() == 7` **且**两两 `id` 不同。
///
/// **这条是为「返回类型的数目」与「清单实际内容」之间的差而写**：返回类型
/// `[Operator; 7]` 已把数目写死在类型里，故 `len() == 7` 那一半不承重（按构造恒真）；
/// 承重的是「两两不同」那一半。
/// 红条件（**取反档**）：把第 7 枚的 id 写成第 6 枚那个串（锚点：
/// `OperatorId::new("research-citation-verification")` 那一处的串——七枚的
/// id 字面量两两不同，故唯一）⇒「两两不同」那一半红，而**返回类型
/// `[Operator; 7]` 的数目仍绿**——本用例的第一断言照常通过，红的是第二断言。
#[test]
fn the_catalog_lists_seven_distinct_ids_despite_the_return_type() {
    let operators = all_operators();
    assert_eq!(
        operators.len(),
        7,
        "清单的数目读数（返回类型已写死 7，故这一半不承重）"
    );

    for (i, earlier) in operators.iter().enumerate() {
        for later in &operators[i + 1..] {
            assert_ne!(
                earlier.id, later.id,
                "id 必须两两不同：{} 出现了两次",
                earlier.id
            );
        }
    }
}

/// §9.1：七枚的 `version` **逐枚**断言 `OperatorVersion::new(1)`。
///
/// 红条件（**取反档**）：改第 4 枚的 `version` 为 `OperatorVersion::new(2)`
/// （锚点：**第 4 枚那个结构体字面量里**的 `version:` 行——`OperatorVersion::new(1)`
/// 这个取值在七枚里出现**七次**，故定位靠它的毗邻串
/// `OperatorId::new("research-evidence-extraction")`）⇒ 第 4 位红；
/// **返回类型不变，故编译仍过**——这也说明它不是编译期照片。
#[test]
fn every_operator_declares_version_one() {
    for (i, op) in all_operators().iter().enumerate() {
        assert_eq!(
            op.version,
            OperatorVersion::new(1),
            "第 {} 枚（{}）的 version 应为 1（设计 §4.2 末的不变量）",
            i + 1,
            op.id
        );
    }
}
