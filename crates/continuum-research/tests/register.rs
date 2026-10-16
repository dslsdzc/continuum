//! `register_research_operators` 的用例：§9.2 的 (c)(d)(e) ＋ 本计划补的一条承重夹具。
//!
//! **两条夹具共用的前提**：`Operator` 的七个字段全 `pub`（`definition.rs:80-87`），
//! 故**任意 `Operator` 就地可造**——这正是设计 §4.1「参数带一批算子（不是只吃
//! `all_operators()`）」的用途；(c)(d) 用 `&all_operators()` 本身，两条探针夹具
//! 就地构造 `research-probe` / `research-probe-a`。

use continuum_artifact::ArtifactType;
use continuum_operator::{
    BackendId, Determinism, Operator, OperatorError, OperatorId, OperatorRegistry, OperatorVersion,
    SideEffectClass,
};
use continuum_research::{all_operators, register_research_operators};

/// 就地构造的探针算子：`Operator` 的七个字段全 `pub`（`definition.rs:80-87`），
/// 故任意一枚都能在此直接写出。`(id, version)` 由调用方给，其余字段取一组合法值。
fn probe_operator(id: &str, version: u32) -> Operator {
    Operator {
        id: OperatorId::new(id),
        version: OperatorVersion::new(version),
        input_schema: vec![],
        output_schema: vec![ArtifactType::Json],
        determinism: Determinism::NonDeterministic,
        side_effect_class: SideEffectClass::Pure,
        backend_candidates: vec![BackendId::new("builtin")],
    }
}

/// §9.2 第 (c) 条：以 `&all_operators()` 调一次 ⇒ `Ok`，随后**逐枚**用
/// **手写的七个 id 串 ＋ version 1** `resolve` 成功，且返回的算子与该枚的 id 相等。
///
/// 七个串手写（对照物是设计 §4.2 的表，不是 `all_operators()` 自身）——
/// 若把它们写成遍历 `all_operators()` 再把 id 拿去查，测的是 Rust 的相等性（§9.2 末的预防）。
/// 红条件（**移除档**）：把 `register_research_operators` 的函数体改成直接 `Ok(())`
/// 不写表（锚点：函数体内的注册循环）⇒ 七次 `resolve` 全是 `Err(NotFound)` ⇒ 本用例红。
#[test]
fn a_full_batch_registers_and_every_id_resolves() {
    // 设计 §4.2 表的行序，手写照录。
    const DESIGNED_IDS: [&str; 7] = [
        "research-question",
        "research-search",
        "research-source-set",
        "research-evidence-extraction",
        "research-contradiction-check",
        "research-synthesis",
        "research-citation-verification",
    ];

    let mut registry = OperatorRegistry::new();
    register_research_operators(&mut registry, &all_operators())
        .expect("以 &all_operators() 调一次应成功");

    for id in DESIGNED_IDS {
        let got = registry
            .resolve(&OperatorId::new(id), &OperatorVersion::new(1))
            .unwrap_or_else(|e| panic!("注册过的算子 {id} 应能解析，实际 {e}"));
        assert_eq!(got.id.as_str(), id, "解析回的算子应与该枚的 id 相等");
    }
}

/// §9.2 第 (d) 条：同一批注册两次——第一次 `Ok`；第二次 ⇒
/// `Err(OperatorError::Duplicate { id, version })`，**断言 `id` 恰是批首那一枚**
/// （`research-question`）与 `version == 1`——**不是「返回了 Err」**。
///
/// 红条件（**取反档**）：把实现里「遇重复即报错」改成「遇重复即跳过」⇒
/// 第二次返回 `Ok` ⇒ 本用例红。
///
/// **第二半「表未变」在本夹具下不承重，照实写明**：同一批的**首枚**即撞车，
/// 故「边核边写」的实现也什么都不写、表也变不了 ⇒ 「表未变」那一半**恒真**。
/// **真正的守卫是下一条**（`a_duplicate_after_a_new_operator_leaves_no_half_registration`；
/// `## 遗留` 第一节第 2 条）。
#[test]
fn registering_the_same_batch_twice_returns_duplicate() {
    let operators = all_operators();
    let mut registry = OperatorRegistry::new();

    register_research_operators(&mut registry, &operators).expect("第一次注册应成功");

    match register_research_operators(&mut registry, &operators) {
        Err(OperatorError::Duplicate { id, version }) => {
            assert_eq!(id.as_str(), "research-question", "撞车应恰报批首那一枚");
            assert_eq!(version.as_u32(), 1, "撞车应报 version 1");
        }
        other => panic!("同一批第二次注册必须报 Duplicate，实际 {other:?}"),
    }

    // (d) 的第二半「表未变」：第二次调用后逐枚仍在、且与第一次之后逐枚相同。
    // **照实标明：这一半在本夹具下恒真、不承重**——第二批的首枚即撞车，
    // 「边核边写」也一字不写、表也变不了。能区分「先核后写」与「边核边写」的
    // 那两条夹具是 (e) 与下一条。
    for op in &operators {
        let got = registry
            .resolve(&op.id, &op.version)
            .expect("第二次调用后，表内容应与第一次之后逐枚相同");
        assert_eq!(got, op, "第二次调用后，表内容应与第一次之后逐枚相同");
    }
}

