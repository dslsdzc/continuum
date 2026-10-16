//! §3.5 的复用照片（设计 §11.2 的 (o)、(p) 条）：**这是 P1 判据的重跑，
//! 不是本块判据的照片**。
//!
//! 复用判据（`cache_key` / `can_reuse`）已在 P1 落地（`crates/continuum-graph/src/reuse.rs`
//! 的 `:16` / `:29`；切分 §四第 7 条的第一枚实例）。本文件**不建第二份判据**——
//! 它只核对「本块声明的 `determinism` 真能让那条判据成立或按预期不成立」（设计 §3.5 末）。
//! 把这句「重跑」写在最前，是**免得复审按与本块别的用例同等的强度读它**：
//! 本块的判据照片在 `tests/resolve_region.rs` / `tests/pixels.rs` 等处；本文件是**声明与
//! 既有判据之间的一致性照片**，价值在变异轮显现（红条件见两条用例的注释）。
//!
//! **失效方向**（设计 §3.5 的理由）：两份判据并存且各有用例、各自都绿时，失配是
//! 「一套说能复用、另一套说不能」——而这种不一致**不会在任何一条用例里显形**。
//!
//! **拍不到的那一半（设计 §11.5 第 5 条）**：本文件拍到的是**判据的返回值**，拍不到
//! 「复用真的省下了一次重算」——`can_reuse` 今天无生产调用方，且 `CacheKey` 无存储。
//!
//! **照片坐标**：`continuum-graph` 是本 crate 的 **dev 边**（生产代码不调它、
//! `src/` 里 `continuum_graph` 零命中）；这条边的使用点就是本文件（计划 Global Constraints；
//! 另一条 `continuum-persist` dev 边的处置见 `tests/lineage.rs` 的文件头）。

use continuum_artifact::ContentHash;
use continuum_graph::{CacheKey, cache_key, can_reuse};
use continuum_image::all_operators;
use continuum_operator::{Operator, OperatorId};

/// 按 id 从 `all_operators()` 里取一枚算子（**按 id 取行、不按下标**——表的重排
/// 是等价变异体）。取不到即 panic：语料与清单对不上时，宁可红也不静默跳过。
fn declared(id: &str) -> Operator {
    all_operators()
        .into_iter()
        .find(|operator| operator.id == OperatorId::new(id))
        .unwrap_or_else(|| panic!("all_operators() 里没有 {id}"))
}

/// §11.2 第 (o) 条（**逐枚两条**——两枚 `Deterministic` 算子各一条）：对
/// `hard-composite` 与 `verify-outside-mask`，`cache_key(op, input_hash)` 为 `Some(..)`，
/// 且四条件齐时（`cached = Some(&key)`、`current_input_hash == key.input_hash`、
/// `contract_affected = false`）`can_reuse` 为真。
/// **这是「有读数」的那一侧**（§11.2 把复用前提成立的那一半交给它）。
///
/// **红条件**：把 `hard-composite` 的 `determinism` 改成 `NonDeterministic`（**收紧档**——
/// `NonDeterministic` 收紧 §305 的复用面；档位口径见计划纪律 1 与 `## 遗留` 第一节第 5 条）
/// → 那一条红，且 `the_three_nondeterministic_operators_never_get_a_cache_key` **仍绿**。
///
/// **`(o)` 与 `(p)` 两条合起来才是这条守卫**（计划纪律 8）：只钉 `(p)` 会让「一律 `None`」
/// 的实现全绿（`can_reuse` 永远为假）；只钉 `(o)` 会让「一律 `Some`」的实现全绿。
///
/// **本用例是 P1 判据的重跑**：`cache_key` / `can_reuse` 是 P1 的函数，本文件零复用函数
/// （判据只许有一处，切分 §四第 7 条）——它读的是 P1 的**判定结果**，不是本块新造的东西。
#[test]
fn the_two_deterministic_operators_get_a_cache_key_and_can_be_reused() {
    // 手写的输入哈希：两枚共用一份——四条件里的「输入未变」由此成立。
    let source = ContentHash::of(b"source-image");

    // 第 4 行：hard-composite（§3.2 表；声明 Deterministic ⇒ Some，四条件齐时可复用）。
    let hard_composite = declared("hard-composite");
    let key: CacheKey = cache_key(&hard_composite, source.clone())
        .expect("hard-composite 声明 Deterministic，cache_key 应为 Some");
    assert!(
        can_reuse(&hard_composite, Some(&key), &source, false),
        "hard-composite：determinism／输入哈希／版本／contract 四条件齐时应判可复用"
    );

    // 第 5 行：verify-outside-mask（同上）。
    let verify_outside_mask = declared("verify-outside-mask");
    let key: CacheKey = cache_key(&verify_outside_mask, source.clone())
        .expect("verify-outside-mask 声明 Deterministic，cache_key 应为 Some");
    assert!(
        can_reuse(&verify_outside_mask, Some(&key), &source, false),
        "verify-outside-mask：四条件齐时应判可复用"
    );
}

/// §11.2 第 (p) 条（**逐枚三枚**——三枚 `NonDeterministic` 算子各一条）：`cache_key` 对
/// `generate-image` / `detect-edit-region` / `local-generative-edit` **逐枚**为 `None`。
/// **这是「空读数」的那一侧**（§11.2：否则 `(o)` 可由「一律 `Some`」满足）。
///
/// **红条件**：把 `generate-image` 的 `determinism` 改成 `Deterministic`（**放宽档**——
/// `Deterministic` 放宽 §305 的复用面；与设计 §11.3 那一行的档位名不同、动作相同，
/// 见计划 `## 遗留` 第一节第 5 条）→ 那一条红，**且 Task 2 的 (a) 的对应行也红**
/// （`tests/operators.rs` 的 `every_row_of_the_table_is_pinned_verbatim`，期望值手写）。
///
/// **本用例与 Task 2 的 (a) 是两条独立的守卫**（设计 §3.5 末与 §11.3）：(a) 读的是
/// **声明的取值**，本用例读的是 P1 的**判定结果**——把某一枚的 `determinism` 改成
/// `Deterministic` 会让两条都红，而把某一枚的 `backend_candidates` 改名只让 (a) 红。
/// **两条都要留。**
///
/// **本用例是 P1 判据的重跑**（同文件头）：判据本体在 P1 的 `reuse.rs`
/// （`NonDeterministic ⇒ None` 那一支），本用例只核本块声明的取值与它一致。
#[test]
fn the_three_nondeterministic_operators_never_get_a_cache_key() {
    let source = ContentHash::of(b"source-image");

    // 第 1 行：generate-image。
    let generate_image = declared("generate-image");
    assert!(
        cache_key(&generate_image, source.clone()).is_none(),
        "generate-image 声明 NonDeterministic，cache_key 应为 None"
    );

    // 第 2 行：detect-edit-region。
    let detect_edit_region = declared("detect-edit-region");
    assert!(
        cache_key(&detect_edit_region, source.clone()).is_none(),
        "detect-edit-region 声明 NonDeterministic，cache_key 应为 None"
    );

    // 第 3 行：local-generative-edit。
    let local_generative_edit = declared("local-generative-edit");
    assert!(
        cache_key(&local_generative_edit, source.clone()).is_none(),
        "local-generative-edit 声明 NonDeterministic，cache_key 应为 None"
    );
}
