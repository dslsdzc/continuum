//! §278 的四个 alias 作用域，以及它们与 §274 的九个 `ResolutionScope` 的映射
//! （**这张映射表是本设计的决定**，见下）。
//!
//! # 本文件为什么在 Task 4 就存在（一处计划次序缺口，照实写）
//!
//! 计划把 `AliasScope` 排在 **Task 7**（`docs/superpowers/plans/2026-10-06-p4-semantic-layer.md:942`，
//! Task 7 的 Produces 逐字列了它；那一节同处把 `src/alias.rs` 定为它的家），
//! 而 **Task 4 的 Produces 里有两条以它为入参／出参的函数**
//! （`alias_scope_to_resolution_scope` / `resolution_scope_to_alias_scope`，同文件 `:635`），
//! Task 4 的用例清单也点了这张映射（设计 §14 那一行，见下）。
//! **两处次序相抵**：Task 4 要用 Task 7 的类型。
//!
//! **本 task 的处置**：把 `AliasScope` 的**词汇那一半**（四枚变体）提前落在这里——
//! 落在计划给它定的那个文件里，**不另起名字、不换家**。Task 7 要往本文件里加的是
//! `Alias` / `AliasError` / `promote_alias` / `save_alias` / `load_alias` / `list_aliases` /
//! `p4_canonical_migrations`，以及 `as_str` / 解析那一对（§278 的四值落库走的那条路）。
//!
//! **为什么不是「Task 4 在 `explicit.rs` 里自定一枚 `AliasScope`」**：那会让同一个名字有
//! 两个可能的家，而计划已经把它写在 `alias.rs`；**为什么不是「Task 4 先不做这条映射」**：
//! 它是本 task Produces 里逐字列出的两枚，且它承载 §4.1.1 末段那条可断言的判据
//! （「一条以 alias scope 表述的约束必须先经上表换成 §274 的值」）。
//!
//! **这一处缺口与 P4 Task 2 报的那一处同形**（`ResolutionResult` 的载荷排在 Task 4、
//! 而 Task 2 的 `finish` 签名要用它），处置也同形：把跨 task 的那一句写进
//! **用到它的那个 task 的 Files/Produces 所覆盖的地方**，而不是留在别处的正文里。
//!
//! # 两套 scope 词汇的关系（本设计的决定）
//!
//! §274（`docs/spec/05-normative.md:1408-1426`）给的是一条**优先级层级**的九个名字
//! （`explicit` / `session` / `conversation` / `project` / `workspace` / `tool` / `plugin` /
//! `user` / `global`），§278（同文件 `:1485-1499`）给的是 alias 的四个寿命
//! （`TURN` / `SESSION` / `PROJECT` / `USER_PERSISTENT`），
//! **两节各自成文、互不引用**——**规范没有给出九值与四值的关系**。
//! 设计 §4.1.1（`docs/superpowers/specs/2026-10-06-p4-semantic-layer-design.md:574-604`）
//! 补了一张四项的对照表，其内容（`TURN`→`conversation`、`SESSION`→`session`、
//! `PROJECT`→`project`、`USER_PERSISTENT`→`user`）就是本文件两条函数的实现。
//!
//! **用途**：`ExplicitConstraint::Scope` 只收 §274 的九值（见 `explicit.rs`），
//! 故**一条以 alias scope 表述的约束必须先经 [`alias_scope_to_resolution_scope`] 换成
//! §274 的值**；换不出来的那五枚（`workspace` / `tool` / `plugin` / `global` / `explicit`）
//! **在 alias 层没有对应**，故**不能用 alias 表述**（[`resolution_scope_to_alias_scope`]
//! 对它们返回 `None`）。
//!
//! **`TURN` → `conversation` 这一对的方向是「放宽」**（`TURN` 只活一轮，`conversation` 活一段对话），
//! 而放宽正是 §278「MUST NOT 默认把 Session Alias 永久化」要防的那一侧——设计 §4.1.1
//! 末段已把这条方向与它的失效条件写死（`design:584-595`）。**本文件照录这张表，不重判它**：
//! 「一旦 `TURN` 有了寿命，这一对须重判」的判据归设计，本层今天只实现映射本身。

use crate::explicit::ResolutionScope;

