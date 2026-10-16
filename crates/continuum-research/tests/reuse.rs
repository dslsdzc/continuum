//! §305 的复用照片（设计 §9.2 第 (k) 条、§6.1）：**本块不建第二份复用判据**。
//!
//! §131 的兑现**不是本块的一个函数**，而是**一枚算子的 `determinism` 声明**——判据本身
//! 已由 P1 落地（`crates/continuum-graph/src/reuse.rs` 的 `cache_key` / `can_reuse`），
//! 本文件只核对「本块声明的 `determinism` 真能让那条判据成立」——**不另写复用函数**
//! （切分 §四第 7 条：判据只许有一处）。失效方向：两份判据并存且各有用例、各自都绿时，
//! 失配是「一套说能复用、另一套说不能」——而这种不一致**不会在任何一条用例里显形**。
//!
//! **`the_cache_key_is_not_a_second_reuse_judgement`：本计划不建这条用例**（切分 §四第 7 条
//! 的另一半）。「本 crate 里没有第二份复用判据」这件事**在本块内没有可写的运行期断言**：
//! 本块不定义任何可与第二份判据比对的函数面，而语料只能从被测清单里取（纪律 9「正控制不得
//! 自证」），故照这样写出来的用例只会是一条**恒绿的空用例**——比没有更坏。**照片改由
//! Task 7 Step 3 的源码面复核（人工）承担**（`src/` 里 `can_reuse` / `cache_key` 等五枚
//! 名字一枚都不重写）。**这一处照的是 P4 对同形情况的取法**：
//! `docs/superpowers/plans/2026-10-06-p4-semantic-layer.md` 的
//! `the_projection_is_value_preserving_dimension_by_dimension` 一条下判定「**不建**」那条
//! 读源码的自定义检查，只把它加进人工复核清单——**P4 取的是「不建」，不是「建一条不写
//! 断言的用例」**。**这一处只有人工照片的代价记在**
//! `docs/superpowers/plans/2026-10-10-p5d-research-domain-operators.md` 的 `## 遗留`
//! 第三节第 10 条。
//!
//! **照片坐标**：`continuum-graph` 是本 crate 的 **dev 边**（生产代码不调它）；
//! 这条边的使用点就是本文件（设计 §2.1 的 `continuum-graph` 行、§2.1 末段）。

use continuum_artifact::ContentHash;
use continuum_graph::{CacheKey, cache_key, can_reuse};
use continuum_operator::{Operator, OperatorId, OperatorVersion};
use continuum_research::all_operators;

/// 按 id 从 `all_operators()` 里取本块声明的那一枚算子。
/// 取不到即 panic：用例的语料行与七枚清单对不上时，宁可红也不静默跳过。
fn declared(id: &str) -> Operator {
    all_operators()
        .into_iter()
        .find(|operator| operator.id == OperatorId::new(id))
        .unwrap_or_else(|| panic!("all_operators() 里没有 {id}"))
}

