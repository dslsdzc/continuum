//! 设计 §7.1 目录条目的用例（§10.2 的 k／l／m／n）＋ (n) 的运行期补件。
//!
//! 本文件钉四类事，缺一条即留一侧静默错误：
//! - **(k)** 七条 `software/` 的 `realized_by` **逐条等于** §7.1 表——五条非空，
//!   `TDD` 与 `debugging` **为空**（协调者 2026-10-10 的裁定；写成「非空即过」
//!   会把那条裁定当场抹掉）；
//! - **(l)** 五条非空里的**每个串**都能 `resolve`——`bind` 的文本弱引用的
//!   **运行期补件**（P5b 设计 §3.2 末：这条弱引用在 P5b 自己的 crate 里没有照片）；
//! - **(m)** **手写的**八个算子 id 全被目录引用、`code-implement` 不被引用
//!   ——§7.1 与 §3.2 两张表之间的逐枚对应；
//! - **(n)** `bind_code_methods` 的返回类型是 `Result<(), MethodError>`——
//!   **编译期照片**（照实写成编译期照片，不冒充运行期红；纪律 1(c)）。
//! - **(n 的运行期补件)** `MethodRegistry::new()` 空表 ⇒ `bind_code_methods` 返回
//!   `Err(MethodError::NotFound { id })`，且失败点是第一个 `bind` 的 `planning`——
//!   失败路径**判别哪一种 `Err`**（实现纪律第 6 条）；钉的是 **`?` 传播**。
//!
//! (o)（复用）与 (p)（backend 名）不在这里——它们分别由 `tests/reuse.rs` 与
//! `tests/workspace_backends.rs` 承担；(a)–(e) 在 `tests/register.rs`、(j) 在
//! `tests/schema_edges.rs`。

use continuum_code::{all_operators, bind_code_methods, register_code_operators};
use continuum_method::{MethodDomain, MethodError, MethodId, MethodRegistry};
use continuum_operator::{Operator, OperatorRegistry, OperatorVersion};

/// 在 `all_operators()` 里按 `id` **线性查找**；**找不到即 `panic!` 并打印那个串**。
///
/// 形态与 `tests/schema_edges.rs` 的 `operator_by_id` **同形**（Task 3 的语料同样按串
/// 指认算子）：**不许写成「找不到就跳过」**——那样 Task 1 的 id 变异（把 §3.2 表第 5、6
/// 行的 id 改成同一个）会让本文件以**静默通过**而非红收场。写成 panic 之后，那一处
/// 变异在这里的红是「找不到 `code-quality-review`」这一条 panic——**红的位置可解释**。
fn operator_by_id(id: &str) -> Operator {
    all_operators()
        .into_iter()
        .find(|op| op.id.as_str() == id)
        .unwrap_or_else(|| panic!("all_operators() 里找不到 id 为 `{id}` 的算子"))
}

/// 设计 §10.2 第 (k) 条：七条 `software/` 目录条目的 `realized_by` **逐条**等于 §7.1 表。
///
/// **期望值手写**（不是「非空即通过」）：P5b 判据 6 的形态是「非空」，而本块**有意**
/// 让 `TDD` 与 `debugging` 两条为空（P5c 设计 §7.2 报出、P5b 设计第十节判据 6 的
/// 2026-10-10 裁定）——写成「非空即过」会把那条裁定当场抹掉。语料不从 `registry`
/// 派生——那是恒真的假照片。
///
/// 红条件（**取反档**）：把 `src/methods.rs` 里 `TDD` 那一行的 `&[]` 改成
/// `&["code-implement", "code-test-run"]` ⇒ 本条红（`TDD` 一行的期望是**空**）。
/// **同时预告（写进注释）**：那一条变异下，下一条 `(l)` **仍绿**——因为那两个 id
/// 都能 `resolve`。**若 (l) 也红了，说明 (l) 不是按 (k) 的方式写的**（(l) 必须只对
/// **非空**行取串，且不能对「多出来的那两枚」作任何断言）。
#[test]
fn every_software_entry_has_the_declared_realized_by() {
    let mut registry = MethodRegistry::seeded();
    bind_code_methods(&mut registry).expect("对 seeded() 的七条 software/ 条目 bind 应全部成功");

    // §7.1 表的七行期望值，**手写**（行序即 P5c 设计 §7.1 表的行序）。
    let expected: [(&str, &[&str]); 7] = [
        // §186 `:62` 的 `Implementation Plan` 与 `:64` 的 `Engineering Tasks` 两步都是规划。
        ("planning", &["code-plan", "code-decompose"]),
        // `TDD`：次序约束、无算子对家——**空**（裁定；理由见 §7.2 第 1 条）。
        ("TDD", &[]),
        // `debugging`：链上无此步、无算子对家——**空**（裁定；理由见 §7.2 第 2 条）。
        ("debugging", &[]),
        // §16 的 git worktree。
        ("worktree", &["code-workspace"]),
        // §186 `:85` 逐字「规格审查与代码质量审查分离」。
        ("review", &["code-spec-review", "code-quality-review"]),
        // §193 `:368-369` 的 Fuzzing 与 Property-based Testing。
        ("fuzz", &["code-fuzz-run"]),
        // §186 `:74` 的 Verification；两枚的分工见设计 §3.3 第 5、11 条。
        ("verification", &["code-test-run", "code-verify"]),
    ];

    for (id, realized_by) in expected {
        let entry = registry
            .resolve(&MethodId::new(id))
            .unwrap_or_else(|err| panic!("{id} 应在 seeded() 的 software/ 目录里，实际为 {err:?}"));

        let actual: Vec<&str> = entry.realized_by.iter().map(String::as_str).collect();
        assert_eq!(
            actual.as_slice(),
            realized_by,
            "{id} 的 realized_by 应**逐条等于** §7.1 表（不是「非空即通过」；\
             `TDD` 与 `debugging` 两条按裁定为空）"
        );
    }
}