/// §278 的四个 alias 作用域。**闭集四枚**——§278（`docs/spec/05-normative.md:1485-1499`）
/// 逐字列的就是这四个词，没有第五枚可以喂进 [`alias_scope_to_resolution_scope`]。
///
/// # 为什么闭集这件事重要（它不是洁癖）
///
/// 「一条以 alias scope 表述的约束必须先经上表换成 §274 的值」这句判据，
/// **只有在定义域封闭时才可断言**：闭集下，「换不出来的」这件事有确定的外延
/// （§274 里那五枚没有 alias 对应），而开放取值域（比如拿 `&str`）下，
/// 「哪个串换不出来」是一条由调用方定义的性质，本层无从断言。
///
/// # 派生
///
/// `Debug` 是断言失败时打得出来；`Clone` 是它要能随值传递；`PartialEq` / `Eq`
/// 是**判据本身要的**——映射的断言形式是「`TURN` 映到 `conversation`」，
/// 那需要「两个 `ResolutionScope` 相等」可判（见 [`crate::ResolutionScope`] 的派生说明）。
///
/// **不带 `as_str` / `parse`**：§278 的四值**落库**时怎么编码（大小写、下划线）
/// 是 Task 7 的地盘（计划 `:942` 的 `scope` 列纪律，`docs/superpowers/plans/2026-10-06-p4-semantic-layer.md:1000`），
/// 本 task 只用它当词汇，不给它编码。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AliasScope {
    /// §278 的 `TURN`。
    Turn,
    /// §278 的 `SESSION`。
    Session,
    /// §278 的 `PROJECT`。
    Project,
    /// §278 的 `USER_PERSISTENT`。
    UserPersistent,
}

/// §278 的四枚 → §274 的一枚。**定义域恰是那四枚**：`AliasScope` 是封闭枚举，
/// 故本函数是**全函数**（没有 `Option` / `Err`——闭集上「没有对应」这件事不存在）。
///
/// # 那句「放宽还是收紧」不在这里写：它在设计里，本函数只是那张表
///
/// 设计 §4.1.1 末段（`design:584-595`）逐条写了四对的方向，其中 `TURN`→`conversation`
/// 是**放宽**（并带一条失效条件：`TURN` 一旦有寿命就要重判）。本文件**不重抄那几句判据**：
/// 抄一遍会让同一句话有两个家，改一处另一处静默失真（本仓「别描述别人的文件的形状」的同一形状）。
///
/// **匹配不带 `..` 且没有通配臂**：给 `AliasScope` 加第五枚即**编译不过**
/// ——这是本函数定义域封闭的编译期照片（按本仓纪律，**编译不过不算变红**，
/// 故它是**类型保证**，不是运行期能取到的红）。运行期那一半（四枚逐枚映到哪一枚）
/// 由 `tests/explicit.rs::the_alias_scope_mapping_has_exactly_the_four_alias_scopes_as_its_domain` 钉。
pub fn alias_scope_to_resolution_scope(scope: &AliasScope) -> ResolutionScope {
    match scope {
        AliasScope::Turn => ResolutionScope::Conversation,
        AliasScope::Session => ResolutionScope::Session,
        AliasScope::Project => ResolutionScope::Project,
        AliasScope::UserPersistent => ResolutionScope::User,
    }
}

/// §274 的九枚 → §278 的四枚，**没有对应就是 `None`**。
///
/// 返回 `Option` 而不是造一枚「无对应」的 `AliasScope` 变体：把五枚塞进一枚新变体
/// 会让「哪些 §274 值能落成 alias」这件事**从类型上消失**，而它正是本函数要说的那一句。
/// （也不是 `Result`：这里没有第二种失败，也没有任何失败原因可给——本层不落库、不做 I/O。）
///
/// **九枚逐枚**：四枚给 `Some`、五枚（`workspace` / `tool` / `plugin` / `global` / `explicit`）
/// 给 `None`，由 `tests/explicit.rs` 的同名用例逐枚断言（**不抽代表**）。
pub fn resolution_scope_to_alias_scope(scope: &ResolutionScope) -> Option<AliasScope> {
    match scope {
        ResolutionScope::Conversation => Some(AliasScope::Turn),
        ResolutionScope::Session => Some(AliasScope::Session),
        ResolutionScope::Project => Some(AliasScope::Project),
        ResolutionScope::User => Some(AliasScope::UserPersistent),
        // 下面五枚在 §278 里**没有对应**——设计 §4.1.1 末段点名的正是这五枚
        // （`design:598-602`）。**逐枚列出、不写 `_ => None`**：通配臂会让
        // 「加一枚 §274 值」静默落进 `None`，而那一枚要不要有对应是**要重判的决定**，
        // 不是默认为「没有」。逐枚写的代价是加值时要改这里，那正是我们要的那次停顿。
        ResolutionScope::Workspace => None,
        ResolutionScope::Tool => None,
        ResolutionScope::Plugin => None,
        ResolutionScope::Global => None,
        ResolutionScope::Explicit => None,
    }
}
