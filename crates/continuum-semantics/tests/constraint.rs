//! §225 的 Contract 不可越权原则与 §6 的 Constraint Violation Gate 的用例集
//! （设计 §11.2／§11.3／§11.4）。
//!
//! # 本文件钉的每一件，以及它的红条件
//!
//! | 用例 | 钉的判据 | 红条件（档位） |
//! |---|---|---|
//! | `the_candidate_plan_is_a_value_of_a_plan_plus_a_footprint` | **输入面签名**：`CandidatePlan<'a> { plan: &'a Plan, footprint: &'a PlanFootprint }` 两字段**逐项穷尽解构**（不带 `..`）；`validate` 的 `fn` 指针标注钉死四参数与返回类型（**不收 `&Tx`、不收 `&dyn` 来源**——§1.2.3 规则 3：跨层输入以值进入） | 加一个参数（如 `&Tx`）／换返回类型／改字段名即**编译不过** |
//! | `the_four_footprint_fields_are_all_declared` | `PlanFootprint` 四字段**逐项**（穷尽解构，不带 `..`）：`authority` / `privacy_class` / `external_effects` / `estimated_cost` | 加／少／改名一个字段即**编译不过** |
//! | `the_seven_may_changes_are_all_allowed` | §225 的 MAY **七条逐项**：各构造一条 `execution_policy`、断言 `Satisfied`（放行）；表长恰 7 | 把任一条 MAY 误当禁令拦下即红（**该条红**） |
//! | `the_three_prohibitions_are_all_blocked` | §225 的 MUST NOT 后三条**逐项**：各一条，断言 `ViolatesProhibition { what }` 且 `what` 是**对应那一枚**（`Authority` / `Privacy` / `Effect`）；表长恰 3；**第三条禁令的两个文档字面**（§225 原文版与设计 §11.2／brief 的省略版）各自被拦 | 不拦（**移除档**）／拦成另一枚（`what` 断言红） |
//! | `the_node_side_inputs_keep_their_original_types` | **输入面签名第二侧**：`Node.constraints` 以 `&[String]`、`Node.execution_policy` 以 `&serde_json::Value` 进入（`fn` 指针标注），**不经过本层的类型转换**；第二侧：本 crate 的 `src/` 里执行层 crate 的下划线拼法零命中 | 换成本层自定义类型即**编译不过**；取执行层类型即源码扫描红 |
//! | `a_violation_of_a_required_requirement_is_blocked` | 裁决第 1 条（§11.3）：键**命中**一条 `class = REQUIRED` 的 Requirement → `ViolatesRequired { requirement }`，断言 `requirement` 是**那一个** | 不拦（**移除档**）／带上别的 id |
//! | `a_violation_of_the_other_three_classes_is_not_blocked` | 裁决第 2–4 条：`PREFERRED` / `FLEXIBLE` / `UNSPECIFIED` **各一条**「违反但不拦下」（§224 的四类含义） | 把所有 class 的违反一律拦下（**放宽档**——这三条红） |
//! | `unknown_constraint_key_is_an_error` | 「判不了」第一侧：串**不在当前 Contract 的 Requirement id 集合里** → `Err(UnknownConstraintKey { key })`，`key` 是**那一个** | 悬空键被当 `Satisfied`（**fail-open**）即红 |
//! | `empty_constraints_is_not_an_error` | 「判不了」第二侧：空 `constraints` **不是** `Err`（`Node::new` 就置空，`crates/continuum-graph/src/node.rs:31`） | 空表被当 `Err`（**收紧档**）即红 |
//! | `a_key_pointing_at_a_preferred_requirement_is_not_an_error` | 「判不了」第三侧（评审查出后补）：键**命中** `class = PREFERRED` 的 Requirement → **不是 `Err`、也不拦下**——`Err` 的触发条件是「**`key` 不在 id 集合里**」，**与 `class` 无关** | 「不指向 REQUIRED 就报错」的实现照样全绿的那一格——本用例专治它（否则它绕掉上面三条） |
//!
//! # 用例数写死：7 ＋ 3 ＝ 10
//!
//! 来历（设计 §11.2 的订正段，2026-10-06 评审查出）：设计初稿把这一处写成「（**14** 项）」。
//! §225（`docs/spec/05-normative.md:257-285`）实为 **MAY 七条 ＋ MUST NOT 四条**，
//! 故 7＋3＝10、7＋4＝11，**都不是 14**。§225 的**第四条** MUST NOT（「修改 REQUIRED」）
//! **不是本表的行**——它是上面 `a_violation_of_a_required_requirement_is_blocked` 那一路
//! （`ViolatesRequired`），本文件因此没有它。
//!
//! # 读法申报（收件人：复审者）
//!
//! - **`execution_policy` 的形状是本层读的**：`Node.execution_policy` 的结构规范未给
//!   （§236 只有字段名，`docs/spec/05-normative.md:558-580`）。本用例集把它读作
//!   **一组变更名的 JSON 数组**（元素为清单在文档里逐字出现过的写法——§225 与
//!   设计 §11.2 正文；第三条禁令因此有两个字面）；命中 MUST NOT 后三条 → 拦、
//!   命中 MAY 七条或不命中任何清单条目 → 放行。读法的边界与代价写在 `src/constraint.rs`。
//! - **第三条禁令有**两个**文档逐字有源的字面**：「擅自增加高风险外部副作用」（§225
//!   原文与 §11.3 的对应表）与「增加高风险外部副作用」（设计 §11.2 正文与 task brief
//!   的转述——§11.2 明说它「属 MUST NOT（拦下）」）。**两个都拦**（fix 轮 1 订正：
//!   此前只收含「擅自」的那一条）。
//! - **`constraints` 的键命中 `REQUIRED` 的那一路**：§11.3 判据 3 的订正段写死了
//!   「命中之后**按 REQUIRED 那一路去判**」——即一个键命中 `REQUIRED` 的 Requirement
//!   时为 `ViolatesRequired`；命中非 `REQUIRED` 时裁决由该 Requirement 的 `class` 决定。
//!
//! # 夹具的来历
//!
//! 本文件**不碰库**（validate 是纯函数：不收 `&Tx`、不读写任何表）——故没有 `new_db`
//! 一类夹具。`sample_plan` / `sample_footprint` / `sample_contract` 每个字段取一个
//! 非默认值：本 task 的判定不读 `plan` / `footprint` 的字段（见 `src/constraint.rs`
//! 的申报），夹具仍取非平凡值，好让将来有人误把某个字段接进判定时读回的是可区分的内容。