/// 设计 §10.2 第 (l) 条：五条非空 `realized_by` 里的**每个串** `resolve` 成功。
///
/// **这一条是本块的补件**：`bind` 的参数是**文本**，本块无法在编译期核对它与
/// `all_operators()` 的 id 一致（P5b 设计 §3.2 末已把这条弱引用的无照片写清）；
/// 本块同时持有 `all_operators()` 与 `MethodRegistry`，故这条核对在**运行期**闭得上。
///
/// 红条件（**取反档**）：把 `src/methods.rs` 里 `fuzz` 一行的 `code-fuzz-run` 改成
/// `code-fuzz-runs` ⇒ 本条红（`operator_by_id` 的 panic，或 resolve 失败），**且 (k) 红**
/// （(k) 的期望里写的是正确的串）——**两条同时红是预期**：k 钉「等于表」，l 钉「表里的
/// 文本指向真实算子」，同一个串写错会同时撞两条。
#[test]
fn every_id_in_a_nonempty_realized_by_resolves() {
    // 先把 `&all_operators()` 注册进一个 `OperatorRegistry`（Task 2 的入口）。
    let mut operators = OperatorRegistry::new();
    register_code_operators(&mut operators, &all_operators())
        .expect("九枚合法算子一次注册应成功");

    let mut methods = MethodRegistry::seeded();
    bind_code_methods(&mut methods).expect("对 seeded() 的七条 software/ 条目 bind 应全部成功");

    // 五条非空行的 method id（手写，行序即 §7.1 表；`TDD` 与 `debugging` 是空行，不在其中）。
    for method_id in ["planning", "worktree", "review", "fuzz", "verification"] {
        let entry = methods.resolve(&MethodId::new(method_id)).unwrap_or_else(|err| {
            panic!("{method_id} 应在 seeded() 的 software/ 目录里，实际为 {err:?}")
        });

        // 把「非空」这一前提显式钉住：空行没有串可查、循环体零次执行——若某一条被留空，
        // 本条会以一条可解释的红收场（而不是只有 (k) 红、本条静默绿）。
        assert!(
            !entry.realized_by.is_empty(),
            "{method_id} 是非空行（§7.1 表），其 realized_by 不应为空——\
             空意味着漏 bind 或采用了错误的裁定（§10.2 第 (l) 条的前提）"
        );

        for s in &entry.realized_by {
            // ① 按串指认算子：在 `all_operators()` 里线性查找，找不到即 panic 并打印那个串
            //    （同形查找；不许「找不到就跳过」）。这同时钉住「文本 s 与 all_operators()
            //    的 id 逐字一致」——本条的补件面。
            let operator = operator_by_id(s);
            // ② 该 (id, version 1) 在注册表里 resolve 成功（version 1 是 §3.2 的不变量）。
            operators
                .resolve(&operator.id, &OperatorVersion::new(1))
                .unwrap_or_else(|err| {
                    panic!(
                        "{s} 所在的条目（{method_id}）绑的是 `{s}`，它在已注册九枚算子的\
                         注册表里应 resolve 成功，实际为 {err:?}（§10.2 第 (l) 条）"
                    )
                });
        }
    }
}

