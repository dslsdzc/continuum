//! §3.5 的注册期规则（先核后写）与它的封闭名单：正例与反例成对
//! （设计 §10.2：缺一条即不算钉住）。
//!
//! 本文件钉的是设计 §10.2 第 (a)(b)(c)(d)(e) 条与 §10.1 的名单逐项断言；
//! 边表 (j)、复用 (o)、后端名 (p) 与目录条目 (k)(l)(m)(n) 由后续 task 的用例承担。

use continuum_artifact::ArtifactType;
use continuum_code::{all_operators, register_code_operators, CodeError, REPRODUCIBLE_BACKENDS};
use continuum_operator::{
    BackendId, Determinism, Operator, OperatorError, OperatorId, OperatorRegistry, OperatorVersion,
    SideEffectClass,
};

/// 设计 §10.2 第 (a) 条：以 `&all_operators()` 调 `register_code_operators`
/// 注册九枚后，逐枚 `resolve` 成功。
///
/// 红条件（**移除档**）：把 `register_code_operators` 改成空实现 ⇒ 逐枚 `resolve`
/// 得 `OperatorError::NotFound`（九枚全红）。
#[test]
fn registering_the_whole_set_then_resolving_each_succeeds() {
    let operators = all_operators();
    let mut registry = OperatorRegistry::new();

    register_code_operators(&mut registry, &operators)
        .expect("九枚合法算子一次注册应成功（§10.2 第 (a) 条）");

    // 逐枚 resolve：每一枚都要能按 (id, version 1) 取回。
    for op in &operators {
        registry
            .resolve(&op.id, &OperatorVersion::new(1))
            .unwrap_or_else(|err| {
                panic!(
                    "{} 注册后应能 resolve 成功，实际为 {err:?}（§10.2 第 (a) 条）",
                    op.id
                )
            });
    }
}

/// 设计 §10.2 第 (b) 条：同一注册表注册两次 ⇒
/// `Err(CodeError::Registry(OperatorError::Duplicate { .. }))`，**不是**静默覆盖。
/// 判定的那一臂要**判准**（`matches!`），不是「是 `Err` 就算过」。
///
/// 第二次调用时，批次里第一枚（第 1 行 `code-workspace`）就会撞上已在表里的同
/// (id, version)——撞车判定是 P1 的（`OperatorError::Duplicate`，注册表的既有条目），
/// 本块只把它经 `CodeError::Registry` 带出来（设计 §3.2：那一臂是**包装**，不是同名臂）。
///
/// 红条件（**移除档**）：把重复注册改成静默覆盖 ⇒ 第二次调用返回 `Ok`，本条红
/// （没有可供判臂的 `Err`）。
/// **锚点唯一**：`register.rs` 里把 `OperatorError` 转进 `CodeError::Registry` 的那个 `?`。
#[test]
fn registering_twice_yields_the_registry_arm_not_a_silent_overwrite() {
    let mut registry = OperatorRegistry::new();
    register_code_operators(&mut registry, &all_operators()).expect("第一次注册应成功");

    let err = register_code_operators(&mut registry, &all_operators())
        .expect_err("同 (id, version) 二次注册应返回 Err，不得静默覆盖（§10.2 第 (b) 条）");

    // 判准那一臂——不是「是 Err 就算过」。
    assert!(
        matches!(&err, CodeError::Registry(OperatorError::Duplicate { .. })),
        "错误臂应为 CodeError::Registry(OperatorError::Duplicate)，实际为 {err:?}"
    );
}

/// 设计 §10.2 第 (c) 条：`determinism == Deterministic` 的**每一枚**算子（不是抽样），
/// 其**每个** `backend_candidate` 都在 §3.5 的名单内。
///
/// 本条遍历九枚里 `Deterministic` 的两枚——第 7 行（`code-test-run`）与
/// 第 9 行（`code-verify`）——逐枚判候选是否在名单内（第 9 行的候选就是 `builtin`，
/// 设计 §3.2 表的 backend 候选列）。「第 7、9 行是 `Deterministic`，其余不是」这一列
/// 由 `tests/operators.rs` 的 `the_determinism_of_every_row`（设计 §10.2 第 (g) 条）钉住；
/// 本条钉的是名单与这些候选之间的一致性。
///
/// **为什么本条与名单逐项断言（`the_list_is_exactly_these_five_names`）都要留**：
/// 「从名单里删掉 `builtin`」这一变异**两条都红**（本条红在第 9 行的候选、名单条
/// 红在名单自身），而两条各有一侧是对方抓不到的——给名单**加**一项（如
/// `"remote-cluster"`）⇒ 名单条红而本条**绿**（九枚的候选都还在名单里）；
/// 把某枚 `Deterministic` 算子的候选**改成**名单外的名 ⇒ 本条红而名单条**绿**（名单没变）。
/// （删 `"builtin"` 的连锁**不止这两条**——注册路径的 (a)(b)(d)(e) 也红，
/// 见 `the_list_is_exactly_these_five_names` 的注释。）
#[test]
fn every_deterministic_operator_has_all_its_candidates_in_the_list() {
    let operators = all_operators();

    for op in &operators {
        if op.determinism != Determinism::Deterministic {
            continue;
        }
        for candidate in &op.backend_candidates {
            assert!(
                REPRODUCIBLE_BACKENDS.contains(&candidate.as_str()),
                "{} 声明 Deterministic，但其候选 backend {} 不在 §3.5 的名单内（§10.2 第 (c) 条）",
                op.id,
                candidate.as_str()
            );
        }
    }
}

