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
use crate::ambiguity::ResolutionResult;

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
/// # 为什么今天只有 mention，没有解出的实体
///
/// 设计没给它的字段清单（见文件头），而本层今天**手上真有的**只有
/// `finish` 的入参：mention 与它那一枚 `ResolutionResult`。而本 task 的
/// `ResolutionResult` 是**前向声明**（见 `ambiguity` 模块的文件头，及本节末的「接缝」），各态**不带载荷**——
/// 它要带的 `entity: EntityId`（§4.1，`design:387-393`）落在 Task 3 的
/// `EntityId` 与 Task 4 的解析器之后。故这里今天存不了「mention → 解出的实体」的对应。
/// **接缝**：Task 4 给四态补上载荷后，本类型应把那一份照录进来（那一处要改的是本类型
/// 与 [`crate::finish`]，两处）。
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
/// **代价照实写**：`ResolutionResult` 里有一枚 `Resolved`，故单看类型，
/// 「`UnresolvedMention` 里装着一枚 `Resolved`」是写得出来的。这个不变量今天靠
/// **构造面**守住而不是靠类型：字段私有、`new` 是 `pub(crate)`，全 crate 只有
/// [`crate::finish`] 造它，而那里**先判过 `Resolved` 才走另一支**。
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
