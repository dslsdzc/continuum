//! Task 2 的照片集：五枚算子声明的**逐枚逐列**断言（§11.2 的 a、q、r 与 §3.4、§5.5）
//! 与 §343 的批次边界。
//!
//! **两条预防**（设计 §11.2 的「一条预防」，逐字落在本文件）：
//! 1. 期望清单**必须手写**——语料若从被测清单里取（遍历 `all_operators()` 去比
//!    `all_operators()`），它测的是**恒等式**，是一张恒真的假照片；
//! 2. 不拿 `backend_candidates` 的**去重后**的集合作语料——去重会把「同一候选写了两遍」
//!    这类漂移吸收掉。
//!
//! **用例按 id 取行、不按下标**：设计 §3.2 表的重排（计划纪律 1(b) 的 (ii)）是
//! **等价变异体**——按 id 取的用例对它是透明的；按下标取（`all_operators()[0]` 之类）
//! 会把断言的锚点指到别的行，本计划不许那种写法。

use continuum_artifact::ArtifactType;
use continuum_image::{V01_OPERATOR_IDS, all_operators};
use continuum_operator::{BackendId, Determinism, SideEffectClass};

/// 本文件手写的五枚 id 清单（设计 §3.2 表的行序，逐字手写）。
/// `the_ids_are_pairwise_distinct_and_all_versions_are_one` 与
/// `no_sixth_operator_is_registered` **共用这一份**——两处维护同一件事会更早漂。
const EXPECTED_IDS: [&str; 5] = [
    "generate-image",
    "detect-edit-region",
    "local-generative-edit",
    "hard-composite",
    "verify-outside-mask",
];

/// 设计 §3.2 表的一行，**逐格手写**（期望值的唯一一份来源，不从 `all_operators()` 派生）。
#[derive(Clone, Copy)]
struct Row {
    id: &'static str,
    input_schema: &'static [ArtifactType],
    output_schema: &'static [ArtifactType],
    determinism: Determinism,
    side_effect_class: SideEffectClass,
    backend_candidates: &'static [&'static str],
}

/// 设计 §3.2 表的五行，**逐字手写**（设计 `:239-243`）。表中 `determinism` /
/// `side_effect_class` / backend 候选三列在规范未给处**由设计定**（表里逐格标「定」）。
const TABLE: [Row; 5] = [
    Row {
        id: "generate-image",
        input_schema: &[ArtifactType::Image],
        output_schema: &[ArtifactType::Image],
        determinism: Determinism::NonDeterministic,
        side_effect_class: SideEffectClass::Pure,
        backend_candidates: &["image-gen-backend"],
    },
    Row {
        id: "detect-edit-region",
        input_schema: &[ArtifactType::Image],
        output_schema: &[ArtifactType::Blob],
        determinism: Determinism::NonDeterministic,
        side_effect_class: SideEffectClass::Pure,
        backend_candidates: &["object-detector-backend"],
    },
    Row {
        id: "local-generative-edit",
        input_schema: &[ArtifactType::Image, ArtifactType::Blob],
        output_schema: &[ArtifactType::Image],
        determinism: Determinism::NonDeterministic,
        side_effect_class: SideEffectClass::Pure,
        backend_candidates: &["inpainting-backend"],
    },
    Row {
        id: "hard-composite",
        input_schema: &[ArtifactType::Image, ArtifactType::Blob],
        output_schema: &[ArtifactType::Image],
        determinism: Determinism::Deterministic,
        side_effect_class: SideEffectClass::Idempotent,
        backend_candidates: &["builtin"],
    },
    Row {
        id: "verify-outside-mask",
        input_schema: &[ArtifactType::Image, ArtifactType::Blob],
        output_schema: &[ArtifactType::Json],
        determinism: Determinism::Deterministic,
        side_effect_class: SideEffectClass::Pure,
        backend_candidates: &["builtin"],
    },
];

/// 端口两列按**集合**读（设计 §3.2 第 1 条：`input_schema` 是**类型**集合、不是**元数**），
/// 故比较前按编码串排序——次序不是判据。
fn sorted_type_names(types: &[ArtifactType]) -> Vec<&'static str> {
    let mut names: Vec<&'static str> = types.iter().map(ArtifactType::as_str).collect();
    names.sort_unstable();
    names
}