/// 设计 §10.2 第 (m) 条：八个手写的算子 id **都**出现在某个非空 `realized_by` 里，
/// **且 `code-implement` 不出现在任何一条里**。
///
/// **语料必须手写**：若写成「遍历 `all_operators()`、再对每枚去 `realized_by` 里找」，
/// 它测的是「每个算子都在某个 `realized_by` 里」——语料从被测清单里取就是**恒真的假
/// 照片**。本条的两份料各有独立来源：八个 id 来自 **§3.2 的算子表视角**（九枚里除
/// `code-implement` 的那八个，§10.2 第 (m) 条逐字列出），被查集合来自
/// `bind_code_methods` 的产出。`code-implement` 不被任何 `realized_by` 引用
/// **不构成缺陷**：§186 `:68` 的 `Implement` 是链上的一步，而 §187 的 `software/`
/// 七条是**方法名**（「怎样做得可靠」），两者不是同一张表（设计 §7.3 已就 `review`
/// 一侧写过同一条）。这一枚是**刻意**的——它使「算子集 ⊋ 目录条目的对家集」这件事
/// 在一条断言里可见。
///
/// 红条件（**取反档**）：把 §7.1 表里 `planning` 一行填错成 `["code-implement"]`——
/// 若实现与 (k) 的手写期望都**照表**（表错、两处同源错），则 (k) **仍绿**（那一行仍是
/// 非空的：非空形态的检查抓不到「填错但非空」），而**本条红**（八个 id 里的 `code-plan`
/// 与 `code-decompose` 不再被任何 `realized_by` 引用）。**这一对红绿是本条存在的理由**
/// ——本条的语料来源与 §7.1 表独立，正是为此留的。若某一轮只改实现一处（(k) 的期望
/// 不动），(k) 也会红——那不是矛盾，是「实现与表不一致」的另一种红法。
#[test]
fn exactly_the_eight_declared_operators_are_referenced_by_the_directory() {
    // 手写八个 id 的名单（§10.2 第 (m) 条逐字列出；行序同 §3.2 表）。
    let eight = [
        "code-workspace",
        "code-plan",
        "code-decompose",
        "code-spec-review",
        "code-quality-review",
        "code-test-run",
        "code-fuzz-run",
        "code-verify",
    ];

    let mut methods = MethodRegistry::seeded();
    bind_code_methods(&mut methods).expect("对 seeded() 的七条 software/ 条目 bind 应全部成功");

    // 被查集合：`software/` 目录里全部非空 `realized_by` 的串（从被测对象取；料在上不在下）。
    let mut bound: Vec<&str> = Vec::new();
    for entry in methods.select(MethodDomain::Software) {
        bound.extend(entry.realized_by.iter().map(String::as_str));
    }

    for id in eight {
        // 按串指认算子（同形查找：找不到即 panic 并打印那个串）；这一句同时钉住
        // 「手写的 id 是真实算子的 id」——Task 1 的 id 变异或移除变异在这里以
        // 「找不到 `code-quality-review`／`code-fuzz-run`」红，位置可解释。
        let operator = operator_by_id(id);
        assert!(
            bound.contains(&operator.id.as_str()),
            "八个 id 之一 `{id}` 应出现在某个非空 realized_by 里（§10.2 第 (m) 条）"
        );
    }

    // `code-implement` 不出现在任何一条里——刻意，理由见本用例的文档注释。
    assert!(
        !bound.contains(&"code-implement"),
        "`code-implement` 不应出现在任何 realized_by 里：§186 的 `Implement` 是链上的一步，\
         不是 §187 的 software/ 方法名（这一枚是刻意的，使「算子集 ⊋ 对家集」可见）"
    );
}

/// 设计 §10.2 第 (n) 条：`bind_code_methods` 的返回类型是 `Result<(), MethodError>`
/// ——**编译期照片**（不是运行期红）。
///
/// 判据：`MethodError::NotFound` 的判定是「方法库里有没有这条方法」（`MethodRegistry`
/// 的表），与 `CodeError` 的两臂（§3.2 的 `UnverifiableDeterminism` / `Registry`）
/// **不同源**，故**直接透出、不并进 `CodeError`**。这与把 `OperatorError::Duplicate`
/// 包进 `CodeError::Registry` 是**同一条判据的两面**——判定是同一件事则合并，是两件事
/// 则分开。
///
/// 红条件（**取反档**）：把返回类型改成 `Result<(), CodeError>` ⇒ **编译不过**
/// （**错误码取自实跑**：`error[E0277]`——lib 里的 `?` 无法把 `MethodError` 转进
/// `CodeError`（`From<MethodError>` 未实现），lib 先拒、到不了本用例的类型标注那一步；
/// 这一步是**编译期照片**，不冒充运行期红；纪律 1(c)：变异导致编译失败时判据是
/// `could not compile` 或 `error[E….`）。
#[test]
fn bind_returns_the_method_error_type() {
    let mut registry = MethodRegistry::seeded();
    // 下面这一行的类型标注就是照片：类型不符即编译失败。
    let _: Result<(), MethodError> = bind_code_methods(&mut registry);
}

/// **失败路径要断言是哪一种 `Err`**（实现纪律第 6 条）：`bind_code_methods` 的
/// `MethodError::NotFound` 失败路径在**运行期可达**——`MethodRegistry::new()` 是空表，
/// 七个 `bind` 的第一个（`planning`）即返回 `Err(NotFound)`。
///
/// 这条钉的是 **`?` 传播**：把首个 `?` 改成 `let _ =`（吞掉错误），错误便从下一行
/// （`TDD`）浮出，本条读到的 id 就不是 `planning` ⇒ 红。
/// `(n)` 只拍了返回类型（编译期），本条补的是**运行期**的那一半。
#[test]
fn binding_into_an_empty_registry_reports_the_first_missing_id() {
    let mut registry = MethodRegistry::new();
    let err = bind_code_methods(&mut registry)
        .expect_err("空表上 bind 七条 `seeded()` 的 id 必失败（`MethodError::NotFound`）");
    assert!(
        matches!(&err, MethodError::NotFound { id } if id.as_str() == "planning"),
        "首个失败点应是第一个 `bind` 的 id `planning`（fail-fast），实际为 {err:?}"
    );
}
