//! §227 的六元组与 §214 的 Specification 编译（设计 §6.1／§6.2）。
//!
//! 计划「文件结构」一节把 `Specification、SpecDraft、Unknowns、Unknown、Conflict、
//! UndefinedBehavior、六元组的成员类型` 归在 `src/spec.rs`
//! （`docs/superpowers/plans/2026-10-06-p4-semantic-layer.md:305`）；本文件与
//! `persist.rs` 的分工照 Task 8 的 `intent.rs` / `persist.rs`：
//! **本文件是 §227 的词汇与记录体，`persist.rs` 是 `specification` 表与 I/O**。
//!
//! # §227 的两句 MUST 的落点是一个类型，不是一个约定（§6.1）
//!
//! §227（`docs/spec/05-normative.md:313-331`）给六元组，并明写两句 MUST：
//! 「`unknowns` MUST 被正式保存」与「系统 MUST NOT 把未知项自动转换成假设」。
//! 落点：
//!
//! - [`Unknowns`] 是**两枚的枚举**，不是 `Vec<Unknown>`——`vec![]` 会同时表示
//!   「已查过、没有」与「没查」，而 §227 禁的正是后者被当成前者。
//! - [`SpecDraft`] 里**没有** `unknowns` 字段；[`SpecDraft::finalize`] 要求调用方
//!   **显式给出** `Unknowns` 之一（不是 `Option<Unknowns>`）——「没查」因此不可表达。
//! - [`Specification`] 的六个字段私有、无公开构造函数，**唯一构造入口是 `finalize`**。
//!
//! 三处的照片（各在何处，逐条）：
//!
//! - `Unknowns` 恰两枚与「各自那一枚」的往返：`tests/spec.rs` 的
//!   `unknowns_has_exactly_two_variants` 与
//!   `none_found_and_an_empty_enumerated_are_distinguishable_after_a_round_trip`；
//! - `finalize` 的签名（不能省 `unknowns`、不能是 `Option`）：
//!   `tests/compile_fail/a_draft_without_unknowns_cannot_be_finalized.rs`（`.stderr`
//!   实跑取值，由 `tests/type_level.rs` 驱动）；
//! - `Specification` 的 crate 外不可构造：
//!   `tests/compile_fail/a_specification_cannot_be_constructed_outside_the_crate.rs`；
//!   它的字段清单**逐项**（穷尽解构、不带 `..`）在本文件的 `#[cfg(test)]` 里——
//!   六个字段私有后，那张照片只有 crate 内做得到（本仓已有「照片住在 `src` 的
//!   `#[cfg(test)]` 里」的体例，见 `continuum-canonical/src/resolver.rs`）。
//!
//! # §214 的「检测冲突/未定义行为」只建类型，不建检测器（§6.1）
//!
//! §214（`docs/spec/04-method.md:1105-1138`）的过程末两步是「检测冲突/未定义行为」，
//! 而**「什么算冲突」规范没给判据**（设计 §16 第 10 条）。故本层只建两个类型
//! （[`Conflict`] / [`UndefinedBehavior`]），并把它们作为 [`Unknowns::Enumerated`]
//! 的成员**保存**——不发明检测算法。否定式照片两份：`tests/compile_fail/` 的
//! `conflict_and_undefined_behaviour_are_types_only.rs`（两个检测器名字取不到）
//! 与 Task 21 的 `src/` grep；正面（只保存）在 `tests/spec.rs` 的同名用例里。

use continuum_canonical::Opaque;