use std::fs;
use std::path::{Path, PathBuf};

use continuum_canonical::Opaque;
use continuum_semantics::{
    CandidatePlan, ConstraintError, ConstraintVerdict, ContractId, IntentId, Plan, PlanFootprint,
    PlanId, PlanType, PlanVersion, Requirement, RequirementClass, RequirementId, ReviewState,
    TaskContract, VerificationRequirement, ViolatedProhibition, validate,
};

/// 一条基准 Plan：每个字段取一个非默认值。
fn sample_plan() -> Plan {
    Plan {
        id: PlanId::from_str("plan-1"),
        version: PlanVersion { major: 2, minor: 3 },
        intent_id: IntentId::from_str("i-1"),
        r#type: PlanType::Execution,
        objectives: vec![Opaque::from_str("obj-1")],
        milestones: vec![Opaque::from_str("ms-1")],
        assumptions: vec![Opaque::from_str("asm-1")],
        risks: vec![Opaque::from_str("risk-1")],
        acceptance_conditions: vec![Opaque::from_str("ac-1")],
        review_state: ReviewState::AiReviewed,
    }
}

/// 一条基准 `PlanFootprint`：四个字段各取一个非默认值。
fn sample_footprint() -> PlanFootprint {
    PlanFootprint {
        authority: Opaque::from_str("authority-1"),
        privacy_class: Opaque::from_str("privacy-class-1"),
        external_effects: vec![Opaque::from_str("effect-1"), Opaque::from_str("effect-2")],
        estimated_cost: Opaque::from_str("cost-1"),
    }
}

