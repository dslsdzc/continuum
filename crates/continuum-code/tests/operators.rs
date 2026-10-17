//! 九枚代码域算子（设计 §3.2）的清单与逐列取值的守卫。
//!
//! 语料（九枚 id、每行的取值）**手写**、逐行取自设计 §3.2 的表：若把期望值写成
//! 按 `all_operators()` 自己遍历，测的是「它等于它自己」——那是恒真的假照片
//! （设计 §10.2 的「预防」）。
//!
//! 本文件钉的是设计 §10.1 的第一条与 §10.2 第 (f)(g)(h)(i) 条，以及 §8「本链零请求」
//! 的可观察形态；边表 (j)、复用 (o)、后端名 (p) 由后续 task 的用例承担。

use continuum_artifact::ArtifactType;
use continuum_code::all_operators;
use continuum_operator::{Determinism, Operator, OperatorVersion, SideEffectClass};

/// 设计 §10.1 的第一条。**两个断言，性质不同**。
///
/// （i）`len() == 9` 是**守卫**，钉的是「返回类型的数目正好是 9」——**它不是恒真的**：
/// 清单少一枚在 Rust 里编不过（数组字面量的元素个数与返回类型不符），故删一枚**必须
/// 同步**把返回类型写成 `[Operator; 8]`，而那一处改动**会让本条红**——这正是设计
/// §10.3 的「移除」那一行预告的 `len() == 9`（纪律 1(b)）。
/// **它不钉的是什么**要一并写明：它**不**钉「九枚的 id 与 §3.2 表逐行对得上」，也**不**
/// 该被写成「清单里没少东西」这一层意思的唯一载体——逐行对应由本节其余各条与
/// Task 4／Task 5 的逐枚断言承担。
///
/// （ii）九个 id 两两比较（不是只查总数）全不相同，且每枚 `version == 1`。
///
/// 红条件（取反档）：把第 5 行与第 6 行的 id 改成同一个 ⇒（ii）红而（i）绿。
/// **锚点唯一**：`operators.rs` 里第 5 行那个 `OperatorId::new("code-spec-review")`。
#[test]
fn the_return_type_pins_the_count_and_the_ids_are_pairwise_distinct() {
    let operators = all_operators();

    // （i）一条守卫，钉「返回类型的数目正好是 9」（设计 §10.1 第一条）。
    assert_eq!(
        operators.len(),
        9,
        "all_operators() 的返回类型钉住的数目应为 9（设计 §10.1）"
    );

    // （ii）两两比较，不是只查总数；每枚 version == 1。
    for (i, earlier) in operators.iter().enumerate() {
        for later in &operators[i + 1..] {
            assert_ne!(
                earlier.id, later.id,
                "id 必须两两不同：{} 出现了两次",
                earlier.id
            );
        }
    }
    for op in &operators {
        assert_eq!(
            op.version,
            OperatorVersion::new(1),
            "{} 的 version 应为 1（设计 §3.2 的不变量）",
            op.id
        );
    }
}

/// 设计 §10.2 第 (f) 条：`side_effect_class` **逐枚，不抽代表**。
/// 第 2、3、5、6、9 行 == `Pure`；第 1、4、7、8 行 == `Idempotent`；
/// **九枚无一为 `NonIdempotent`**（第三条单独一行断言，与前两条合起来才是完整的）。
///
/// 来历：第 4 行于 2026-10-10 由 `NonIdempotent` 改判 `Idempotent`——原判的理由
/// 「重跑一次实现得到的是另一份补丁」属 `Determinism`（两者正交，第 4 行本就是
/// `NonDeterministic`），而「消耗资源」不是「改变状态」（设计 §3.3 第 8 条）。
/// 第 1 行是 `Idempotent` 而非 `NonIdempotent` 的判据同条（§3.3 第 4 条：效果是
/// 目标态、重复调用不产生第二份工作区，与第 8 条并列）；第 7、8 行是 `Idempotent`
/// 而非 `Pure`——两枚的执行体（`cargo-test` / `cargo-fuzz` / `proptest`）在 Task
/// Workspace 里跑构建与测试、**自己落盘**（§3.3 第 9 条）。
///
/// 红条件（取反档）：把第 4 行的 `side_effect_class` 改成 `NonIdempotent`。
/// **锚点唯一**：`operators.rs` 里 `code-implement` 那一行的 `side_effect_class` 字段。
#[test]
fn the_side_effect_class_of_every_row() {
    let operators = all_operators();

    // 第 2、3、5、6、9 行（0 基下标 1、2、4、5、8）== Pure。
    for i in [1usize, 2, 4, 5, 8] {
        assert_eq!(
            operators[i].side_effect_class,
            SideEffectClass::Pure,
            "第 {} 行（{}）的 side_effect_class 应为 Pure（§3.2 表）",
            i + 1,
            operators[i].id
        );
    }

    // 第 1、4、7、8 行（0 基下标 0、3、6、7）== Idempotent。
    for i in [0usize, 3, 6, 7] {
        assert_eq!(
            operators[i].side_effect_class,
            SideEffectClass::Idempotent,
            "第 {} 行（{}）的 side_effect_class 应为 Idempotent（§3.2 表）",
            i + 1,
            operators[i].id
        );
    }

    // 第三条断言（单独一行）：九枚无一为 NonIdempotent（§3.3 第 8 条）。
    assert!(
        operators
            .iter()
            .all(|op| op.side_effect_class != SideEffectClass::NonIdempotent),
        "九枚代码域算子无一为 NonIdempotent（§3.3 第 8 条）"
    );
}