/// §227 的六元组。**无公开构造函数**——唯一构造入口是 [`SpecDraft::finalize`]。
///
/// 六个字段**私有**：私有性就是「只能经 `finalize` 造出」这句话的类型形态——
/// 若它们是 `pub`，crate 外可以直接拼一个字面量，那 §227 的「显式给出 `unknowns`」
/// 就被绕过去了（一份运行期用例证明不了「做不到」，故这里靠类型 + 
/// `tests/compile_fail/a_specification_cannot_be_constructed_outside_the_crate.rs`）。
///
/// 读侧经六个访问器（[`Specification::goals`] 等）——**加一个字段不会自动多一个
/// 访问器**：字段清单**逐项**的照片在本文件的 `#[cfg(test)]`（穷尽解构、不带 `..`），
/// 访问器与字段的对应由 `tests/spec.rs` 的
/// `a_specification_has_exactly_the_six_tuple_fields` 逐项断言。
///
/// # 成员类型为什么是不透明串（[`Goal`] 等五个）
///
/// §214 只给了六个名字（`Goal` / `Invariant` / `Acceptance Predicate` /
/// `Forbidden State` / `Preference` / `Unknown`），**未给任何一个的结构**。
/// 故五个成员类型各包一个 [`Opaque`]：本层**保存事实，不发明取值域**
/// （同 [`crate::Priority`] 的处置）。
#[derive(Debug, Clone)]
pub struct Specification {
    /// §227 的 `goals[]`。
    goals: Vec<Goal>,
    /// §227 的 `invariants[]`。
    invariants: Vec<Invariant>,
    /// §227 的 `acceptance_predicates[]`。
    acceptance_predicates: Vec<AcceptancePredicate>,
    /// §227 的 `forbidden_states[]`。
    forbidden_states: Vec<ForbiddenState>,
    /// §227 的 `preferences[]`。
    preferences: Vec<Preference>,
    /// §227 的 `unknowns[]`。**非 `Option`，非 `Vec` 的裸形态**——见 [`Unknowns`]。
    unknowns: Unknowns,
}

impl Specification {
    /// 六个分量 → 一枚 `Specification`。**`pub(crate)`**——crate 外的唯一构造路径是
    /// [`SpecDraft::finalize`]（`tests/compile_fail/a_specification_cannot_be_constructed_outside_the_crate.rs`
    /// 钉的就是「外面拼不出字面量」）；crate 内的第二个用点是 `persist.rs` 的行解码
    /// （读回来的记录需要一个构造点，同 `CanonicalRepresentation::new` 的收法）。
    pub(crate) fn new(
        goals: Vec<Goal>,
        invariants: Vec<Invariant>,
        acceptance_predicates: Vec<AcceptancePredicate>,
        forbidden_states: Vec<ForbiddenState>,
        preferences: Vec<Preference>,
        unknowns: Unknowns,
    ) -> Self {
        Specification {
            goals,
            invariants,
            acceptance_predicates,
            forbidden_states,
            preferences,
            unknowns,
        }
    }

    /// §227 的 `goals[]`，按写入顺序。
    pub fn goals(&self) -> &[Goal] {
        &self.goals
    }

    /// §227 的 `invariants[]`，按写入顺序。
    pub fn invariants(&self) -> &[Invariant] {
        &self.invariants
    }

    /// §227 的 `acceptance_predicates[]`，按写入顺序。
    pub fn acceptance_predicates(&self) -> &[AcceptancePredicate] {
        &self.acceptance_predicates
    }

    /// §227 的 `forbidden_states[]`，按写入顺序。
    pub fn forbidden_states(&self) -> &[ForbiddenState] {
        &self.forbidden_states
    }

    /// §227 的 `preferences[]`，按写入顺序。
    pub fn preferences(&self) -> &[Preference] {
        &self.preferences
    }

    /// §227 的 `unknowns[]` 的那一枚（`Enumerated` / `NoneFound`）。
    pub fn unknowns(&self) -> &Unknowns {
        &self.unknowns
    }
}