/// 候选 backend 同样按**集合**读（设计 §3.5 的判定与 §245 的 `is_candidate_backend`
/// 都按集合读）：故「两项换序」是**等价变异体**（计划纪律 1(b) 的 (i)），
/// 而「换成另一个同样合法的串」不是——期望值手写时它会红那一枚。
fn sorted_backend_ids<'a>(backends: &[&'a str]) -> Vec<&'a str> {
    let mut ids: Vec<&'a str> = backends.to_vec();
    ids.sort_unstable();
    ids
}

/// §11.2 第 (a) 条：五枚算子的端口两列、`determinism`、`side_effect_class`、backend 候选
/// **逐枚逐列**断言——**不抽代表**（五行各自断言，不是「抽查一行」）。
///
/// **红条件逐列**（每条都指到唯一锚点——「哪一枚算子的哪一列」）：
/// - 把 `generate-image` 的 `input_schema` 写成空 `Vec`（**取反档**）→ 那一枚的红；
/// - 把 `detect-edit-region` 的 `output_schema` 改成 `[ArtifactType::Image]`（**取反档**）
///   → 那一枚的红；
/// - 把 `local-generative-edit` 的 `determinism` 改成 `Deterministic`（**放宽档**——
///   `Deterministic` 放宽 §305 的复用面；档位口径见计划纪律 1 与 `## 遗留` 第一节第 5 条）
///   → 那一枚的红；
/// - 把 `hard-composite` 的 `side_effect_class` 改成 `Pure`（**取反档**）→ 那一枚的红；
/// - 把 `verify-outside-mask` 的候选串 `builtin` 改成 `core`（**取反档**）→ 那一枚的红。
///
/// **`backend_candidates` 的次序在本块的全部断言里都不是判据**（它是候选集合）：
/// 故「两项换序」是**等价变异体**（计划纪律 1(b) 的 (i)）；**「换成另一个同样合法的串」
/// 不是**——本用例的期望值手写、逐枚断言，它**会红那一枚**（计划纪律 1(b) 里
/// 「先排除一处易误判的」那段与 `## 遗留` 第一节第 12 条：设计 §11.3 预告的那一行
/// 「全绿」与设计自己的 §11.2 第 (a) 条相抵，本计划按「取反」记）。
#[test]
fn every_row_of_the_table_is_pinned_verbatim() {
    let ops = all_operators();
    for row in TABLE {
        // 「按 id 取行」：从 all_operators() 里按 id 找这一行对应的算子（不按下标——
        // 行序的重排是等价变异体）。
        let op = ops
            .iter()
            .find(|op| op.id.as_str() == row.id)
            .unwrap_or_else(|| panic!("all_operators() 里没有 id = {}（期望表有）", row.id));

        // 端口两列（类型集合，按集合读）。
        assert_eq!(
            sorted_type_names(&op.input_schema),
            sorted_type_names(row.input_schema),
            "{} 的 input_schema 与 §3.2 表不符",
            row.id
        );
        assert_eq!(
            sorted_type_names(&op.output_schema),
            sorted_type_names(row.output_schema),
            "{} 的 output_schema 与 §3.2 表不符",
            row.id
        );
        assert_eq!(
            op.determinism, row.determinism,
            "{} 的 determinism 与 §3.2 表不符",
            row.id
        );
        assert_eq!(
            op.side_effect_class, row.side_effect_class,
            "{} 的 side_effect_class 与 §3.2 表不符",
            row.id
        );
        let got_backends: Vec<&str> = op
            .backend_candidates
            .iter()
            .map(BackendId::as_str)
            .collect();
        assert_eq!(
            sorted_backend_ids(&got_backends),
            sorted_backend_ids(row.backend_candidates),
            "{} 的 backend_candidates 与 §3.2 表不符（按集合读）",
            row.id
        );
    }
}

