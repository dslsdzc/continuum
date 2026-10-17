//! `bind_research_methods`——`research/` 四条方法的 `realized_by`（§9.2 的 (f)(g)(h)）。
//!
//! 三条用例的语料**全部手写**：四条 `realized_by` 的期望表照设计 §7 的表逐行照录，
//! **不从 `all_operators()` 的 id 推出**——凡从被测清单里取语料的断言都是恒真的
//! 假照片（设计 §9.2 末「一条预防」的同一条判据已写在 `tests/catalog.rs` 文件头）。
//!
//! 四条用例里 (f) 与本轮新增的「空表 `?` 传播」钉本块的产物；另两条的射程照实标注：
//! (g) 钉的是 **P5b 的 `seeded()`** 在 `research/` 域恰四条、(h) 钉的是 **P5b 的
//! `bind`** 的错误臂——两条的射程见各自注释（实现计划 `## 遗留` 第一节第 3、4 条）；
//! 第四条用例（文件末）钉 `bind_research_methods` 的 `?` 传播（终审 I-1）。

use continuum_method::{MethodDomain, MethodError, MethodId, MethodRegistry};
use continuum_operator::{OperatorId, OperatorRegistry, OperatorVersion};
use continuum_research::{all_operators, bind_research_methods, register_research_operators};

/// §9.2 第 (f) 条：`MethodRegistry::seeded()` 起一个注册表、`OperatorRegistry` 注册
/// 七枚（否则解析必失败）、调 `bind_research_methods` ⇒ `Ok`；随后**按手写表逐条**
/// `resolve(&MethodId::new(名))`，断言 `realized_by` **逐位等于**表里那一条
/// （**不是「非空」**——只断非空会让「填错成别的算子、但那个算子恰好存在」静默通过），
/// 且 `realized_by` 的**每个串**在 `OperatorRegistry::resolve` 上以
/// `OperatorVersion::new(1)` 成功。
///
/// **这条是 `realized_by` 文本弱引用的运行期补件**（P5b 设计 §3.2 末）：`realized_by`
/// 是 `Vec<String>`，**块间是文本弱引用、没有编译期照片**——写错的串照样编得过。
/// 本用例是它的**运行期补件**，且它**在本 crate 内闭得上**，因为本块同时持有
/// `all_operators()` 与 `MethodRegistry`：「这个串对应的算子存在吗」这一问，
/// 只有同时看得见两张表的地方才问得出（设计 §7 末）。
///
/// 红条件（**移除档**）：删掉 `bind_research_methods` 里 `retrieval` 那一次 `bind`
/// （锚点：那一次调用的 `MethodId` 字面量 `MethodId::new("retrieval")`——该字面量在
/// 四处 `bind` 里只出现一次，故唯一）⇒ `retrieval` 的 `realized_by` 为空
/// ⇒ 逐位相等红（长度先红）。
/// 红条件（**取反档**）：把 `"research-source-set"` 写成 `"research-source-set "`
/// （多一个尾空格；锚点：`bind_research_methods` 里 `retrieval` 那一次 `bind`
/// 的第二个实参列表中**那一枚**字面量——该串在四处 `bind` 里只出现**一次**，故唯一）
/// ⇒ 逐位相等的**第 2 位**红，**且** `resolve` 得 `Err(NotFound)` ⇒ 本用例红。
#[test]
fn every_research_method_is_bound_to_the_four_handwritten_lists() {
    // 设计 §7 表的四条 `realized_by`，**逐行手写照录**——对照物是设计原文的表，
    // 不是 `all_operators()` 的 id（顺序与 §7 表的行序一致）。
    const HANDWRITTEN: [(&str, &[&str]); 4] = [
        ("retrieval", &["research-search", "research-source-set"]),
        ("evidence-analysis", &["research-evidence-extraction"]),
        ("contradiction-check", &["research-contradiction-check"]),
        ("citation-verification", &["research-citation-verification"]),
    ];

    // seeded() 起 P5b 的方法库（四条 id 全部已在里面）；七枚算子注册进算子表——
    // 否则下面的逐串核对必失败（realized_by 的串要经 OperatorRegistry::resolve 才验得出）。
    let mut methods = MethodRegistry::seeded();
    let mut operators = OperatorRegistry::new();
    register_research_operators(&mut operators, &all_operators())
        .expect("以 &all_operators() 注册七枚应成功");

    assert_eq!(
        bind_research_methods(&mut methods),
        Ok(()),
        "四条 id 全部已 seed，bind_research_methods 必须返回 Ok"
    );

    for (method_id, expected) in HANDWRITTEN {
        let entry = methods
            .resolve(&MethodId::new(method_id))
            .unwrap_or_else(|e| panic!("方法 {method_id} 已在 seeded() 的目录里，应能解析，实际 {e}"));

        // 逐位相等（不是「非空」）：先钉位数，再逐位核内容——
        // 「填错成别的算子」与「多一位/少一位」都会在这里红。
        assert_eq!(
            entry.realized_by.len(),
            expected.len(),
            "{method_id} 的 realized_by 位数应恰为 {}（逐位相等，不是非空）",
            expected.len()
        );
        for (i, (got, want)) in entry.realized_by.iter().zip(expected.iter()).enumerate() {
            assert_eq!(
                got.as_str(),
                *want,
                "{method_id} 的 realized_by 第 {} 位",
                i + 1
            );
        }

        // 弱引用的另一半：串逐位对上了，还要确证它在算子表里**真存在**。
        for op_id in &entry.realized_by {
            operators
                .resolve(&OperatorId::new(op_id.as_str()), &OperatorVersion::new(1))
                .unwrap_or_else(|e| {
                    panic!("{method_id} 的 realized_by 串 {op_id} 应能在算子表里解析，实际 {e}")
                });
        }
    }
}