/// Specification 的**草案**：五个产物列表的半成品，**没有 `unknowns` 字段**。
///
/// # 为什么字段是 `pub`（而 [`Specification`] 的不是）
///
/// 区别在于**有没有不变量可挂**（同 `continuum-canonical` 的 `Candidate` 与
/// `CanonicalRepresentation` 的对照）：草案是一个可自由拼装、逐项添加的中间物，
/// 本层没有比 §214 更强的约束可挂；而 [`Specification`] 挂的正是「必须显式给出
/// `unknowns`」这一条（§227 的 MUST），故它锁起来。
///
/// # 为什么没有 `unknowns` 字段
///
/// 这就是「没查」**不可表达**的机制：草案里没有地方可以省掉它——`unknowns` 只在
/// [`SpecDraft::finalize`] 的**入参**上出现，且是必填的 `Unknowns`（不是 `Option`）。
/// 照片：`tests/compile_fail/a_draft_without_unknowns_cannot_be_finalized.rs`。
#[derive(Debug, Clone)]
pub struct SpecDraft {
    /// 草案的 `goals`。
    pub goals: Vec<Goal>,
    /// 草案的 `invariants`。
    pub invariants: Vec<Invariant>,
    /// 草案的 `acceptance_predicates`。
    pub acceptance_predicates: Vec<AcceptancePredicate>,
    /// 草案的 `forbidden_states`。
    pub forbidden_states: Vec<ForbiddenState>,
    /// 草案的 `preferences`。
    pub preferences: Vec<Preference>,
}

impl SpecDraft {
    /// 定稿：把草案与**显式给出的** `unknowns` 合成一枚 [`Specification`]。
    ///
    /// **入参不是 `Option<Unknowns>`**（§227 的两句 MUST + 设计 §6.1）：
    /// `None` 若能表示「没查」，§227 要挡的那条路就重新变得可表达。
    /// **本函数不失败**——五个成员类型与 `Unknowns` 都是纯值，没有可拒绝的输入
    /// （同 [`Specification`] 各成员类型的「无校验」承诺）。
    pub fn finalize(self, unknowns: Unknowns) -> Specification {
        Specification::new(
            self.goals,
            self.invariants,
            self.acceptance_predicates,
            self.forbidden_states,
            self.preferences,
            unknowns,
        )
    }
}

/// §227 两句 MUST 的落点：`Enumerated` 是「**显式枚举出的**未知项」，
/// `NoneFound` 是「**已查过、没有**」。
///
/// # 为什么不是 `Vec<Unknown>`
///
/// `vec![]` 同时表示「已查过、没有」与「没查」，而 §227 禁的正是后者被当成前者
/// （「系统 MUST NOT 把未知项自动转换成假设」）。两枚的枚举让那次区分**可表达**，
/// 落库形态的区分由 `persist.rs` 的 `unknowns_kind` 列承载——
/// 照片：`tests/spec.rs` 的
/// `none_found_and_an_empty_enumerated_are_distinguishable_after_a_round_trip`
/// （`NoneFound` 与 `Enumerated(vec![])` 各写一次、读回**各自那一枚**）。
///
/// **恰两枚**是编译期照片：`tests/spec.rs` 的 `kind_of`（与 `tests/type_level.rs`
/// 那边的清单无关）用穷尽 `match` **不带 `..`** 钉住——加第三枚即**编译不过**。
#[derive(Debug, Clone)]
pub enum Unknowns {
    /// 显式枚举出的未知项（**空列表**就是「已查过、枚举为空」，与 [`Unknowns::NoneFound`]
    /// 是两枚不同的值）。
    Enumerated(Vec<Unknown>),
    /// 「已查过、没有未知项」。
    NoneFound,
}