/// 五枚 id **逐枚**比对（按 id 取行，不按下标）＋ **两两比较**全不相同 ＋ 每枚 `version == 1`。
///
/// **本块不写 `len() == 5`**（设计 §11.1 明写「不补」）：它测的是 Rust 的 `len()`，
/// 而「五枚」这个数目已写在 `all_operators()` 的**返回类型** `[Operator; 5]` 里
/// （编译期可见——写少一枚而不改返回类型，直接编不过）。
/// **本用例的逐枚比对才是「数目」的运行期照片**：手写清单（`EXPECTED_IDS`）少一枚、
/// 多一枚或漂一枚，两个方向的比对都会红。
///
/// **红条件**：把 `hard-composite` 与 `verify-outside-mask` 的 id 改成同一个（**取反档**）
/// →「两两比较」那条红。
#[test]
fn the_ids_are_pairwise_distinct_and_all_versions_are_one() {
    let ops = all_operators();

    // 方向一：手写清单的每一枚都能在 all_operators() 里找到（按 id 取行）。
    for id in EXPECTED_IDS {
        assert!(
            ops.iter().any(|op| op.id.as_str() == id),
            "all_operators() 里应有 id = {id}（手写清单列出）"
        );
    }
    // 方向二：all_operators() 的每一枚都在这份手写清单里。
    for op in &ops {
        assert!(
            EXPECTED_IDS.contains(&op.id.as_str()),
            "all_operators() 出现了手写清单之外的 id = {}",
            op.id.as_str()
        );
    }

    // 「两两比较」全不相同——不是只查总数（总数相同、内容可重）。
    for i in 0..ops.len() {
        for j in (i + 1)..ops.len() {
            assert_ne!(
                ops[i].id,
                ops[j].id,
                "第 {i} 枚与第 {j} 枚的 id 相同：{}",
                ops[i].id
            );
        }
    }

    // 每枚 version == 1。
    for op in &ops {
        assert_eq!(op.version.as_u32(), 1, "{} 的 version 应为 1", op.id);
    }
}

/// §11.2 第 (q) 条：两个方向都断——(i) `V01_OPERATOR_IDS` 的每一项都能在
/// `all_operators()` 的 id 里找到；(ii) 它的补集**恰为**其余四枚——**逐枚断言**。
///
/// **不是只看长度**：只看长度的话，把两批的成员对调也能过（设计 §9.2）——
/// 故 (ii) 是「`generate-image` 一条『在』＋ 其余四枚各一条『不在』」。
///
/// **强度的两半据实分开**（设计 §9.1 第 2 条与设计 §14 第 1 条）：
/// - 「局部修改在 v0.1 之外」是**规范明写**——`docs/spec/05-normative.md:2737` 的
///   「可以暂不实现」清单含 `:2743` 的 `Image Local Edit`；
/// - 「完整生成仍在内」是**推断**——`generate-image` **既不在**那份暂缓清单里，
///   **也不在** `:2711` 的必做十六项里。
/// **若协调者把那一条裁反，本用例的期望值随之改，而动作不变。**
///
/// **红条件**：把 `V01_OPERATOR_IDS` 换成 `["hard-composite"]`（**取反档**）→ (ii) 里
/// `generate-image` 那条「在」与 `hard-composite` 那条「不在」**各一条红**（逐枚，不抽代表）。
#[test]
fn the_v01_batch_is_generate_image_and_the_complement_is_the_other_four() {
    let ops = all_operators();

    // (i) 每一项都能在 all_operators() 的 id 里找到。
    for id in V01_OPERATOR_IDS {
        assert!(
            ops.iter().any(|op| op.id.as_str() == id),
            "v0.1 批的 {id} 在 all_operators() 里找不到"
        );
    }

    // (ii) 补集恰为其余四枚——逐枚断言（不抽代表、不是只看长度）。
    assert!(
        V01_OPERATOR_IDS.contains(&"generate-image"),
        "generate-image 应在 v0.1 批里（设计 §9.2 的 v0.1 批那一行）"
    );
    assert!(
        !V01_OPERATOR_IDS.contains(&"detect-edit-region"),
        "detect-edit-region 不在 v0.1 批里（设计 §9.2 的其后批）"
    );
    assert!(
        !V01_OPERATOR_IDS.contains(&"local-generative-edit"),
        "local-generative-edit 不在 v0.1 批里（设计 §9.2 的其后批）"
    );
    assert!(
        !V01_OPERATOR_IDS.contains(&"hard-composite"),
        "hard-composite 不在 v0.1 批里（设计 §9.2 的其后批）"
    );
    assert!(
        !V01_OPERATOR_IDS.contains(&"verify-outside-mask"),
        "verify-outside-mask 不在 v0.1 批里（设计 §9.2 的其后批）"
    );
}