/// 设计 §10.2 第 (g) 条：`determinism` **逐枚**。
/// 第 7、9 行 == `Deterministic`；第 1、2、3、4、5、6、8 行 == `NonDeterministic`。
///
/// §3.3 第 7 条写明这一列的代价：**规范没有一处给过 operator 的 determinism**
/// （§244 `:737-750` 只给字段名）——第 2–6 行的 `NonDeterministic` 是本设计的判定；
/// 规划与拆任务的产物因此不进 §305 的复用，每轮重算。
/// **第 7、8 行分作两枚的理由**（§3.3 第 5 条）：若合成一枚并把 fuzz 放进它的候选里，
/// 那一枚的 `Deterministic` 声明**当场为假**——可观察面就是本条的第 8 行与 Task 4 的 (o)。
///
/// 红条件（**放宽档**，范围少了一半条件）：把第 1 行（`code-workspace`）改成
/// `Deterministic`。**锚点唯一**：`operators.rs` 里 `code-workspace` 那一行的
/// `determinism` 字段。
/// **并写明**：`worktree` 与 `overlay` 两个候选**都在** §3.5 的名单里，故这一条变异
/// **不会**让 Task 2 的注册期规则变红——**§3.5 的规则与这一条是两条独立的守卫**
/// （设计 §3.5 末：前者抓「候选不可复现」，后者抓「输入不可见」）。
#[test]
fn the_determinism_of_every_row() {
    let operators = all_operators();

    // 第 7、9 行（0 基下标 6、8）== Deterministic。
    for i in [6usize, 8] {
        assert_eq!(
            operators[i].determinism,
            Determinism::Deterministic,
            "第 {} 行（{}）的 determinism 应为 Deterministic（§3.2 表）",
            i + 1,
            operators[i].id
        );
    }

    // 第 1、2、3、4、5、6、8 行（0 基下标 0、1、2、3、4、5、7）== NonDeterministic。
    for i in [0usize, 1, 2, 3, 4, 5, 7] {
        assert_eq!(
            operators[i].determinism,
            Determinism::NonDeterministic,
            "第 {} 行（{}）的 determinism 应为 NonDeterministic（§3.2 表）",
            i + 1,
            operators[i].id
        );
    }
}

/// 设计 §10.2 第 (h) 条：第 1 行的 `input_schema` 是空 `Vec`，
/// 且 `output_schema == [SourceTree]`。
///
/// 出处（§3.3 第 2 条）：它的输入是 **Base 工作区，而 Base 不是制品**——给它一个
/// 制品身份等于让下游把 Base 当输入制品消费，而 §256 `:1046` 的
/// `AI_WRITE(BaseWorkspace) = DENY` 正是要拦住这件事。**代价是 Base 的形态不进端口**
/// （设计 §4.3 的第二处结构缺口）。
///
/// 红条件（取反档；按纪律 1：改的是被断言的声明取值，把「空输入」改成「与期望不是
/// 同一枚」）：给它加一个 `[ArtifactType::Json]` 输入 ⇒ 空 `Vec` 断言红。
#[test]
fn code_workspace_takes_no_artifact_input_and_yields_a_source_tree() {
    let operators = all_operators();
    let workspace = &operators[0];

    assert!(
        workspace.input_schema.is_empty(),
        "第 1 行（code-workspace）的 input_schema 应为空 Vec（§3.3 第 2 条）"
    );
    assert_eq!(
        workspace.output_schema,
        vec![ArtifactType::SourceTree],
        "第 1 行（code-workspace）的 output_schema 应为 [SourceTree]（§3.2 表）"
    );
}