impl Unknowns {
    /// `unknowns_kind` 列的落库字面量（小写、多词接 `_`）。
    ///
    /// **`pub(crate)`**：外部可观察的用处只有 `specification` 表那一列（要落库就调
    /// [`crate::save_intent_with_specification`]；要读就调
    /// [`crate::load_specification`]），同 [`crate::IntentKind::as_str`] 的收法。
    /// **无通配臂**：加第三枚即编译不过。解码走 [`Unknowns::parse_kind`]。
    pub(crate) fn kind_str(&self) -> &'static str {
        match self {
            Unknowns::Enumerated(_) => "enumerated",
            Unknowns::NoneFound => "none_found",
        }
    }

    /// [`Unknowns::kind_str`] 的逆——只回答「这个串是两枚中哪一枚的落库字面量」。
    ///
    /// # 为什么是 `Option<UnknownsKind>` 而不是 `Option<Unknowns>`
    ///
    /// `Enumerated` 带载荷（列表要经容器解码才拿得到），**装不进 `Option<Self>`**——
    /// 故判别式单列一个小枚举 [`UnknownsKind`]；列表的容器解码留在 `persist.rs` 的
    /// `decode_unknowns`（那是容器层的活，本文件不碰 JSON）。
    ///
    /// # 形状照 [`crate::IntentKind::parse`]
    ///
    /// **定义域是所有串**，认不出就是 `None`；返回 `Option` 而不返回一枚「未知」变体——
    /// 本函数没有失败原因可给（它不读库、不写库）。`None` 转成哪个错误由调用方的解码处
    /// 决定（`continuum-semantics` 的 `persist.rs` 转成 [`crate::SpecError::UnknownsKind`]，
    /// 带原始取值）。
    ///
    /// **带通配臂是刻意的**（与 [`Unknowns::kind_str`] 的无通配相反）：这一边的输入是
    /// **开放集**（任意串），逐枚列出永远列不完。
    pub(crate) fn parse_kind(kind: &str) -> Option<UnknownsKind> {
        match kind {
            "none_found" => Some(UnknownsKind::NoneFound),
            "enumerated" => Some(UnknownsKind::Enumerated),
            _ => None,
        }
    }
}

/// [`Unknowns::parse_kind`] 的返回值：「这个 `unknowns_kind` 串是哪一枚」。
///
/// **`pub(crate)`、不外泄**：它不是第二份数据模型——[`Unknowns`] 带载荷的那一支
/// （`Enumerated`）装不进 `Option<Unknowns>`，故白名单判定单列一个判别式，
/// 列表的容器解码留在 `persist.rs`。**不落库、不被 crate 外看见。**
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UnknownsKind {
    /// 对应 [`Unknowns::Enumerated`]。
    Enumerated,
    /// 对应 [`Unknowns::NoneFound`]。
    NoneFound,
}

/// §214 的 `Unknown`：一个未知项。今天的成员恰两类——[`Conflict`] 与
/// [`UndefinedBehavior`]（§6.1 的「只建类型」）。
///
/// 两枚变体**逐项**有照片（`tests/spec.rs` 的
/// `conflict_and_undefined_behaviour_are_types_only`：两枚各写一次、读回、逐项断言）。
/// **无通配臂**的 [`Unknown::kind_str`] 是它的编码（加第三类即编译不过）；
/// 解码走带通配臂的 [`Unknown::parse_kind`]（输入是开放集）。
#[derive(Debug, Clone)]
pub enum Unknown {
    /// 检测冲突的产物——**只有类型，没有检测器**（§6.1；理由见 [`Conflict`]）。
    Conflict(Conflict),
    /// 检测未定义行为的产物——**只有类型，没有检测器**（理由见 [`UndefinedBehavior`]）。
    UndefinedBehavior(UndefinedBehavior),
}