/// 一条基准 Requirement：五个字段各取一个非默认值。
fn requirement(id: &str, class: RequirementClass) -> Requirement {
    Requirement {
        id: RequirementId::from_str(id),
        class,
        expression: Opaque::from_str(&format!("expr-{id}")),
        source: Opaque::from_str(&format!("source-{id}")),
        verification_requirement: VerificationRequirement::from_str(&format!("vr-{id}")),
    }
}

/// 一条基准 TaskContract：每个字段取一个非默认值（六个不透明声明各不同）。
fn sample_contract(requirements: Vec<Requirement>) -> TaskContract {
    TaskContract {
        id: ContractId::from_str("c-1"),
        version: 1,
        intent_id: IntentId::from_str("i-1"),
        requirements,
        budget: Opaque::from_str("budget-1"),
        authority_limit: Opaque::from_str("authority-limit-1"),
        data_policy: Opaque::from_str("data-policy-1"),
        model_policy: Opaque::from_str("model-policy-1"),
        node_policy: Opaque::from_str("node-policy-1"),
        acceptance_predicate: Opaque::from_str("acceptance-1"),
    }
}

/// 「期望 `Ok(Satisfied)`」的形状断言（`ConstraintVerdict` 含 `RequirementId`，
/// 而 id 四枚没有 `PartialEq`——见 `src/ids.rs`；故断言经 `match` 读出）。
fn assert_satisfied(result: Result<ConstraintVerdict, ConstraintError>, what: &str) {
    match result {
        Ok(ConstraintVerdict::Satisfied) => {}
        other => panic!("{what}：期望 Ok(Satisfied)，实得 {other:?}"),
    }
}

/// 输入面（§11.2）：`CandidatePlan` 是**值**——`Plan` ＋ `PlanFootprint` 两个引用。
///
/// `fn` 指针的类型标注把 `validate` 的签名钉死：加一个参数（如 `&Tx`）、换返回类型
/// （如非 `Result`）、或让某个参数变成 `&dyn` 来源，都**编译不过**。§1.2.3 规则 3：
/// 跨层输入以值进入——本 crate 不登记指向执行层的边，也不收任何存储句柄。
///
/// **红条件**：加／换任一参数类型、换返回类型、给 `CandidatePlan` 加／少／改名一个
/// 字段即**编译不过**（编译期照片，承重在编译器）。
#[test]
fn the_candidate_plan_is_a_value_of_a_plan_plus_a_footprint() {
    let check: fn(
        &CandidatePlan<'_>,
        &TaskContract,
        &[String],
        &serde_json::Value,
    ) -> Result<ConstraintVerdict, ConstraintError> = validate;

    let plan = sample_plan();
    let footprint = sample_footprint();
    let candidate = CandidatePlan { plan: &plan, footprint: &footprint };

    // 两个字段**逐项**穷尽解构（不带 `..`）：加／少／改名一个字段即编译不过。
    let CandidatePlan { plan: plan_ref, footprint: footprint_ref } = &candidate;
    assert_eq!(plan_ref.id.as_str(), "plan-1", "`plan` 字段应是调用方给的那一份");
    assert_eq!(
        footprint_ref.authority.as_str(),
        "authority-1",
        "`footprint` 字段应是调用方给的那一份"
    );

    // 真调一次：这条路径同时证明四参数与返回类型可用。
    let verdict = check(&candidate, &sample_contract(Vec::new()), &[], &serde_json::Value::Null);
    assert_satisfied(verdict, "无约束、无策略声明的候选计划");
}