/// **本计划补的承重夹具**（`## 遗留` 第一节第 2 条、第四节）：空表先注册**只含批首那一枚**
/// （`research-question`）的批；再调 `[probe, 批首那一枚]`（`probe` 是 `research-probe`，
/// 就地构造）⇒ `Err(Duplicate)`，**且 `probe` 用 `resolve` 查不到**（`Err(NotFound)`）。
///
/// 红条件（**移除档**）：删掉「先核后写」（改成逐枚 `register` 并在错处返回）⇒
/// `probe` 已写进表 ⇒ 本用例红。
/// **这是本块能区分「先核后写」与「边核边写」的两条夹具里的第二条**（(e) 是第一条），
/// 两条各覆盖「先核」漏掉的一半：**本条覆盖「漏核对既有表」**（撞车的那一枚在表里、
/// 新算子排在它之前），**(e) 覆盖「漏核批内自撞」**（撞车发生在同一批的两枚之间）。
/// **二者不可互换，故两条都要留。**
/// 这里**不是**「本块唯一能区分的夹具」：按 (e) 自己的红条件，(e) 同样能让
/// 「边核边写」露出来（见 `## 遗留` 第一节第 2 条）。
#[test]
fn a_duplicate_after_a_new_operator_leaves_no_half_registration() {
    let mut registry = OperatorRegistry::new();

    // 空表先注册只含批首那一枚（research-question）的批。
    let first = all_operators().into_iter().next().expect("清单非空");
    register_research_operators(&mut registry, std::slice::from_ref(&first))
        .expect("先注册只含批首那一枚的批应成功");

    // 批 = [probe, 批首那一枚]：probe 是全新的，撞车的那一枚排在它之后、且已在表里。
    let probe = probe_operator("research-probe", 1);
    match register_research_operators(&mut registry, &[probe.clone(), first]) {
        Err(OperatorError::Duplicate { id, version }) => {
            assert_eq!(id.as_str(), "research-question", "撞车应报已在表里的那一枚");
            assert_eq!(version.as_u32(), 1);
        }
        other => panic!("撞上既有表的那一枚必须报 Duplicate，实际 {other:?}"),
    }

    // 「不留半注册」的承重断言：probe 排在撞车的那一枚之前，若实现边核边写，
    // 它此刻已经躺在表里了。
    match registry.resolve(&probe.id, &probe.version) {
        Err(OperatorError::NotFound { .. }) => {}
        other => panic!("撞车时整个批次一个字都不应写：probe 应查不到，实际 {other:?}"),
    }
}

/// §9.2 第 (e) 条（**批内自撞**）：空表，批 = `[probe_a, probe_a2]`，两枚的
/// `(id, version)` 相同（就地构造；其余字段不同同样撞车——判定只看 `(id, version)`）
/// ⇒ `Err(Duplicate)` 且 `probe_a` **查不到**。
///
/// 红条件（**移除档**）：同上的「边核边写」⇒ `probe_a` 可查 ⇒ 本用例红。
#[test]
fn a_batch_with_an_internal_collision_registers_nothing() {
    let probe_a = probe_operator("research-probe-a", 1);
    // 同 (id, version)、其余字段不同。
    let mut probe_a2 = probe_operator("research-probe-a", 1);
    probe_a2.determinism = Determinism::Deterministic;

    let mut registry = OperatorRegistry::new();
    match register_research_operators(&mut registry, &[probe_a.clone(), probe_a2]) {
        Err(OperatorError::Duplicate { id, version }) => {
            assert_eq!(id.as_str(), "research-probe-a");
            assert_eq!(version.as_u32(), 1);
        }
        other => panic!("批内两枚同 (id, version) 必须报 Duplicate，实际 {other:?}"),
    }

    match registry.resolve(&probe_a.id, &probe_a.version) {
        Err(OperatorError::NotFound { .. }) => {}
        other => panic!("批内自撞时一枚都不应写进表：probe_a 应查不到，实际 {other:?}"),
    }
}