/// §9.2 第 (k) 条（§6.1 的六行**逐行**，不是抽样）：一（且仅一）枚为 `Some`，
/// 六枚为 `None`——**两侧都钉**。
///
/// （i）对 `research-source-set` 断言 `cache_key(&op, h)` 是 `Some(k)` 且
/// `k.operator_version == OperatorVersion::new(1)`、`k.input_hash == h`；
/// 对**另外六枚逐枚**断言 `cache_key(&op, h).is_none()`（**逐枚六条，不抽代表**）。
/// （ii）`can_reuse` 的四条件齐时（`cached = Some(&k)`、`current_input_hash = h`、
/// `contract_affected = false`）为真。
///
/// **语料手写**：七个 id 与逐枚的期望取自设计 §4.2 的表（唯一事实来源），不从实现侧取——
/// 若把期望写成按 `all_operators()` 自己遍历再计数（「非 `Deterministic` 的枚数等于 6」），
/// 它测的是一次减法（设计 §9.2 的「一条预防」）。
///
/// **本条的作用域（照实写明）**：本块**不建第二份复用判据**（切分 §四第 7 条），
/// 本条的兑现**不是本块的一个函数**，而是**一枚算子的 `determinism` 声明**——
/// `cache_key` 读的正是 `operator.determinism`（`reuse.rs:17`）而**不看 backend**
/// （即 P5e 设计 §5.4 记的那处结构缺口；本块的处置见设计 §6.2 与 `## 遗留`）。
///
/// 红条件（**取反档**）：把第 3 枚的 `determinism` 由 `Deterministic` 改成
/// `NonDeterministic`（锚点：`src/operators.rs` 中 `research-source-set` 那一枚的
/// `determinism:` 字段值——`Deterministic` 在七枚里只出现**一次**，故唯一）
/// ⇒ 那一枚由 `Some` 变 `None` ⇒ 本用例红（设计 §9.3 第 1 行）。
/// 红条件（**取反档，另一侧**）：把第 2 枚的 `determinism` 由 `NonDeterministic`
/// 改成 `Deterministic`（锚点：**`research-search` 那一枚那个结构体字面量里**的
/// `determinism:` 行——`NonDeterministic` 在七枚里出现**六次**，故定位靠它的毗邻串
/// `OperatorId::new("research-search")`，不靠这一行本身）⇒ 该枚由 `None` 变 `Some`
/// ⇒ 本用例红（设计 §9.3 第 2 行）。
/// **两条红条件的兑现都在 Task 7 Step 5 的变异轮**（第 3 枚是那一轮的 (iii-a)、
/// 第 2 枚是 (iii-b)）——**两侧都要跑**：这一条两侧对钉的守卫只跑一侧等于没钉（纪律 6）。
///
/// **据实记**：本条不是「先假后真」型的用例——Task 1 的 `determinism` 已写对，
/// 它是**声明与既有判据之间的一致性照片**，价值在变异轮（上两条红条件）显现
/// （设计 §9.5 第 3 条：§305 的复用今天无生产调用方，`CacheKey` 无存储，
/// 故只能拍到判据的返回值）。
#[test]
fn only_the_source_set_yields_a_cache_key_and_the_other_six_do_not() {
    // 手写的输入哈希：正组四条件之一（输入未变）由此成立。
    let h = ContentHash::of(b"research-input");

    // 第 3 枚（设计 §4.2 第 3 行）：research-source-set 声明 Deterministic
    // ⇒ cache_key 为 Some，且键的两格逐位对上手写期望。
    let source_set = declared("research-source-set");
    let key: CacheKey = cache_key(&source_set, h.clone())
        .expect("research-source-set 声明 Deterministic，cache_key 应为 Some");
    assert_eq!(
        key.operator_version,
        OperatorVersion::new(1),
        "缓存键的 operator_version 应取算子的 version（设计 §4.2 末：七枚一律为 1）"
    );
    assert_eq!(
        key.input_hash, h,
        "缓存键的 input_hash 应逐位等于传入的输入哈希"
    );

    // 正组第二半：四条件齐（determinism ✓、输入哈希未变 ✓、版本未变 ✓、contract 未受影响 ✓）
    // ⇒ can_reuse 为真。
    assert!(
        can_reuse(&source_set, Some(&key), &h, false),
        "research-source-set：四条件齐时应判可复用（§305 与 ENG-002 的落地点）"
    );

    // 另外六枚**逐枚**断言 None（不抽代表）。每枚按设计 §4.2 的行序、逐行手写。
    // 第 1 行：research-question 声明 NonDeterministic ⇒ None。
    let question = declared("research-question");
    assert!(
        cache_key(&question, h.clone()).is_none(),
        "research-question 声明 NonDeterministic，cache_key 应为 None"
    );

    // 第 2 行：research-search 声明 NonDeterministic ⇒ None。
    let search = declared("research-search");
    assert!(
        cache_key(&search, h.clone()).is_none(),
        "research-search 声明 NonDeterministic，cache_key 应为 None"
    );

    // 第 4 行：research-evidence-extraction 声明 NonDeterministic ⇒ None。
    let evidence_extraction = declared("research-evidence-extraction");
    assert!(
        cache_key(&evidence_extraction, h.clone()).is_none(),
        "research-evidence-extraction 声明 NonDeterministic，cache_key 应为 None"
    );

    // 第 5 行：research-contradiction-check 声明 NonDeterministic ⇒ None。
    let contradiction_check = declared("research-contradiction-check");
    assert!(
        cache_key(&contradiction_check, h.clone()).is_none(),
        "research-contradiction-check 声明 NonDeterministic，cache_key 应为 None"
    );

    // 第 6 行：research-synthesis 声明 NonDeterministic ⇒ None。
    let synthesis = declared("research-synthesis");
    assert!(
        cache_key(&synthesis, h.clone()).is_none(),
        "research-synthesis 声明 NonDeterministic，cache_key 应为 None"
    );

    // 第 7 行：research-citation-verification 声明 NonDeterministic ⇒ None。
    let citation_verification = declared("research-citation-verification");
    assert!(
        cache_key(&citation_verification, h.clone()).is_none(),
        "research-citation-verification 声明 NonDeterministic，cache_key 应为 None"
    );
}