/// 设计 §10.1 的逐项断言：§3.5 的名单**恰是**手写的这五项，**集合相等且个数相等**
/// （排序只为失败输出的可读性；排序后逐位比较同时钉住两个方向与个数）。
///
/// **期望值必须手写、不从常量自身取**——从 `REPRODUCIBLE_BACKENDS` 派生期望值，
/// 测的是「它等于它自己」，那是恒真的假照片（纪律 1(b)(ii)）。
///
/// 红条件（**移除档**）：从常量里删掉 `"builtin"`，**并把类型字面量同步改成
/// `[&str; 4]`** ⇒ 本条红、**(c) 也红**（(c) 遍历九枚里 `determinism ==
/// Deterministic` 的两枚——第 7、9 行——逐枚判候选是否在名单内，而第 9 行
/// `code-verify` 的候选就是 `builtin`（设计 §3.2 表的 backend 候选列），
/// 删掉该项后第 9 行立刻不成）。
/// **本条与 (c) 的红不经过注册路径**（直接读常量判名单／候选）；**注册路径的四条也红**
/// ——删掉 `"builtin"` 后**核段先拒 `all_operators()` 的 `code-verify`**：(a) 的
/// `.expect` 当场 panic、(b) 的第一次注册同样 panic、(d) 里 `operators[8]` 先于违规枚
/// 被拒而返回的 `operator` 是 `code-verify` 而非 rogue（字段判据红）、(e) 里表外枚
/// （候选也是 `builtin`）被拒而 `.expect` panic。**两串连锁要分开读。**
///
/// **类型字面量那一半要写进红条件**：`REPRODUCIBLE_BACKENDS` 是 `[&str; 5]`
/// （本 task Step 3），只删一项而不改类型即**编译不过**，按纪律 1(c) **不算红**。
/// **这一句要写进注释**：它说明为什么本条与 (c) 都要留——本条钉名单自身的封闭
/// （多一项／少一项／重复一项都红），(c) 钉名单与九枚声明的逐枚一致；
/// 只留任一条都留下一侧静默错误（两侧的例子见 (c) 的注释）。
#[test]
fn the_list_is_exactly_these_five_names() {
    let mut actual: Vec<&str> = REPRODUCIBLE_BACKENDS.to_vec();
    actual.sort_unstable();

    let mut expected: Vec<&str> = vec![
        "worktree",
        "overlay",
        "builtin",
        "cargo-test",
        "local-test-runner",
    ];
    expected.sort_unstable();

    assert_eq!(
        actual, expected,
        "REPRODUCIBLE_BACKENDS 应恰为手写的五项，集合相等且个数相等（§3.5／§10.1）"
    );
}