/// 设计 §10.2 第 (i) 条：第 5 行与第 6 行**只差 `id`**——其余字段逐字段相同。
///
/// 出处（§3.3 第 10 条）：§186 `:85` 逐字「规格审查与代码质量审查**分离**」——分离
/// 这一条本身是 §186 列的四条优势之一，合成就把它抹掉；而**分离这一条只有两枚同形
/// 的东西才说得上**（两枚是**两个问题**——「这段代码满足了规格吗」与「这段代码的质量
/// 合格吗」，不是一个问题的两种读法）。
///
/// **比较的写法写死：不带 `..` 的穷尽解构**（`Operator` 的七个字段逐个绑定再逐字段
/// 比较）。留了 `..` 的话，`Operator` 将来多出一个字段时本条**静默照过**；不带 `..`
/// 时多一个字段即**编译不过**——那张照片才是「逐字段相同」而不是「我列出的那几列
/// 相同」（纪律 1(c) 末段）。
///
/// 红条件（取反档）：把第 6 行的 `input_schema` 改成 `[Patch]` ⇒ 该字段的断言红。
#[test]
fn the_two_review_operators_differ_only_in_id() {
    let operators = all_operators();

    // 穷尽解构（无 `..`）：七个字段逐个绑定。spec = 第 5 行，quality = 第 6 行。
    let Operator {
        id: spec_id,
        version: spec_version,
        input_schema: spec_input,
        output_schema: spec_output,
        determinism: spec_determinism,
        side_effect_class: spec_side_effect,
        backend_candidates: spec_backends,
    } = &operators[4];
    let Operator {
        id: quality_id,
        version: quality_version,
        input_schema: quality_input,
        output_schema: quality_output,
        determinism: quality_determinism,
        side_effect_class: quality_side_effect,
        backend_candidates: quality_backends,
    } = &operators[5];

    assert_ne!(spec_id, quality_id, "两枚审查算子的 id 必须不同");
    // version 不在 §10.2 第 (i) 条点名的五字段里（它由本条第一个用例统一钉为 1）；
    // 这里一并比较，因为穷尽解构的七个字段各自都要有去处。
    assert_eq!(spec_version, quality_version, "version 应逐字段相同");
    assert_eq!(spec_input, quality_input, "input_schema 应逐字段相同");
    assert_eq!(spec_output, quality_output, "output_schema 应逐字段相同");
    assert_eq!(
        spec_determinism, quality_determinism,
        "determinism 应逐字段相同"
    );
    assert_eq!(
        spec_side_effect, quality_side_effect,
        "side_effect_class 应逐字段相同"
    );
    assert_eq!(
        spec_backends, quality_backends,
        "backend_candidates 应逐字段相同"
    );
}

/// §8「本链零请求」的可观察形态：九枚算子的 `input_schema ∪ output_schema` 里出现的型
/// **恰是**手写的四枚 `{SourceTree, Patch, TestResult, Json}`（**手写期望，不从遍历
/// 结果派生**）。
///
/// 红条件（取反档）：给某一枚算子加一个 `ArtifactType::Blob` 端口。
/// **限度**：这条钉的是「本链用到的型恰是那四枚」，**不钉**「那四枚在 `ArtifactType`
/// 里还在」——后者由 `continuum-artifact` 自己的用例钉（本块一个变体都不加，§8）。
#[test]
fn the_port_types_in_use_are_exactly_the_four_types_of_this_chain() {
    let operators = all_operators();

    let mut seen: Vec<ArtifactType> = operators
        .iter()
        .flat_map(|op| op.input_schema.iter().chain(op.output_schema.iter()))
        .copied()
        .collect();
    // 排序与去重只为失败输出的可读性；断言的是**集合**相等。
    seen.sort_by_key(|t| t.as_str());
    seen.dedup();

    let mut expected = vec![
        ArtifactType::SourceTree,
        ArtifactType::Patch,
        ArtifactType::TestResult,
        ArtifactType::Json,
    ];
    expected.sort_by_key(|t| t.as_str());

    assert_eq!(
        seen, expected,
        "九枚算子端口里出现的型应恰是 {{SourceTree, Patch, TestResult, Json}}（设计 §8）"
    );
}
