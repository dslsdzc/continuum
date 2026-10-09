//! §3.1 的最后一个组件：**Ambiguity Manager**。
//!
//! 设计 §3.1 的表（`docs/superpowers/specs/2026-10-06-p4-semantic-layer-design.md:333`）
//! 给它的定义是：聚合同一次输入里所有 mention 的 `ResolutionResult`，
//! 产出 `CanonicalRepresentation` 或挂起。落成函数就是 [`finish`]。
//!
//! # `ResolutionResult` 在本 task 是**前向声明**（一处计划次序缺口）
//!
//! 计划把 §4.1 的四态排在 **Task 4**
//! （`docs/superpowers/plans/2026-10-06-p4-semantic-layer.md:597-599`：
//! `Produces: … ResolutionResult, Resolved, ConfirmRequired, Ambiguous, Unknown, UnknownReason …`），
//! 而 **Task 2 的 `finish` 签名就要用它**（同文件 `:494`）。**这是计划的一处次序缺口：
//! Task 2 用到 Task 4 的产物**，而本层不能把 Task 4 提前做（它要 Task 3 的 `EntityId`）。
//!
//! 故本 task 只落**这个任务真需要的那一层**：`ResolutionResult` **是哪一态**。
//! 各态的**载荷**按 §4.1 由 Task 4 补上（`design:378-425` 有逐支的字段清单；
//! 例如 `Resolved { entity: EntityId, satisfied: Vec<ExplicitConstraint>, candidate_count: usize }`），
//! 本处**不预先发明**那些字段——它们的类型（`EntityId`、`Candidate`、`ExplicitConstraint`）
//! 分别落在 Task 3 与 Task 4，今天在本 crate 里**根本写不出来**。
//!
//! ## 为什么不是另外两种写法
//!
//! - **不是「本 task 就把 §4.1 的载荷照录」**：那需要 `EntityId`（Task 3）、
//!   `Candidate` / `ExplicitConstraint`（Task 4）。按纪律 0 的落点签名三问，
//!   **返回类型对不上** ⇒ 这一枚落在**层③**（写不出来，只能靠改依赖表／改签名表达，
//!   而那时唯一可观察者是调用侧编译失败，**按纪律不算红**）。
//! - **不是「本层自定一枚二元类型（如 `Resolved` / `Unresolved`），Task 4 再写映射」**：
//!   那会发明一个计划里没有的公开名字，而 `finish` 的签名是简报**逐字给定**的
//!   （`plans/…:494` 写的就是 `Vec<(Mention, ResolutionResult)>`）。
//!
//! ## 接缝（Task 4 要动的两处）
//!
//! 1. 类型本体会从本文件搬到 `resolver.rs`；
//! 2. [`finish`] 里那一处 `match` 的 `ResolutionResult::Resolved` 要写成
//!    `ResolutionResult::Resolved(_)`（**只有这一处**模式匹配四态）。
//!
//! 本 task 的用例也直接构造四态（它们是夹具，见 `tests/canonical.rs`），
//! 故 Task 4 补载荷时那些夹具要一并改——这一点记在这里，免得到时以为是回归。

use crate::Mention;
use crate::canonical::{CanonicalRepresentation, Canonicalized, UnresolvedMention};

/// §4.1 的四态（`design:378-425`）——**本 task 只落「是哪一态」**。
///
/// 四支都是**返回值，不是错误**（§206 的 `abstain` 已把「没有可靠解析」收进 `Unknown`；
/// `design:433-437`）。本 task 的处置是**前向声明**：载荷见本文件头。
///
/// `ConfirmRequired` 与 `Ambiguous` **必须可区分**：前者是「有占优候选但门槛未过」、
/// 后者是「多个候选都合格」，两者处置方向不同（`design:443-448` 把这条边界记为遗留，
/// 但**合并成一支会让门槛失掉唯一的可观察后果**，Tests 那侧 Task 4 有一条钉它）。
/// 本 task 用不到这条区分，但**照 §4.1 的四枚落**，免得 Task 4 从三枚补一枚时
/// 有一刻两处各说一套。
///
/// `Debug` / `Clone` / `PartialEq` / `Eq`：本 task 的用例要断言「列表里那一枚是哪一态」，
/// 故要 `PartialEq`；断言失败要打得出来，故要 `Debug`。**不派生 `Copy`**——
/// Task 4 补载荷后各支将不再是 `Copy` 的（`Vec` 在里头），现在派生会给自己留一处
/// 到时候必须撤掉的表达力。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolutionResult {
    /// §206「允许自动绑定」。**`Canonical` 支要求每一枚 mention 都是这一态**。
    Resolved,
    /// §206「第一候选明显，但不足以安全执行」。
    ConfirmRequired,
    /// §206「多个候选仍然成立」。
    Ambiguous,
    /// §206「当前没有可靠解析」。
    Unknown,
}

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
/// `design:2319-2322`）。故本文件的两支今天只被 `tests/canonical.rs` 的**夹具**走到，
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
            ResolutionResult::Resolved => resolved.push(mention),
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
