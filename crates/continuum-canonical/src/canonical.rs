//! §3.2 的规范化产物：`Canonicalized` 和类型与它两支各自的载荷。
//!
//! §3.2（`docs/superpowers/specs/2026-10-06-p4-semantic-layer-design.md:340-354`）
//! 只给了**和类型本身**（`design:346-349`，`Canonical` / `NeedsResolution` 两支），
//! **没有给任何一支载荷的字段清单**——全篇提到 `CanonicalRepresentation` 的地方只有两处
//! （`design:333` 与 `design:347`），提到 `UnresolvedMention` 的只有一处（`design:348`），
//! 三处都只是**名字**。故本文件的两枚载荷是**本层自定的形状**（`grep -n` 实测，
//! 不是转抄）。它们今天的内容因此只有「本层手上真有的东西」：
//! 切出来的 mention 与它那一枚判定结果；解析出的实体（`EntityId`）要等 Task 3/4，见下。
//!
//! # 一处悬空引用（据实记）
//!
//! §3.1 的表末（`design:333`）写「产出 `CanonicalRepresentation` 或挂起（**§3.3**）」，
//! 而设计 §3 只有 §3.1 与 §3.2 两节（`grep -n '^## 3\.'` 实测：`design:318`、`design:340`），
//! **没有 §3.3**。本 task 按 §3.2 落，§3.3 那个号码的来历照录于此，不代设计补写。
//!
//! # 两支互斥是**类型**保证的，不是判据保证的
//!
//! `Canonical` 这一支的载荷 [`CanonicalRepresentation`] 的字段私有、
//! **构造入口只有 [`crate::finish`]**（`pub(crate)` 的那个 `new`）。于是
//! 「`Canonical` 里带着未解 mention」这种值**在 crate 外造不出来**——
//! 不是「本层约定不去造它」。这正是 §3.2 要的形状（`design:352-354`）：
//! 一支是另一支的补集，不是两个可以同时为真也可以同时为假的标志位。

use crate::Mention;
use crate::resolver::ResolutionResult;

/// §2.4 第 1 条（`docs/02-工程.md:130`）的产物：一个和类型，**两支互斥**。
///
/// 两支的含义见 [`CanonicalRepresentation`] 与 [`UnresolvedMention`]。
///
/// # 两支穷尽，且没有第三支
///
/// 读到本类型时的**穷尽 `match` 不带 `..`** 是 §3.2「两支互斥」的编译期那一半：
/// 加第三支即编译不过。**编译不过不算变红**（本仓纪律），故那半张照片是**类型保证**，
/// 不是运行期能取到的红；运行期那一半（恰有一支成立）由
/// `tests/canonical.rs::the_two_arms_are_complements` 钉住。
///
/// # `Debug` 与 `Clone`
///
/// 同 [`crate::Opaque`] 的理由：`Debug` 是断言失败时打得出来，`Clone` 是结构体要能装进
/// `Vec` 并随值传递（设计 §1.2 的「以值进入」）。**不派生 `PartialEq`**：
/// 本类型没有哪一条判据要拿「两个产物相等」去核；等到有了再加，
/// 免得现在给出一枚没有用例的表达力。
#[derive(Debug, Clone)]
pub enum Canonicalized {
    /// 每一个 mention 都是 `Resolved`（§3.2 的闭集条件，`design:352-354`）。
    Canonical(CanonicalRepresentation),
    /// 至少有一枚没解出来；**列表里恰是那些没解的**，不含已解的。
    NeedsResolution(Vec<UnresolvedMention>),
}

/// `Canonical` 支的载荷：**解出来的那些 mention**，按输入次序照录。
///
/// # 为什么只有 mention，没有解出的实体（Task 4 复判后**仍不改**，理由照实记）
///
/// Task 2 在这里留过一处接缝：「Task 4 给四态补上载荷后，本类型应把那一份照录进来
/// （那一处要改的是本类型与 [`crate::finish`]，两处）」。**Task 4 补载荷时复判了它，
/// 结论是不改**，三条理由：
///
/// 1. **没有判据要它**：设计 §3.2（`design:340-354`）只给了「两支互斥」这一条性质，
///    `CanonicalRepresentation` 的字段清单**全篇没给**（本文件头已实测这一点）。
///    接缝那一句是 Task 2 的**推测性**承诺（「应」），不是规范要求。
/// 2. **加了就是一条没有照片的表达力**：本 task 的用例清单里没有任何一条会读
///    「mention → 解出的实体」这个对应（它的第一个真实消费者在 P4 之后的 Intent Compiler
///    那一侧），而本仓的口径是**表达力要么有照片、要么先不加**。
/// 3. **它会改掉一个已经被钉住的读函数**：`mentions()` 现在返回 `&[Mention]`
///    （`tests/canonical.rs` 有一条断言它等于那两枚 mention）。要同时带上实体，
///    这个读函数就得换形状（迭代器或平行向量），而**换形状这件事本身没有判据**
///    ——平行向量还会引入一处可以失步的冗余。
///
/// **代价照实写**：今天 `Canonical` 支的载荷说不出「这一枚 mention 解到了哪个实体」，
/// 故它**不是一个可直接使用的规范化结果**，只是一份「哪些 mention 已经解出」的记录。
/// 补它的那个 task 应当带一条读它的用例来（那时才有判据）。
///
/// # 一处与简报预期的相抵（照实记）
///
/// 简报写「本 task 补载荷，即改 `ambiguity.rs` 的定义与 `canonical.rs` 的 `finish` 签名」
/// （`docs/superpowers/plans/2026-10-06-p4-semantic-layer.md:616`）。
/// **`finish` 的签名本 task 没有改**，也不需要改：`Vec<(Mention, ResolutionResult)>`
/// 这个入参形状装得下带载荷的四态（载荷在 `ResolutionResult` 里面）。
/// 唯一的改动是 [`crate::finish`] 里那一处 `match` 从 `Resolved` 写成 `Resolved(_)`
/// （`ResolutionResult` 有了载荷之后的必然改法，Task 2 已把那处接缝写在 `ambiguity.rs` 文件头）。
///
/// # 不可从外部构造
///
/// 字段私有、`new` 是 `pub(crate)`：外面的调用方只能从 [`crate::finish`] 拿到它。
/// 这条不是洁癖——它是 §3.2 那句「两支互斥」在**值**这一层的兑现方式：
/// 造不出「`Canonical` 里带着未解 mention」的值。
#[derive(Debug, Clone)]
pub struct CanonicalRepresentation {
    mentions: Vec<Mention>,
}