/// §11.2 的 `PlanFootprint`：四个字段**逐项**（穷尽解构，不带 `..`）。
///
/// # 本类型的由来（brief 要求写明的同一段）
///
/// §228 的 `Plan` **没有**这几个字段，而 §225 的四条 MUST NOT 的后三条比的是
/// **计划的声明值**与 Contract 的 `authority_limit` / `data_policy`——没有计划侧的
/// 对偶声明，后三条**无可判**。**故本类型是「让 §225 有判据」的载体**，
/// 不是本层新造的判据：判据是 §225 给的，本类型只是它的第二个操作数。
///
/// **红条件**：加／少／改名一个字段即**编译不过**。
#[test]
fn the_four_footprint_fields_are_all_declared() {
    let footprint = PlanFootprint {
        authority: Opaque::from_str("authority-2"),
        privacy_class: Opaque::from_str("privacy-class-2"),
        external_effects: vec![Opaque::from_str("effect-3")],
        estimated_cost: Opaque::from_str("cost-2"),
    };

    // 四字段逐项穷尽解构（不带 `..`）。
    let PlanFootprint { authority, privacy_class, external_effects, estimated_cost } = &footprint;
    assert_eq!(authority.as_str(), "authority-2", "计划要用的权限上限");
    assert_eq!(privacy_class.as_str(), "privacy-class-2", "计划触及的最高隐私等级");
    assert_eq!(
        external_effects.iter().map(Opaque::as_str).collect::<Vec<_>>(),
        ["effect-3"],
        "计划要产生的外部副作用（类型 + 目标）"
    );
    assert_eq!(estimated_cost.as_str(), "cost-2", "计划的成本估计（§110 的输入）");
}

/// §225 的 MAY **七条逐项**（§11.2 的 10 条之 7）：各一条、断言 `Satisfied`（放行）。
///
/// **用例数写死：7 ＋ 3 ＝ 10**——来历见文件头（设计初稿的「14 项」是错的，
/// 7＋4＝11 也不是 14；第四条 MUST NOT「修改 REQUIRED」不在本表——它是
/// `ViolatesRequired` 那一路）。
///
/// **红条件**：把任一条 MAY 误放进禁令表拦下即**该条红**（**放宽档**）。
#[test]
fn the_seven_may_changes_are_all_allowed() {
    let may_changes = [
        "换模型",
        "换工具",
        "换节点",
        "换算法",
        "重新规划",
        "改变并行度",
        "改变内部执行顺序",
    ];
    assert_eq!(may_changes.len(), 7, "§225 的 MAY 清单恰七条（不许抽代表）");

    let contract = sample_contract(Vec::new());
    let plan = sample_plan();
    let footprint = sample_footprint();
    let candidate = CandidatePlan { plan: &plan, footprint: &footprint };

    for change in may_changes {
        let policy = serde_json::json!([change]);
        let verdict = validate(&candidate, &contract, &[], &policy);
        assert_satisfied(verdict, &format!("{change} 属 §225 的 MAY 清单，应放行"));
    }
}