/// 设计 §10.2 第 (d) 条（**两个子例，两条都要**）：fail-open 的那一侧（纪律 7）。
///
/// （i）造一枚 `determinism == Deterministic` 而候选含名单外 backend
/// （`BackendId::new("remote-cluster")`）的算子，连同若干合法算子一次传进去 ⇒
/// `Err(CodeError::UnverifiableDeterminism { operator, backend })`，
/// **且判准 `operator` 与 `backend` 两个字段的值**。
///
/// （ii）**同一调用之后，注册表内容与调用前逐枚相同**（先核后写，不留半注册）。
/// 只钉（i）会让「注册表被写坏了一半」漂过去——（ii）就是 fail-open 的那一侧。
///
/// 红条件一（**取反档**）：把名单判定改成恒真 ⇒（i）红（没有 `Err` 可判）。
/// 红条件二（**取反档**）：把函数改成边核边写（先注册前几枚、遇到违规才返回）⇒
/// （ii）红而（i）**仍绿**——**（i）为什么仍绿**：「边核边写」不改返回的 `Err` 值
/// （仍是 `UnverifiableDeterminism { operator, backend }`；批次里违规枚之前的合法算子
/// 都能注册成功、没人撞车），故（i）的两个字段判据照过；设计 §10.3 那一行也只预告
/// （ii）那一个子例红、并明写「不该红 c、a」。
/// **若（i）在这一档下也红了，说明两件不相干的事被混进了一条断言。**
#[test]
fn an_operator_with_an_unlisted_backend_is_rejected_before_any_write() {
    let operators = all_operators();

    // 违规枚：id 故意取一个不在九枚里的串（id 的取值域不由本块封闭，见 (e) 条用例）；
    // 声明 Deterministic，候选只有名单外的 remote-cluster。
    // 本条规则只读 determinism 与 backend_candidates，其余字段不参与判定。
    let rogue = Operator {
        id: OperatorId::new("code-remote-test"),
        version: OperatorVersion::new(1),
        input_schema: vec![],
        output_schema: vec![ArtifactType::Json],
        determinism: Determinism::Deterministic,
        side_effect_class: SideEffectClass::Pure,
        backend_candidates: vec![BackendId::new("remote-cluster")],
    };

    // 调用前：表里有一枚既有算子（第 2 行 code-plan，不在批次里）——
    // 核失败的调用不许动它。
    let mut registry = OperatorRegistry::new();
    registry
        .register(operators[1].clone())
        .expect("基线注册（第 2 行 code-plan）应成功");

    // 批次：合法 → 合法 → 违规 → 合法。违规枚**后面**还放一枚合法，钉的是「不写」
    // 而不是「写到违规枚为止就不写了」；违规枚**前面**的两枚给红条件二留着——
    // 「边核边写」的实现会先把它们写进表里，于是（ii）红。
    let batch = vec![
        operators[6].clone(),
        operators[8].clone(),
        rogue.clone(),
        operators[0].clone(),
    ];

    let err = register_code_operators(&mut registry, &batch)
        .expect_err("含名单外 backend 的批次应被整批拒绝（§10.2 第 (d) 条）");

    // （i）两个字段的值都要判准。
    match err {
        CodeError::UnverifiableDeterminism { operator, backend } => {
            assert_eq!(operator, rogue.id, "错误里的 operator 应是违规的那一枚");
            assert_eq!(
                backend,
                BackendId::new("remote-cluster"),
                "错误里的 backend 应是名单外的那一个"
            );
        }
        other => panic!("错误臂应为 CodeError::UnverifiableDeterminism，实际为 {other:?}"),
    }

    // （ii）同一调用之后，注册表内容与调用前逐枚相同。
    // 方向一：调用前那一枚仍在、逐字段未变。
    let baseline = registry
        .resolve(&operators[1].id, &operators[1].version)
        .expect("调用前已注册的那一枚在核失败后应仍在表里");
    assert_eq!(baseline, &operators[1], "核失败不得改动调用前已有的条目");
    // 方向二：批次里一枚都没进去——不留半注册。
    for op in &batch {
        assert!(
            matches!(
                registry.resolve(&op.id, &op.version),
                Err(OperatorError::NotFound { .. })
            ),
            "{} 不应出现在注册表里（先核后写，核失败不留半注册）",
            op.id
        );
    }
}

/// 设计 §10.2 第 (e) 条：用表外的 `OperatorId`（`codew`）造一枚候选全在名单内的
/// `Deterministic` 算子，与合法算子混合注册 ⇒ 逐枚成功。
///
/// 注释里写明：`OperatorId` 的取值域**不由本块封闭**
/// （`crates/continuum-operator/src/definition.rs:24-31` 是有字段的 newtype，**无枚举**），
/// 本块**不发明**一个 id 白名单。
/// **这一条是上一条（d）的另一侧**：否则 (d) 可以由「一律拒」满足——那是一条把合法
/// 输入也拒掉的假守卫。
///
/// 红条件（**移除档**）：给 id 加一层白名单校验 ⇒ 本条红（`codew` 被拒）。
#[test]
fn an_operator_with_an_id_outside_the_table_is_accepted() {
    let operators = all_operators();

    // 表外的算子：id 取 "codew"；声明 Deterministic 而候选全在名单内（builtin）——
    // 它正是 (d) 的对照面：规则只看候选，不看 id 是不是九枚之一。
    let outside = Operator {
        id: OperatorId::new("codew"),
        version: OperatorVersion::new(1),
        input_schema: vec![],
        output_schema: vec![ArtifactType::SourceTree],
        determinism: Determinism::Deterministic,
        side_effect_class: SideEffectClass::Pure,
        backend_candidates: vec![BackendId::new("builtin")],
    };

    // 混合：第 7 行（合法）→ 表外 → 第 9 行（合法）。
    let batch = vec![operators[6].clone(), outside.clone(), operators[8].clone()];

    let mut registry = OperatorRegistry::new();
    register_code_operators(&mut registry, &batch)
        .expect("表外 id 的算子不得被拒——本块不发明 id 白名单（§10.2 第 (e) 条）");

    // 逐枚成功（含表外的那一枚）。
    for op in &batch {
        assert!(
            registry.resolve(&op.id, &op.version).is_ok(),
            "{} 应能 resolve 成功（混合注册逐枚成功）",
            op.id
        );
    }
}
