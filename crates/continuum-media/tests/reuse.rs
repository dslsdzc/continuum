//! §305 的复用照片（设计 §6.2 的表、§12.2 第 (r) 条）：**不建第二份判据**。
//!
//! §131 的兑现不是本 crate 的函数，而是本领域算子的 `determinism` 声明；判据本身
//! 已由 P1 落地（`crates/continuum-graph/src/reuse.rs` 的 `cache_key` / `can_reuse`），
//! 本文件只核对「本块声明的 `determinism` 真能让那条判据成立」——**不另写复用函数**
//! （切分 §四第 7 条：判据只许有一处）。失效方向：两份判据并存且各有用例、各自都绿时，
//! 失配是「一套说能复用、另一套说不能」——而这种不一致**不会在任何一条用例里显形**。
//!
//! 本条拍到的是**判据的返回值**（`cache_key` / `can_reuse` 的取值），不是端到端效果：
//! §131 的复用今天无生产调用方（`CacheKey` 无存储），「少跑了一次重算」在本轮拍不到
//! （设计 §12.5 第 4 条）。
//!
//! **照片坐标**：`continuum-graph` 是本 crate 的 **dev 边**（生产代码不调它）；
//! 这条边的使用点就是本文件（设计 §2.1 第 2 行、§6.2 末）。

use continuum_artifact::ContentHash;
use continuum_graph::{CacheKey, cache_key, can_reuse};
use continuum_media::all_operators;
use continuum_operator::{Operator, OperatorId};

/// 按 id 从 `all_operators()` 里取本块声明的那一枚算子。
/// 取不到即 panic：用例的语料行与 17 枚清单对不上时，宁可红也不静默跳过。
fn declared(id: &str) -> Operator {
    all_operators()
        .into_iter()
        .find(|operator| operator.id == OperatorId::new(id))
        .unwrap_or_else(|| panic!("all_operators() 里没有 {id}"))
}

/// 第 (r) 条（§6.2 的六行**逐行**，不是抽样）：四枚产派生产物的算子
/// （`shot-detect` / `audio-analysis` / `metadata-analysis` / `proxy-build`）的
/// `cache_key` 为 `Some` 且四条件齐时 `can_reuse` 为真；另两枚
/// （`transcribe` / `vision-analysis`）为 `None`。
///
/// **语料手写**：六个 id 与各自的期望逐行取自设计 §6.2 的表（唯一事实来源），
/// 不从实现侧取——若期望值写成按 `all_operators()` 自己遍历，测的是「它等于它自己」。
///
/// **红条件（取反档）**：把 `shot-detect` 的 `determinism` 改成 `NonDeterministic` ⇒
/// 该行的 `cache_key` 由 `Some` 变 `None`，红。
/// （设计 §12.3 的放宽档同形：把 `transcribe` 改成 `Deterministic` ⇒ 本条第二组红，
/// 且同一轮 §5.4 的 (h) 会红——`local-whisper` 等三个候选不在那份名单里。）
///
/// **两侧代价**（设计 §6.3）：`transcribe` / `vision-analysis` 取 `NonDeterministic` 的代价是
/// §131 对这三类产物（`Transcript` / `FaceIndex` / `Embeddings`）的「无需重新计算」**不成立**
/// （每轮都重跑模型，代价是算力与等待时间）；反过来的代价是**静默地让一份不同的结果
/// 冒充这次的结果**，且它发生在「输入没变」这个最不容易被怀疑的条件下。
/// **取向的理由是 §305 自己的措辞**：该节给的是「Runtime **MAY** 直接复用」——一个 MAY，
/// 即「允许复用」而非「可以跳过重算的保证」；而 §131 的「无需」是**定位文档里的成本陈述**，
/// 不是规范正文里的 MUST/SHOULD。故判据的权威在 §305 一侧，本设计按它取 fail-closed。
///
/// **据实记**：本条不是「先假后真」型的用例——Task 3 的 `determinism` 已写对，
/// 它是**声明与既有判据之间的一致性照片**，价值在变异轮（上两条红条件）显现。
#[test]
fn the_six_derived_artifact_producers_have_the_declared_reuse_outcome() {
    // 手写的「原素材」输入哈希：六行共用同一份——正组的四条件之一（输入未变）由此成立。
    let source = ContentHash::of(b"source-material");

    // 第 1 行：shot-detect 产 ShotIndex（§131），本块声明 Deterministic ⇒ Some，四条件齐时可复用。
    let shot_detect = declared("shot-detect");
    let key: CacheKey = cache_key(&shot_detect, source.clone())
        .expect("shot-detect 声明 Deterministic，cache_key 应为 Some");
    assert!(
        can_reuse(&shot_detect, Some(&key), &source, false),
        "shot-detect：determinism／输入哈希／版本／contract 四条件齐时应判可复用"
    );

    // 第 2 行：audio-analysis 产 AudioFeatures（§131）与 beat map（§33）⇒ 同上。
    let audio_analysis = declared("audio-analysis");
    let key: CacheKey = cache_key(&audio_analysis, source.clone())
        .expect("audio-analysis 声明 Deterministic，cache_key 应为 Some");
    assert!(
        can_reuse(&audio_analysis, Some(&key), &source, false),
        "audio-analysis：四条件齐时应判可复用"
    );

    // 第 3 行：metadata-analysis 产 metadata analysis（§33。不是 §131 的条目，出处照实留在格里）⇒ 同上。
    let metadata_analysis = declared("metadata-analysis");
    let key: CacheKey = cache_key(&metadata_analysis, source.clone())
        .expect("metadata-analysis 声明 Deterministic，cache_key 应为 Some");
    assert!(
        can_reuse(&metadata_analysis, Some(&key), &source, false),
        "metadata-analysis：四条件齐时应判可复用"
    );

    // 第 4 行：transcribe 产 Transcript（§131）⇒ 本块声明 NonDeterministic。
    let transcribe = declared("transcribe");
    assert!(
        cache_key(&transcribe, source.clone()).is_none(),
        "transcribe 声明 NonDeterministic，cache_key 应为 None"
    );

    // 第 5 行：vision-analysis 产 FaceIndex 与 Embeddings（§131）⇒ 同上。
    let vision_analysis = declared("vision-analysis");
    assert!(
        cache_key(&vision_analysis, source.clone()).is_none(),
        "vision-analysis 声明 NonDeterministic，cache_key 应为 None"
    );

    // 第 6 行：proxy-build 产「建立 proxy」（§33。不是 §131 的条目，出处照实留在格里）⇒ Some，四条件齐时可复用。
    let proxy_build = declared("proxy-build");
    let key: CacheKey = cache_key(&proxy_build, source.clone())
        .expect("proxy-build 声明 Deterministic，cache_key 应为 Some");
    assert!(
        can_reuse(&proxy_build, Some(&key), &source, false),
        "proxy-build：四条件齐时应判可复用"
    );
}