/// §9.2 第 (g) 条：`bind_research_methods` 之后 `select(MethodDomain::Research)`
/// 返回**四条**，且四条 `id` 恰是手写的四个。
///
/// **本条钉的是「P5b 的 `seeded()` 在 `research/` 域恰四条」**——多一枚或漏一枚
/// 都不会静默。**「四条都被 bind 过」不是本条钉的**（那是上一条 (f) 钉的）：
/// P5b 的 `select(MethodDomain)` **返回该领域的全部条目、与 `realized_by` 是否绑过
/// 无关**（`bind` 只改 `realized_by`，不改条目的存否）⇒ 一个 `select` 的数目断言
/// 钉不到「都被 bind 过」。设计 §9.2 第 (g) 条的自陈把这两半写反了，本计划照实
/// 改标作用域（实现计划 `## 遗留` 第一节第 3 条）。
///
/// 期望串按 **`select` 的 id 升序**手写（P5b 设计 §4.5：返回该领域的全部条目、
/// 按 id 升序排列）——故与 §187 的原文次序（`retrieval` 在最前）不同，这不是笔误。
///
/// 红条件（**移除档**，**跨 crate**）：删掉 P5b `seeded()` 里 `research/` 的一条
/// （锚点：`crates/continuum-method` 的 `seeded()` 目录里 **`"contradiction-check"`
/// 那一行**——`research/` 有四条，故要指名是哪一条，不能写「那一条」）
/// ⇒ `select` 返回三条 ⇒ 本用例红。
#[test]
fn the_research_domain_selects_exactly_four_entries() {
    // 四个方法名手写；次序＝select 的 id 升序（不是 §187 的原文次序）。
    const HANDWRITTEN: [&str; 4] = [
        "citation-verification",
        "contradiction-check",
        "evidence-analysis",
        "retrieval",
    ];

    let mut methods = MethodRegistry::seeded();
    bind_research_methods(&mut methods).expect("四条 id 全部已 seed，bind_research_methods 应成功");

    let selected: Vec<&str> = methods
        .select(MethodDomain::Research)
        .iter()
        .map(|entry| entry.id.as_str())
        .collect();
    assert_eq!(
        selected, HANDWRITTEN,
        "select(Research) 必须恰四条、且四个 id 逐位相等（数目手写）"
    );
}