/// §225 的 MUST NOT 后三条**逐项**（§11.2 的 10 条之 3）：各一条，
/// 断言 `ViolatesProhibition { what }` 且 `what` 是**对应那一枚**。
///
/// 第三条禁令有**两个文档逐字有源的字面**（§225 原文版与设计 §11.2／brief 的省略版），
/// 补一条省略字面的输入——**两条字面各自被拦**（fix 轮 1；见文件头的读法申报）。
///
/// **红条件**：不拦（**移除档**）即该条红；拦成另一枚（`what` 不是对应那枚）即该条红。
#[test]
fn the_three_prohibitions_are_all_blocked() {
    let prohibitions = [
        ("扩大权限", ViolatedProhibition::Authority),
        ("降低隐私约束", ViolatedProhibition::Privacy),
        ("擅自增加高风险外部副作用", ViolatedProhibition::Effect),
    ];
    assert_eq!(prohibitions.len(), 3, "§225 的 MUST NOT 后三条恰三条（不许抽代表）");

    let contract = sample_contract(Vec::new());
    let plan = sample_plan();
    let footprint = sample_footprint();
    let candidate = CandidatePlan { plan: &plan, footprint: &footprint };

    for (change, expected) in prohibitions {
        let policy = serde_json::json!([change]);
        match validate(&candidate, &contract, &[], &policy) {
            Ok(ConstraintVerdict::ViolatesProhibition { what }) => {
                assert_eq!(what, expected, "{change} 应对应 {expected:?} 那一枚");
            }
            other => panic!("{change} 属 §225 的 MUST NOT 后三条，应拦下，实得 {other:?}"),
        }
    }

    // 第三条禁令的**两个文档字面**各自被拦：§225 原文版（上面主表已跑）与
    // 设计 §11.2 正文 / task brief 的省略版（§11.2 明说它「属 MUST NOT（拦下）」）。
    // fix 轮 1 补——只有含「擅自」的那一条曾被拦，省略版走 `None` 被放行。
    for change in ["擅自增加高风险外部副作用", "增加高风险外部副作用"] {
        let policy = serde_json::json!([change]);
        match validate(&candidate, &contract, &[], &policy) {
            Ok(ConstraintVerdict::ViolatesProhibition { what }) => {
                assert_eq!(what, ViolatedProhibition::Effect, "{change} 应对应 Effect 那一枚");
            }
            other => panic!("{change} 属 §225 的 MUST NOT，应拦下，实得 {other:?}"),
        }
    }
}

/// 输入面第二侧（§11.2、§13.2 末行）：两个图侧输入以**原样类型**进入。
///
/// `Node.constraints: Vec<String>`（`crates/continuum-graph/src/node.rs:15`）以
/// `&[String]` 进入、`Node.execution_policy: serde_json::Value`（同文件 `:18`）以
/// `&serde_json::Value` 进入，**不经过本层的类型转换**——`fn` 指针的类型标注钉死
/// 这两个参数就是原样类型（换成 `&[Opaque]`、本层自定义类型即编译不过）。
///
/// 第二侧（本 crate 对 `continuum-graph` 的引用为零）：源码扫描，
/// 与 `tests/intent.rs` 的 `graph_id_and_product_id_are_opaque_strings` 同款。
///
/// **红条件**：任一参数换成非原样类型即**编译不过**；`src/` 里出现执行层 crate 的
/// 下划线拼法（`use` 或全限定路径）即源码扫描红。
#[test]
fn the_node_side_inputs_keep_their_original_types() {
    // 「原样类型」的签名照片：类型标注逐参照抄 §11.2 的两个类型。
    let check: fn(
        &CandidatePlan<'_>,
        &TaskContract,
        &[String],
        &serde_json::Value,
    ) -> Result<ConstraintVerdict, ConstraintError> = validate;

    // 真调一次：`Node::new` 的默认值（`Value::Null` ＋ 空 `constraints`）是合法输入。
    let plan = sample_plan();
    let footprint = sample_footprint();
    let candidate = CandidatePlan { plan: &plan, footprint: &footprint };
    let constraints: Vec<String> = Vec::new();
    let policy = serde_json::Value::Null;
    assert_satisfied(
        check(&candidate, &sample_contract(Vec::new()), &constraints, &policy),
        "`Node::new` 的默认值形态（空 constraints、Null policy）",
    );

    // 第二侧：源码扫描（射程：本 crate 的 `src/`，不含 `tests/`——本文件里出现的
    // 那个拼法是**被断言的语料**，不是使用）。
    let sources = crate_sources();
    let joined: String = sources.iter().map(|(_, s)| s.as_str()).collect::<Vec<_>>().join("\n");
    // **先**证明扫描真的读到了这棵树（否则下面那条断言在空语料上恒真、恒绿）。
    assert!(
        sources.len() >= 4,
        "只扫到 {} 个 `src/` 下的 `.rs`：源码树没被读到",
        sources.len()
    );
    assert!(
        joined.contains("pub struct CandidatePlan"),
        "读到了 {} 个文件，但拼起来找不到 `pub struct CandidatePlan`——读到的不是这棵树",
        sources.len()
    );
    assert!(
        !joined.contains("continuum_graph"),
        "本 crate 的 `src/` 里出现了执行层 crate 的下划线拼法——那会写出一条指向执行层的\
         依赖边（设计 §1.2.3 的规则 3）。两个输入都以原样类型进入，不取执行层的类型。"
    );
}