impl Unknown {
    /// `unknowns` 列 JSON 里元素的 `kind` 落库字面量（小写、多词接 `_`）。
    ///
    /// 同 [`Unknowns::kind_str`]：`pub(crate)`、无通配臂。解码走 [`Unknown::parse_kind`]。
    pub(crate) fn kind_str(&self) -> &'static str {
        match self {
            Unknown::Conflict(_) => "conflict",
            Unknown::UndefinedBehavior(_) => "undefined_behavior",
        }
    }

    /// [`Unknown::kind_str`] 的逆（连同载荷）：`unknowns` 列 JSON 元素的两半 →
    /// 一枚 `Unknown`。
    ///
    /// # 形状照 [`crate::IntentKind::parse`]
    ///
    /// **定义域是所有 `(串, 串)` 对**，认不出就是 `None`；返回 `Option` 而不返回一枚
    /// 「未知」变体——本函数没有失败原因可给（它不读库、不写库）。`None` 转成哪个错误由
    /// 调用方的解码处决定（`continuum-semantics` 的 `persist.rs` 转成
    /// [`crate::SpecError::UnknownItemKind`]，带原始取值）。
    ///
    /// **带通配臂是刻意的**（与 [`Unknown::kind_str`] 的无通配相反）：这一边的输入是
    /// **开放集**（任意串），逐枚列出永远列不完。
    pub(crate) fn parse_kind(kind: &str, value: &str) -> Option<Unknown> {
        match kind {
            "conflict" => Some(Unknown::Conflict(Conflict::from_str(value))),
            "undefined_behavior" => {
                Some(Unknown::UndefinedBehavior(UndefinedBehavior::from_str(value)))
            }
            _ => None,
        }
    }

    /// 这一枚的说明文本（不透明，见 [`Conflict`] / [`UndefinedBehavior`]）。
    pub(crate) fn description(&self) -> &str {
        match self {
            Unknown::Conflict(c) => c.as_str(),
            Unknown::UndefinedBehavior(u) => u.as_str(),
        }
    }
}

/// §214 的「检测冲突」的产物类型。**不建检测器**。
///
/// # 为什么只有一个不透明说明
///
/// §214 的「什么算冲突」**规范没给判据**（设计 §16 第 10 条：「缺的是『什么算冲突』
/// 这一步。收件人：规范维护者」）。故本层不发明结构（例如「冲突双方是谁」的字段表
/// 就是发明）——只把一个不透明串保存下来，判定归将来拿到判据的那一方。
/// 与 [`Goal`] 等成员类型同一个处置（包 [`Opaque`]）。
#[derive(Debug, Clone)]
pub struct Conflict(Opaque);

impl Conflict {
    /// 取里子串。**不改写、不裁剪**。
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// 从任意串构造。**无校验**（同 [`Opaque::from_str`]）。
    pub fn from_str(s: &str) -> Self {
        Conflict(Opaque::from_str(s))
    }
}

/// §214 的「检测未定义行为」的产物类型。**不建检测器**。
///
/// 不建检测器的理由与 [`Conflict`] 逐字相同（设计 §16 第 10 条）。
#[derive(Debug, Clone)]
pub struct UndefinedBehavior(Opaque);

impl UndefinedBehavior {
    /// 取里子串。**不改写、不裁剪**。
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// 从任意串构造。**无校验**。
    pub fn from_str(s: &str) -> Self {
        UndefinedBehavior(Opaque::from_str(s))
    }
}

/// §214 的 `Goal`。**§214 只给名字、未给结构**——本层取不透明串，不发明字段。
///
/// # 为什么不直接取 [`Opaque`] / `String`
///
/// 五个成员类型（[`Goal`] / [`Invariant`] / [`AcceptancePredicate`] /
/// [`ForbiddenState`] / [`Preference`]）今天都是不透明串，但它们是**六元组的五个
/// 不同位置**——合并成一个类型会让「把 preference 填进 goals」在类型上可写。
/// 与 [`crate::Priority`] 的处置同一条理由；代价是每类型各写一遍那两个方法，
/// 那是**一分不值的重复**（没有不变量、没有编码差异），换来的是**位置互换在编译期被挡**。
///
/// **「不改写、不裁剪」「无校验」的照片**：`tests/spec.rs` 的往返用例让五个类型各自
/// 过「什么字符都可能有」的串（`a_specification_has_exactly_the_six_tuple_fields`
/// 与 `save_intent_with_specification` 的编码路径），逐字节进什么出什么。
#[derive(Debug, Clone)]
pub struct Goal(Opaque);