/// §9.2 第 (h) 条：对**未 seed 的** `MethodId::new("research-probe")` 调
/// `MethodRegistry::bind` ⇒ `Err(MethodError::NotFound { id })`，且 `id` 恰是那一个。
///
/// **本条钉的是 P5b 的 `bind` 的错误臂**（实现计划 `## 遗留` 第一节第 4 条）。
/// **`bind_research_methods` 的失败路径另有照片**：它是 `pub fn`、registry 由调用方给，
/// 传空表（`MethodRegistry::new()`）时四个 id 全未 seed ⇒ 第一条 `bind`（`retrieval`）
/// 即 `NotFound` 并经 `?` 原样透出（下一条用例；终审 I-1 补，2026-10-17）。
/// 「本块不新增错误类型」在本块内的照片是**签名**（返回类型里写的是 `MethodError`）＋
/// 那条 `?` 传播用例。`research-probe` 不在 §187 的十八个 id 里 ⇒ 它没有被
/// `seeded()` 登记过 ⇒ 对它的 `bind` 走的正是未命中分支。
///
/// 红条件（**取反档**，**跨 crate**）：把 `MethodRegistry::bind` 的未命中分支改成
/// `Ok(())`（锚点：`crates/continuum-method` 的 `bind` 实现体里
/// `ok_or_else(|| MethodError::NotFound { id: id.clone() })` 那一处）⇒ 本用例红。
#[test]
fn binding_an_unseeded_method_id_is_not_found() {
    let id = MethodId::new("research-probe");
    let mut methods = MethodRegistry::seeded();

    assert_eq!(
        methods.bind(&id, &["research-search"]),
        Err(MethodError::NotFound { id: id.clone() }),
        "对未 seed 的 id 的绑定必须返回 NotFound，且 id 恰是那一个"
    );
}

/// 终审 I-1（2026-10-17）：`bind_research_methods` 的 **`?` 传播**照片。
///
/// 传**空表** `MethodRegistry::new()` ⇒ 四条 id 全未 seed、`bind` 逐个 `NotFound`；
/// 函数在**第一条**（`retrieval`）处经 `?` 返回 ⇒ 必须得到
/// `Err(MethodError::NotFound { id: retrieval })`。
///
/// **本条钉的是 `?` 的传播，不是 P5b 的 `bind` 本身**（后者由上一条钉）：
/// 把 `retrieval` 那次 `bind` 后的 `?` 改成 `let _ =`（吞掉 Err、继续下一条）
/// ⇒ 返回的 `id` 变成 `evidence-analysis` ⇒ 本条红；四处都吞掉 ⇒ 返回 `Ok(())`
/// ⇒ 本条也红。**「断到具体 id」正是为了抓第一种**——只断 `Err(_)` 会漏掉它。
///
/// **订正留痕**：原注释/计划写「四个 id 全已 seed ⇒ `bind_research_methods`
/// 不可能返回 `Err`、本块对它的失败路径没有照片」——**这半错了**：registry 由调用方给，
/// 空表让「已 seed」的前提不成立，函数在本块内**可达 `Err`**（终审 I-1）。
#[test]
fn bind_research_methods_on_an_empty_registry_returns_the_first_not_found() {
    let mut methods = MethodRegistry::new();

    assert_eq!(
        bind_research_methods(&mut methods),
        Err(MethodError::NotFound {
            id: MethodId::new("retrieval")
        }),
        "空表时第一条 bind（retrieval）的 NotFound 应经 `?` 原样透出（不是被吞掉后继续）"
    );
}