/// 裁决第 1 条（§11.3）：键**命中**一条 `class = REQUIRED` 的 Requirement →
/// `ViolatesRequired { requirement }`，`requirement` 是**那一个**。
///
/// §11.3 判据 3 的订正段：「命中之后**按 REQUIRED 那一路去判**」。
///
/// **红条件**：不拦（**移除档**）即红；带上别的 id 即红。
#[test]
fn a_violation_of_a_required_requirement_is_blocked() {
    let contract = sample_contract(vec![requirement("r-required", RequirementClass::Required)]);
    let plan = sample_plan();
    let footprint = sample_footprint();
    let candidate = CandidatePlan { plan: &plan, footprint: &footprint };

    let result = validate(&candidate, &contract, &["r-required".to_owned()], &serde_json::Value::Null);
    match result {
        Ok(ConstraintVerdict::ViolatesRequired { requirement }) => {
            assert_eq!(requirement.as_str(), "r-required", "`requirement` 应是**那一个** id");
        }
        other => panic!("命中 REQUIRED 的约束键应得 ViolatesRequired，实得 {other:?}"),
    }
}

/// 裁决第 2–4 条（§11.3 末段）：`PREFERRED` / `FLEXIBLE` / `UNSPECIFIED` 的
/// 「违反」**不拦下**——§224 的四类含义：PREFERRED「有充分理由时可调整」、
/// FLEXIBLE「明确允许 Runtime 自动调整」。
///
/// **红条件**：把所有 class 的违反一律拦下（**放宽档**——这三条红）。
#[test]
fn a_violation_of_the_other_three_classes_is_not_blocked() {
    let classes = [
        RequirementClass::Preferred,
        RequirementClass::Flexible,
        RequirementClass::Unspecified,
    ];
    assert_eq!(classes.len(), 3, "除 REQUIRED 外恰三枚 class（不许抽代表）");

    let plan = sample_plan();
    let footprint = sample_footprint();
    let candidate = CandidatePlan { plan: &plan, footprint: &footprint };

    for class in classes {
        let contract = sample_contract(vec![requirement("r-1", class)]);
        let result = validate(&candidate, &contract, &["r-1".to_owned()], &serde_json::Value::Null);
        assert_satisfied(result, &format!("{class:?} 的违反不拦下（§224 的四类含义）"));
    }
}

/// 「判不了」第一侧（§11.3 判据 1、2）：**悬空约束键**必须被拦下，
/// 而「拦下」在 `Result` 上的形态是 `Err`（`Satisfied` 会让它以「已判过、通过」的
/// 形式流下去——正是 fail-open）。
///
/// **红条件**：悬空键被当 `Satisfied` 即红；`key` 不是**那一个**（被改写／裁剪）即红。
#[test]
fn unknown_constraint_key_is_an_error() {
    let contract = sample_contract(vec![requirement("r-1", RequirementClass::Required)]);
    let plan = sample_plan();
    let footprint = sample_footprint();
    let candidate = CandidatePlan { plan: &plan, footprint: &footprint };

    let result = validate(&candidate, &contract, &["r-unknown".to_owned()], &serde_json::Value::Null);
    match result {
        Err(ConstraintError::UnknownConstraintKey { key }) => {
            assert_eq!(key, "r-unknown", "`key` 应是**那一个**原始串（不改写、不裁剪）");
        }
        other => panic!("不在 Requirement id 集合里的键应得 Err(UnknownConstraintKey)，实得 {other:?}"),
    }
}

