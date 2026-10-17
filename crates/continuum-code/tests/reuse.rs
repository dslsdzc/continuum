//! 设计 §3.6 的照片（§10.2 第 (o) 条）：本块的 `determinism` 声明**让 §305 的判据
//! 成立或落空**——这不是「本块实现了一个复用判据」。
//!
//! §305（`docs/spec/05-normative.md:1986-1998`）的三条件（`input_hash` 未变、
//! `operator_version` 未变、`contract` 未受影响）**已由 P1 落地**：
//! `crates/continuum-graph/src/reuse.rs` 的 `CacheKey`（`:9`）、`cache_key`（`:16`）与
//! `can_reuse`（`:29`）。
//! 本文件**只消费它们**、**不另写一个**（切分 §四第 7 条的第一枚实例）——设计 §3.6 逐字：
//! 本块的兑现「不是一个函数」，而是 §3.2 表里的 `determinism` 那一列；`cache_key` 对
//! `NonDeterministic` 的算子返回 `None`（`reuse.rs:22`），「非确定节点的输出不写缓存键，
//! 因此永远不满足复用前提」。
//!
//! **钉的是两半**（纪律 7：两侧都要）：
//!
//! - `cache_key` 的**逐枚**取值：第 7、9 行（`code-test-run` / `code-verify`）为 `Some`，
//!   第 1、2、3、4、5、6、8 行为 `None`——九枚全断言，不是抽样；
//! - `can_reuse` 的**两侧**：正例（第 7 行配它自己的 `CacheKey`、`input_hash` 未变、
//!   `contract_affected = false` ⇒ `true`）与四条反例（`input_hash` 变 / `operator_version`
//!   变 / `contract_affected = true` / `cached = None` ⇒ 各自 `false`）。
//!
//! **三处逐枚理由的落点**（§3.3）：
//!
//! - 第 1 行 `code-workspace` 的 `None` 是 §3.3 第 3 条的照片（fail-closed 的一侧）：
//!   它的真实输入（Base 的当前工作树）**不在端口上**，`input_hash` 无从取得；若声明
//!   `Deterministic`，`cache_key` 会给出 `Some`，而「`input_hash` 未变」这一条会在**前提
//!   不成立**的地方被当成成立——上游 Base 动过之后，一份**旧的工作区树**会冒充本次的结果。
//!   取 `NonDeterministic` 的代价是每轮重跑工作区创建；与「复用一棵陈旧的工作区树」相比，
//!   取向明确。
//! - 第 7 行的 `Some` 是 §3.3 第 6 条的**收益**（不是缺口）：输入树未变、算子版本未变、
//!   Contract 未受影响时，§305 允许直接复用上一轮的 `TestResult`，不重跑测试套件。
//! - 第 8 行 `code-fuzz-run` 的 `None` 是 §3.3 第 5 条的可观察面之一：fuzz 以熵源生成输入，
//!   故不进复用；若把它与第 7 行合成一枚，那一枚的 `Deterministic` 声明**当场为假**。
//!
//! **红条件（放宽档，唯一锚点）**：把 `crates/continuum-code/src/operators.rs` 里
//! `code-workspace` 那一行的 `determinism` 字段改成 `Deterministic` ⇒ 第 1 行由 `None`
//! 变 `Some`（None 组的第一条断言红），**本用例红**、Task 1 的 `g`
//! （`tests/operators.rs` 的 `the_determinism_of_every_row`）红；**而 Task 2 的 (c)**
//! （`tests/register.rs` 的 `every_deterministic_operator_has_all_its_candidates_in_the_list`）
//! **不红**——`worktree` / `overlay` 两个候选**都在** §3.5 的名单里，注册期那条规则不响。
//! **这正是设计 §3.5 末点名的那两条独立守卫**：§3.5 的规则抓「候选不可复现」，
//! §3.3 第 3 条抓「输入不可见」；缺任一条都留下一类静默错误。
//!
//! **本用例钉不住什么（照实写）**：「§305 的复用真的省下了一次重跑」今天拍不出照片
//! ——`can_reuse` 无生产调用方，且 `CacheKey` 无存储（P1 设计 §18 把它列在「无执行点的
//! 机制」里；计划「拍不到照片的地方」已登记此句）。本用例拍到的只是**判据在声明上的
//! 读数**；本块**不替它建表**（设计 §3.6）。