/// §11.2 第 (r) 条：两枚 `Deterministic` 算子的候选**恰一个**——逐枚两条
/// （`hard-composite` 与 `verify-outside-mask` 各一条，循环展开即两条）。
///
/// **用途（写清）**（设计 §3.5）：**这一条红即说明本块需要自己那份 §3.5 的名单检查**
/// ——候选集合是单元集时，「名单内 / 名单外」在本块没有可触发的形态；本块因此
/// **不重写那条与领域无关的规则**（它只写在 P5e 一处，守的是 §305 复用的前提），
/// **也不给它建第二份实现**。
///
/// **红条件**：给 `hard-composite` 的候选加一项（**取反档**）→ 那一条红。
#[test]
fn the_two_deterministic_operators_have_exactly_one_backend_candidate() {
    let ops = all_operators();

    for id in ["hard-composite", "verify-outside-mask"] {
        let op = ops
            .iter()
            .find(|op| op.id.as_str() == id)
            .unwrap_or_else(|| panic!("all_operators() 里没有 id = {id}"));
        assert_eq!(
            op.backend_candidates.len(),
            1,
            "{id} 的 backend_candidates 应恰有一个（设计 §3.2 表的单元集）"
        );
    }
}

/// §3.4 末「本块向 P5e 报出的清单为空」的**可观察形态**：五枚算子的端口
/// （`input_schema ∪ output_schema`）出现的型**恰是**手写的三枚
/// `{Image, Blob, Json}`（**手写期望，不从遍历结果派生**）。
///
/// **与计划的偏离（照实）**：计划（与 Task 2 的 brief）写本用例的期望是「手写的两枚
/// `{Image, Blob}`」、红条件是「加 `ArtifactType::Json` 端口 → 红」；而设计 §3.2 表的
/// 第 5 行（`verify-outside-mask`，设计 `:243`）的输出就是 `[Json]`——端口出现的型是
/// **三枚**，不是两枚。「两枚」与它自己要求实现的「逐枚逐列照 §3.2 表」相抵，
/// 同源于设计 §3.4 末那一句（设计 `:298`）的漏数（它只数了 `Image` 与 `Blob`，
/// 漏了同样「已在枚举里」的 `Json`）。照本任务的第一硬约束（照表），本处在**表**
/// 这一侧取三枚期望、红条件改用**表外**的型；该偏离已记入 Task 2 报告。
///
/// **它钉的是「本块用到的型恰是这三枚」，不钉「这三枚在 `ArtifactType` 里还在」**
/// ——后者由 `continuum-artifact` 自己的用例钉（本块一个变体都不加）。
///
/// **红条件**：给某一枚算子加一个**表外**的型端口（如 `ArtifactType::Audio`）（**取反档**）→ 红。
#[test]
fn the_port_types_in_use_are_exactly_the_three_types_of_the_table() {
    // 手写期望，不从遍历结果派生。
    const EXPECTED_PORT_TYPES: [ArtifactType; 3] =
        [ArtifactType::Image, ArtifactType::Blob, ArtifactType::Json];

    // 遍历五枚的 `input_schema ∪ output_schema`，收出现的型（去重）。
    let mut seen: Vec<ArtifactType> = Vec::new();
    for op in all_operators() {
        for t in op.input_schema.iter().chain(op.output_schema.iter()) {
            if !seen.contains(t) {
                seen.push(*t);
            }
        }
    }

    // 两个方向合起来是「恰是」。
    for t in EXPECTED_PORT_TYPES {
        assert!(seen.contains(&t), "端口应出现 {t:?}（设计 §3.2 表）");
    }
    for t in &seen {
        assert!(
            EXPECTED_PORT_TYPES.contains(t),
            "端口出现了表外的型 {t:?}"
        );
    }
}

/// §5.5 的可观察形态：本块**不注册** §168 第一项检查（「修改区域是否符合要求」）的算子
/// ——断言 `all_operators()` 的 id 集合**恰等于**手写的五枚清单（同一个 `EXPECTED_IDS`，
/// 与 `the_ids_are_pairwise_distinct_and_all_versions_are_one` **共用**——
/// 两处维护同一件事会更早漂）。
///
/// **来历**（设计 §5.5）：那一项是**语义判定**（要读 §324 `:2341` 的 `instruction`，
/// 判「改成这样算不算符合要求」）；它的判据规范未给，它要的是 §262 第 3 级的独立
/// 模型验证器——**给它编一枚算子等于给它编一个判据**。
/// 本用例的「恰等于」就是这条「不注册」的可观察形态：任何第六枚（尤其承担那项
/// 语义检查的）出现在 `all_operators()` 里即红。
#[test]
fn no_sixth_operator_is_registered() {
    let ops = all_operators();

    let mut got: Vec<&str> = ops.iter().map(|op| op.id.as_str()).collect();
    got.sort_unstable();
    let mut expected = EXPECTED_IDS.to_vec();
    expected.sort_unstable();

    assert_eq!(
        got, expected,
        "all_operators() 的 id 集合应恰等于手写的五枚清单（§5.5：多一枚即红）"
    );
}