/// 另一侧：复用判据的**第四条件**（`contract_affected`）不在本块的声明射程内——
/// §305 的四条件里，本块声明的 `determinism` 只占第一条。
///
/// 同一枚算子（本块声明的 `shot-detect`）、同一输入哈希、同一版本：只因
/// `contract_affected` 从 `false` 翻到 `true`，`can_reuse` 就从真翻到假。
///
/// **红条件（移除档）**：把 `can_reuse` 里 `contract_affected` 那一支删掉 ⇒ 本条红。
/// **那是 P1 的代码（`crates/continuum-graph/src/reuse.rs`），本 task 只核、不改**——
/// 故**这一条的作用是留下「本块声明的 determinism 不是复用的全部条件」的读数**，
/// 不是本块要建的第二份判据（判据只有 P1 那一处，切分 §四第 7 条）。
///
/// **照片坐标**：本 task 的 `continuum-graph` 是 **dev 边**（生产代码不调它），
/// 这条边的使用点就是本文件（另见设计 §2.1 第 2 行）。
#[test]
fn the_contract_condition_is_the_fourth_thing_can_reuse_checks() {
    let shot_detect = declared("shot-detect");
    let source = ContentHash::of(b"source-material");
    let key: CacheKey = cache_key(&shot_detect, source.clone()).expect("shot-detect 应产生缓存键");

    // 对照位：同一组参数在 contract_affected == false 时为真——没有这一条，
    // 「can_reuse 一律为假」的坏实现也能过本用例，差异就没钉在 contract 这一格上。
    assert!(
        can_reuse(&shot_detect, Some(&key), &source, false),
        "对照位：contract 未受影响时，其余三项齐应判可复用"
    );

    // 同一枚算子、同一输入哈希、同一版本，只把 contract_affected 翻到 true ⇒ 判为不可复用。
    assert!(
        !can_reuse(&shot_detect, Some(&key), &source, true),
        "contract_affected == true 不满足 §305，应判不可复用"
    );
}