use continuum_artifact::ContentHash;
use continuum_code::all_operators;
use continuum_graph::{cache_key, can_reuse};
use continuum_operator::OperatorVersion;

/// 设计 §10.2 第 (o) 条。
#[test]
fn every_deterministic_operator_produces_a_cache_key_and_every_other_does_not() {
    let operators = all_operators();
    let input_hash = ContentHash::of(b"reuse-photo: the same tree");

    // 行号语料**手写**（0 基下标）：哪些行是 `Some`、哪些行是 `None` 逐行列出，
    // 不从 `determinism` 遍历派生——从遍历派生测的是「它等于它自己」（纪律 1(b)(ii)）。
    //
    // Some 组：第 7、9 行（0 基下标 6、8）——这两枚 `Deterministic` 进复用；
    // 第 7 行那一枚是 §3.3 第 6 条的收益。
    let some_rows = [6usize, 8];
    for i in some_rows {
        assert!(
            cache_key(&operators[i], input_hash.clone()).is_some(),
            "第 {} 行（{}）的 cache_key 应为 Some（§10.2 第 (o) 条）",
            i + 1,
            operators[i].id
        );
    }

    // None 组：第 1、2、3、4、5、6、8 行（0 基下标 0、1、2、3、4、5、7）——
    // 第 1 行那一枚是 §3.3 第 3 条的照片，第 8 行那一枚是 §3.3 第 5 条的可观察面之一。
    let none_rows = [0usize, 1, 2, 3, 4, 5, 7];
    for i in none_rows {
        assert!(
            cache_key(&operators[i], input_hash.clone()).is_none(),
            "第 {} 行（{}）的 cache_key 应为 None（§10.2 第 (o) 条）",
            i + 1,
            operators[i].id
        );
    }

    // 覆盖守卫：两组下标合起来**恰是九枚**（0..9）。漏写一行会让某一枚不被断言——
    // 那是抽样，不是「逐枚」；重复下标在这里同样红（九个元素盖不住 0..9 的每一枚）。
    let mut covered: Vec<usize> = some_rows.into_iter().chain(none_rows).collect();
    covered.sort_unstable();
    let expected_cover: Vec<usize> = (0..9).collect();
    assert_eq!(
        covered, expected_cover,
        "两组下标合起来必须恰是九枚（逐枚，不是抽样）"
    );

    // `can_reuse` 的**两侧**（纪律 7）。正例与四条反例都用第 7 行（`code-test-run`）：
    // 它是 `Deterministic`，四条件的故事在它身上讲得开。
    let op = &operators[6];
    let cached = cache_key(op, input_hash.clone()).expect("第 7 行（code-test-run）应给出 Some");

    // 正例：配它自己的 `CacheKey`、`input_hash` 未变、`contract_affected = false` ⇒ true。
    assert!(
        can_reuse(op, Some(&cached), &input_hash, false),
        "四条件齐时 can_reuse 应为 true（§10.2 第 (o) 条的正例）"
    );

    // 反例一：`input_hash` 变 ⇒ false（§305 的第一条件「`input_hash` 未变」）。
    let other_hash = ContentHash::of(b"reuse-photo: another tree");
    assert!(
        !can_reuse(op, Some(&cached), &other_hash, false),
        "input_hash 变了，can_reuse 应为 false（§305：「input_hash 未变」）"
    );

    // 反例二：`operator_version` 变——造一枚 version = 2 的同 id 算子 ⇒ false
    // （§305 的第二条件「`operator_version` 未变」）。
    let mut bumped = op.clone();
    bumped.version = OperatorVersion::new(2);
    assert!(
        !can_reuse(&bumped, Some(&cached), &input_hash, false),
        "算子升到 version 2 后旧 CacheKey 不得复用（§305：「operator_version 未变」）"
    );

    // 反例三：`contract_affected = true` ⇒ false（§305 的第三条件「`contract` 未受影响」）。
    assert!(
        !can_reuse(op, Some(&cached), &input_hash, true),
        "Contract 受影响时 can_reuse 应为 false（§305：「contract 未受影响」）"
    );

    // 反例四：`cached = None` ⇒ false——没有上一轮的凭据；非确定节点的输出不写缓存键，
    // 因此永远不满足复用前提（P1 在 `cache_key` 上的内部注释逐字）。
    assert!(
        !can_reuse(op, None, &input_hash, false),
        "无 CacheKey 时不得复用（「非确定节点的输出不写缓存键，因此永远不满足复用前提」）"
    );
}