/// 「判不了」第二侧：空 `constraints` **不是** `Err`（空集合法——
/// `Node::new` 就置空，`crates/continuum-graph/src/node.rs:31`）。
///
/// **红条件**：空表被当 `Err`（**收紧档**）即红。
#[test]
fn empty_constraints_is_not_an_error() {
    let contract = sample_contract(vec![requirement("r-1", RequirementClass::Required)]);
    let plan = sample_plan();
    let footprint = sample_footprint();
    let candidate = CandidatePlan { plan: &plan, footprint: &footprint };

    let result = validate(&candidate, &contract, &[], &serde_json::Value::Null);
    assert_satisfied(result, "空 constraints 合法（`Node::new` 就置空）");
}

/// 「判不了」第三侧（评审查出后补的那一条）：键**命中**一条 `class = PREFERRED` 的
/// Requirement → **不是 `Err`、也不拦下**。
///
/// **没有这一条，「不指向 REQUIRED 就报错」的实现照样全绿**，而那会绕掉
/// `a_violation_of_the_other_three_classes_is_not_blocked` 的三条。
/// **判据写死**：`Err` 的触发条件是「**`key` 不在当前 Contract 的 Requirement id
/// 集合里**」，**与 `class` 无关**；命中的键的裁决由该 Requirement 的 `class` 决定。
///
/// **红条件**：「不指向 REQUIRED 就报错」的实现（`Err` 半边红）或把它拦下
/// （Satisfied 半边红）——**两条断言都写，不靠一条顺带**。
#[test]
fn a_key_pointing_at_a_preferred_requirement_is_not_an_error() {
    let contract = sample_contract(vec![requirement("r-preferred", RequirementClass::Preferred)]);
    let plan = sample_plan();
    let footprint = sample_footprint();
    let candidate = CandidatePlan { plan: &plan, footprint: &footprint };

    let result = validate(&candidate, &contract, &["r-preferred".to_owned()], &serde_json::Value::Null);
    // 第一条：不是 `Err`（`Err` 的触发条件与 `class` 无关）。
    assert!(
        result.is_ok(),
        "命中 PREFERRED 的键不是 Err（`Err` 的触发条件是「不在 id 集合里」），实得 {result:?}"
    );
    // 第二条：也不拦下（裁决由该 Requirement 的 `class` 决定）。
    assert_satisfied(result, "命中非 REQUIRED 的键：裁决由 `class` 决定——不拦下");
}

/// 本 crate `src/` 下的全部 `.rs`（路径排序，消息里的次序因此确定）。
///
/// **按 `CARGO_MANIFEST_DIR` 定，不按进程 CWD**（同 `tests/plan.rs` 的 `crate_sources`：
/// 按相对路径扫的守卫在 crate 根下跑是绿的，换个目录跑就**读不到东西**）。
fn crate_sources() -> Vec<(PathBuf, String)> {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    collect_rs(&src, &mut files);
    files.sort();
    files
        .into_iter()
        .map(|path| {
            let source = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("读不到源文件 {}：{e}", path.display()));
            (path, source)
        })
        .collect()
}

/// 递归收集 `dir` 下全部 `.rs`。**读不到的目录直接炸，不跳过**（同 `tests/plan.rs`）。
fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("列不出目录 {}：{e}", dir.display()))
    {
        let path = entry.unwrap_or_else(|e| panic!("列目录 {} 时出错：{e}", dir.display())).path();
        if path.is_dir() {
            collect_rs(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}