impl CanonicalRepresentation {
    /// **唯一入口**（crate 内）：只由 [`crate::finish`] 调用，且在它那一支里
    /// 已经确认过「每一枚都是 `Resolved`」。
    pub(crate) fn new(mentions: Vec<Mention>) -> Self {
        CanonicalRepresentation { mentions }
    }

    /// 解出来的每一枚 mention，按输入次序。
    pub fn mentions(&self) -> &[Mention] {
        &self.mentions
    }
}

/// `NeedsResolution` 支的元素：**一枚没解出来的 mention**，连同**它没解成什么**。
///
/// # 为什么带 `state`
///
/// §2.4 第 1 条的原文是「被明确标记为 `Unknown` / `Ambiguous`」
/// （`docs/02-工程.md:130`）——**标记的是「哪一种没解」**。
/// 只记「哪一枚」的话，挂起的那一方说不出是 `Unknown` 还是 `Ambiguous`，
/// 而这两件事的处置方向相反（前者要扩大检索、后者要问用户；§4.1 的 `UnknownReason`
/// 分两枚同一条理由，`design:417-424`）。
///
/// # 为什么字段是 `ResolutionResult` 而不是一枚「未解态」枚举
///
/// 另立一枚三变体枚举（`ConfirmRequired` / `Ambiguous` / `Unknown`）会在两处各存一份
/// §4.1 的三枚——将来加一枚、改一个名，两处得同时改，而**没有任何东西会因此变红**
/// （本仓「枚举断言须逐项有照片」的同一形状）。故这里直接照收判定结果本身。
///
/// # M-2-3：「`UnresolvedMention` 里装着 `Resolved`」为什么**没有**做成类型上不可表示
///
/// **代价照实写**：`ResolutionResult` 里有一枚 `Resolved`，故单看类型，
/// 「`UnresolvedMention` 里装着一枚 `Resolved`」是写得出来的。这个不变量靠
/// **构造面**守住而不是靠类型：字段私有、`new` 是 `pub(crate)`，全 crate 只有
/// [`crate::finish`] 造它，而那里**先判过 `Resolved` 才走另一支**。
///
/// **Task 4 给四态补上载荷后复判了这一条**（它是 P4 Task 2 查出、裁定写进 Task 4
/// 的 Files 的那一枚），结论：**不改成类型上不可表示**，理由三条——
///
/// 1. **唯一的改法是另立一枚三变体枚举**（`ConfirmRequired` / `Ambiguous` / `Unknown`），
///    而那正是本节上面刚否掉的那件事：它会在两处各存一份 §4.1 的三枚，
///    将来加一枚、改一个名要两处同改，而**没有任何东西会因此变红**
///    （本节上面那段已写明这条）。
/// 2. **外部的门比 Task 2 时写的那句话更窄**（这一半是 Task 4 补的实测）：
///    `Resolved` 的载荷类型今天字段是 `pub`（`resolver.rs` 的文件头写了为什么），
///    故 crate 外**写得出**一枚 `ResolutionResult::Resolved(Resolved { entity, satisfied, candidate_count })`；
///    但 [`UnresolvedMention::new`] 是 `pub(crate)`，**那枚值进不来**
///    ——crate 外能做的只有「把一枚 `Resolved` 交给 [`crate::finish`]」，
///    而 `finish` 会把它归到 `Canonical` 那一支、根本不构造本类型。
///    **故本不变量今天仍然只由那一处构造面守**，而不是「类型守不住、谁都能破」。
/// 3. **有一条运行期照片**：`tests/canonical.rs::every_mention_resolved_gives_the_canonical_arm`
///    断言「每一枚都是 `Resolved` ⟹ 落 `Canonical` 支」——把已解的并进挂起列表的变异体
///    会红在那一条上（Task 2 的变异轮实测过同一形状）。
///    **它是运行期照片，不是类型保证**：本条要求的「类型上不可表示」这一侧，
///    本层**没有做到**，也不假装做到。
///
/// **收件人：无需上报**——这一条是「不处置也是处置，写出来即可」（计划
/// `docs/superpowers/plans/2026-10-06-p4-semantic-layer.md:623-624` 给的两条路里的第二条）。
#[derive(Debug, Clone)]
pub struct UnresolvedMention {
    mention: Mention,
    state: ResolutionResult,
}

impl UnresolvedMention {
    /// **唯一入口**（crate 内）：只由 [`crate::finish`] 调用，且调用点已确认
    /// `state` **不是** `Resolved`。
    pub(crate) fn new(mention: Mention, state: ResolutionResult) -> Self {
        UnresolvedMention { mention, state }
    }

    /// 哪一枚 mention 没解出来。
    pub fn mention(&self) -> &Mention {
        &self.mention
    }

    /// 它没解成哪一态（§4.1 的四态里除 `Resolved` 外的三枚之一）。
    pub fn state(&self) -> &ResolutionResult {
        &self.state
    }
}