impl Goal {
    /// 取里子串。**不改写、不裁剪**。
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// 从任意串构造。**无校验**（本类型没有「合法取值」这个概念）。
    pub fn from_str(s: &str) -> Self {
        Goal(Opaque::from_str(s))
    }
}

/// §214 的 `Invariant`。**只给名字、未给结构**——处置与理由见 [`Goal`]。
#[derive(Debug, Clone)]
pub struct Invariant(Opaque);

impl Invariant {
    /// 取里子串。**不改写、不裁剪**。
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// 从任意串构造。**无校验**。
    pub fn from_str(s: &str) -> Self {
        Invariant(Opaque::from_str(s))
    }
}

/// §214 的 `Acceptance Predicate`。**只给名字、未给结构**——处置与理由见 [`Goal`]。
#[derive(Debug, Clone)]
pub struct AcceptancePredicate(Opaque);

impl AcceptancePredicate {
    /// 取里子串。**不改写、不裁剪**。
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// 从任意串构造。**无校验**。
    pub fn from_str(s: &str) -> Self {
        AcceptancePredicate(Opaque::from_str(s))
    }
}

/// §214 的 `Forbidden State`。**只给名字、未给结构**——处置与理由见 [`Goal`]。
#[derive(Debug, Clone)]
pub struct ForbiddenState(Opaque);

impl ForbiddenState {
    /// 取里子串。**不改写、不裁剪**。
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// 从任意串构造。**无校验**。
    pub fn from_str(s: &str) -> Self {
        ForbiddenState(Opaque::from_str(s))
    }
}

/// §214 的 `Preference`。**只给名字、未给结构**——处置与理由见 [`Goal`]。
#[derive(Debug, Clone)]
pub struct Preference(Opaque);

impl Preference {
    /// 取里子串。**不改写、不裁剪**。
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }

    /// 从任意串构造。**无校验**。
    pub fn from_str(s: &str) -> Self {
        Preference(Opaque::from_str(s))
    }
}

// ============================================================================
// 下面这个 `#[cfg(test)]` 模块里的用例机制是**对 `Specification` 穷尽解构、
// 不带 `..`**——而穷尽解构要在字面量里命名字段，字段私有后只有 crate 内做得到
// （本仓已有「照片住在 `src` 的 `#[cfg(test)]` 里」的体例，见
// `continuum-canonical/src/resolver.rs`）。
// ============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    /// §227 六元组的字段清单**逐项**：机制 = 穷尽解构、**不带 `..`**。
    ///
    /// 加／少／改一个字段即**编译不过**（按本仓纪律，编译不过不算「变红」，
    /// 故这里是**编译期照片**：运行期断言为空，承重在编译器）。
    /// 运行期那一半（经六个访问器读回内容）在 `tests/spec.rs` 的
    /// `a_specification_has_exactly_the_six_tuple_fields`；
    /// crate 外那一半（拼字面量编不过）在
    /// `tests/compile_fail/a_specification_cannot_be_constructed_outside_the_crate.rs`。
    #[test]
    fn the_six_tuple_fields_are_exhaustively_destructured() {
        let spec = SpecDraft {
            goals: vec![Goal::from_str("g")],
            invariants: vec![Invariant::from_str("i")],
            acceptance_predicates: vec![AcceptancePredicate::from_str("a")],
            forbidden_states: vec![ForbiddenState::from_str("f")],
            preferences: vec![Preference::from_str("p")],
        }
        .finalize(Unknowns::NoneFound);

        let Specification {
            goals,
            invariants,
            acceptance_predicates,
            forbidden_states,
            preferences,
            unknowns,
        } = spec;
        // `let _ = (…)` 只是把绑定消费掉，免得 unused-variable 警告。
        let _ = (
            goals,
            invariants,
            acceptance_predicates,
            forbidden_states,
            preferences,
            unknowns,
        );
    }
}
