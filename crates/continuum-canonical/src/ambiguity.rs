//! §3.1 的最后一个组件：**Ambiguity Manager**。
//!
//! 设计 §3.1 的表（`docs/superpowers/specs/2026-10-06-p4-semantic-layer-design.md:333`）
//! 给它的定义是：聚合同一次输入里所有 mention 的 `ResolutionResult`，
//! 产出 `CanonicalRepresentation` 或挂起。落成函数就是 [`finish`]。
//!
//! # `ResolutionResult` 的类型本体已搬到 `resolver.rs`（本处只留它的一次分流）
//!
//! Task 2 时它是**前向声明**（只有「是哪一态」那一层，载荷留空）：计划把 §4.1 的四态
//! 排在 **Task 4**（`docs/superpowers/plans/2026-10-06-p4-semantic-layer.md:604`，
//! Task 4 的 Produces 逐字列了 `ResolutionResult, Resolved, ConfirmRequired, Ambiguous,
//! Unknown, UnknownReason`），而 **Task 2 的 `finish` 签名就要用它**
//! ——那是一处计划次序缺口，Task 2 把接缝写在这里，**Task 4 已按它落地**：
//!
//! 1. 类型本体**已搬到 [`crate::resolver`]**（载荷要 `EntityId`（Task 3）与
//!    `Candidate` / `ExplicitConstraint`（Task 4），Task 2 时在本 crate 里写不出来）；
//! 2. [`finish`] 里那一处 `match` **已改成** `ResolutionResult::Resolved(_)`
//!    ——那是全 crate 唯一一处模式匹配四态的地方。
//!
//! `ResolutionResult` 的类型文档（四态各自的读法、`ConfirmRequired` 与 `Ambiguous`
//! 为什么必须可区分、为什么四支都是返回值）现在住在 `resolver.rs`，本文件不重抄。

use crate::Mention;
use crate::canonical::{CanonicalRepresentation, Canonicalized, UnresolvedMention};
use crate::resolver::ResolutionResult;

/// §3.1 的 Ambiguity Manager：聚合同一次输入里所有 mention 的判定结果。
///
/// **`Canonical` 这一支要求每个 mention 都是 `Resolved`**（§3.2，`design:352-354`；
/// 那句「每个候选都判到」的闭集判据本身落在 §4.2 的断言 A／D，由 Task 5 钉）。
/// 只要有一枚不是 `Resolved`，整份输入就进 `NeedsResolution` 支，
/// 且**列表里恰是那些没解的 mention**（不含已解的）——「挂起时只说清哪些没解」。
///
/// # 今天没有真实的调用方
///
/// 本函数的入参今天**没有任何生产方**：`ResolutionResult` 的产生方是 `resolve`，
/// 落在 Task 4/5；而真实候选来源今天**零个**（设计 §16 第 5 条，
/// `design:2331-2334`）。故本文件的两支今天只被 `tests/canonical.rs` 的**夹具**走到，
/// 钉的是**分支机制**（设计 §15.2 第 3 条，`design:2264-2266`）。
///
/// # 零 mention 的输入落 `Canonical` 支
///
/// 「每个 mention 都是 `Resolved`」在空集上**平凡成立**，故零命中的输入落 `Canonical` 支、
/// 载荷里的 mention 列表为空。**这是有意的**：§2.4 第 1 条要的是「被规范化，
/// **或**被明确标记为 Unknown / Ambiguous」（`docs/02-工程.md:130`），
/// 而一句没有任何 mention 的输入**没有需要解析的东西**——把它判成挂起是发明一条判据。
/// 零命中这一侧的形态（不 panic、不造占位 mention）由 Task 1 的
/// `an_input_without_any_mention_yields_an_empty_vec` 钉住。
pub fn finish(parts: Vec<(Mention, ResolutionResult)>) -> Canonicalized {
    let mut resolved = Vec::new();
    let mut unresolved = Vec::new();

    for (mention, result) in parts {
        // 四态里只有 `Resolved` 走 `Canonical` 侧。`other` 那一支照收三枚未解态，
        // 不在本层再分一次类——分类是 §4.1 给的，本层只是分流
        // （这也是 `UnresolvedMention` 直接收 `ResolutionResult` 的理由，见其文档）。
        match result {
            ResolutionResult::Resolved(_) => resolved.push(mention),
            other => unresolved.push((mention, other)),
        }
    }

    if unresolved.is_empty() {
        Canonicalized::Canonical(CanonicalRepresentation::new(resolved))
    } else {
        Canonicalized::NeedsResolution(
            unresolved
                .into_iter()
                .map(|(mention, state)| UnresolvedMention::new(mention, state))
                .collect(),
        )
    }
}
