//! §272 的候选面：`EntityId`、`Candidate`、`CandidateSource` 与装配方的 `CandidateSources`。
//!
//! §272（`docs/spec/05-normative.md:1378-1392`）给的是一个**数据记录**的三字段
//! （`entity_id` / `evidence[]` / `retrieval_score?`）与一句禁令；
//! §197（`docs/spec/04-method.md:514-551`）给的是**七种手段**的名字与一条分工
//! （「高召回候选生成，而不是直接决定最终语义」，同文件 `:542-548`）；
//! §201（同文件 `:662-701`）把核心语义定成「`Candidate Set` → 显式约束 → …」。
//! 设计 §4.3（`docs/superpowers/specs/2026-10-06-p4-semantic-layer-design.md:700-729`）
//! 把这三处折成本模块的四个类型。**上面三处行号是本 task 实测的**
//! （`grep -n '^## 27[12]\.'` 与 `grep -n '^## 19[67]\.'`），不是从设计文档转抄的；
//! 引用一律按节号取（§272／§201／§197），行号只作定位用——行号会随那两份文件的下一次编辑漂移。
//!
//! # 本模块只有它自己，不碰 Task 2 的产物
//!
//! `Candidate` 的 `retrieval_score` **不出现在已解出的结果上**（§4.4 第 1 条）——
//! 那一条的落点在 Task 4 的 `ResolutionResult` 字段清单上，本模块只负责「分数只长在候选上」这半边。
//! 同理，`EntityId` 今天**没有任何消费者**：第一个用它的是 Task 4 的 `Resolved { entity, … }`
//! 与 Task 7 的 alias 表；本 task 只定义它、并为它拍下「crate 外没有公开字面量构造」那张照片
//! （`tests/compile_fail/entity_id_has_no_public_literal_construction.rs`）。

use crate::Mention;

/// §272 的 `entity_id`：**不透明 newtype**，不是 `ToolId` / `ModelId` / `ConnectorId` 中的任何一个。
///
/// # 为什么是不透明 newtype（理由不是偏好）
///
/// 那三个 id 住在**资源层／边界层**（`continuum-model-registry` / `continuum-capability` /
/// `continuum-connector`），引用它们就要登记**指向那些 crate 的依赖边**，
/// 而设计 §1.2.3 的规则 3（`design:99-138`）要求本层**不登记任何指向执行层／资源层／边界层的边**。
/// 这条规则在仓里有既有的守卫：`crates/continuum-runtime/tests/dependency_direction.rs` 的
/// `ALLOWED` 逐对断言，本 crate 的允许集是**空条目**
/// （`dependency_direction.rs:156`，行号本 task 实测）。
///
/// 于是「mention 解析出的实体」与「注册表里的 `ToolId`」之间的对应由**装配方**维持。
/// **代价照实写**：这份对应今天只活在装配方的代码里，它错了本层看不出来
/// （可能静默失配——两个 crate 各自有一个串，长得一样不代表是同一个东西）。
/// **对冲在 Task 5**：§4.2 的断言 A（`Resolved(e)` 蕴含 `e ∈ 检索集 ∪ {注入的那一枚}`，
/// 比的是**同一个 `EntityId` 值**）把「解析器自造实体」那条路堵住；
/// 它堵不住装配方自己给错，那是另一件事，本层没有判据。
///
/// # 不提供语义能力
///
/// **没有 `PartialEq` / `Eq`**（也就没有 `Ord` / `PartialOrd` / `Hash`）：与 [`crate::Opaque`]
/// 同一条理由——设计没有给「两个实体是同一个」以外的任何判据，而本类型今天连那一条也没有
/// （它只是一个串）。要比较就经 [`EntityId::as_str`]，调用点看得到自己在比什么。
/// 加 `PartialEq` 是**一个可以随时加的表达力**，现在加了，等于凭空给出一枚没有用例的表达力
/// （本仓「枚举断言须逐项有照片」的同一条纪律：表达力要么有照片，要么先不加）。
///
/// **`Debug` 与 `Clone` 不是语义能力**：前者是断言失败时必须打得出来，
/// 后者是 `Candidate` 要能装进 `Vec` 并随值传递（设计 §1.2 的「以值进入」；
/// §4.2 的 `let working = retrieved.clone();` 就要它）。
#[derive(Debug, Clone)]
pub struct EntityId(String);

impl EntityId {
    /// 取里子串。**不改写、不裁剪、不折叠大小写**。
    ///
    /// 规范化发生在**更前面**（[`crate::normalize`]）：进了本层的串都已经过了那四步折叠。
    /// 本类型若再折一次，就会出现「同一个实体的两种写法在本层是两个值」这种**本层没有判据**的后果
    /// （本处不是 `Surface` 的第二遍：那一遍的判据是 §270 给的，这一遍没有任何来源）。
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// 从任意串构造。**无校验、无失败路径**——本类型没有「合法取值」这个概念
    /// （§272 只给了字段名，没给取值域；§4.3 也没给）。
    ///
    /// **空串是被接受的**：它往返得空串。拒掉空串会是一条本层没有判据的收紧，
    /// 且会把「空串」与「没有实体」两件事混起来——**没有实体**这一态属于 §4.1 的四态
    /// （`Unknown`），不属于本类型。`tests/candidate.rs` 的
    /// `entity_id_round_trips_and_has_no_public_literal_construction` 把这条处置
    /// **具体地**断言出来了（不是只断言「不 panic」）。
    ///
    /// 名字用 `from_str` 而**没有实现 `std::str::FromStr`**：后者要求一个 `Err` 关联类型，
    /// 而本类型**不失败**。为一个恒成功的转换实现 `FromStr` 会把「可能失败」写成谎话
    /// （与 [`crate::MentionKind::from_str`] 同一条理由）。
    ///
    /// **它也是 crate 外唯一的构造入口**：里子串私有，故元组构造
    /// `EntityId("x".to_owned())` 在 crate 外写不出来
    /// （`tests/compile_fail/entity_id_has_no_public_literal_construction.rs`，
    /// 正控制是 `tests/compile_pass/entity_id_from_str_is_constructible.rs`）。
    pub fn from_str(s: &str) -> Self {
        EntityId(s.to_owned())
    }
}

/// §272 的候选。三个字段照录，**没有第四个**。
///
/// # 字段是 `pub` 的（而 [`crate::CanonicalRepresentation`] 的不是）
///
/// 这里不是「本层对数据记录就宽松」：区别在于**有没有不变量可挂**。
/// `CanonicalRepresentation` 的字段私有，是因为「它里面不能装着未解 mention」这条不变量
/// 只能由构造面守（§3.2 的两支互斥）；而 `Candidate` 是 §272 逐字给出的记录，
/// 本层没有比 §272 更强的约束可挂——把它锁起来只会让每个来源多写一个构造函数，
/// 而不变量一条也不会多。故这里取 `pub`，并且**这条选择本身有照片**：
/// `tests/candidate.rs` 的 `the_candidate_records_exactly_the_three_fields_of_272`
/// 用**不带 `..`** 的穷尽解构钉住字段清单恰好是这三个。
///
/// # `retrieval_score` 只长在候选上
///
/// §272 说它 MAY 用于排序、MUST NOT 自动被当成真实概率。设计 §4.4（`design:731-748`）
/// 把这条禁令折成**类型的位置**：分数出现在 [`Candidate`] 上，**不出现在已解出的结果上**
/// （Task 4 的 `ResolutionResult`）。于是「拿分数再判一次」这条路在**类型**上就走不通——
/// 一个 `Resolved` 值携带不了分数。Task 4 那侧另有一条字段清单断言钉同一件事的另一半。
///
/// **本层不对 `retrieval_score` 做任何解释**：它没有取值域（§272 未给）、不被归一、
/// 不被换算成概率（本层连一个读它的函数都没有）。它在本层是**原样保存的事实**——
/// 排序是消费者的事，而今天没有消费者。
#[derive(Debug, Clone)]
pub struct Candidate {
    /// 哪一枚实体。见 [`EntityId`]（不透明 newtype）。
    pub entity_id: EntityId,
    /// §272 的 `evidence[]`：候选的来源凭据。**本层不解释它的元素**
    /// （规范只给了字段名，没给「一条凭据长什么样」），故取 `Vec<String>` 原样存。
    pub evidence: Vec<String>,
    /// §272 的 `retrieval_score?`：**可选**——「来源没给分数」与「来源给了 0 分」是两件事，
    /// 由一个**默认 `0.0`** 抹掉这个区别的地方，下游任何按分数排序的代码都会把前者当成后者。
    /// `tests/candidate.rs` 的 `a_score_is_optional_and_absent_is_not_zero` 断言两者可区分。
    pub retrieval_score: Option<f64>,
}

/// §197 的候选来源接口。**返回值只能是 `Vec<Candidate>`——没有任何返回单一实体的方法。**
///
/// # 为什么方法集就这么大
///
/// §197（`docs/spec/04-method.md:542-548`）把分工写死了：这些手段负责
/// 「**高召回候选生成**，而不是直接决定最终语义」。一个返回单实体的方法
/// （`pick_one(&Mention) -> Candidate` 之类）会把「挑哪一枚」这件事提前搬到来源里，
/// 而**挑**正是 §201 的约束解析要干的事（`Candidate Set` → 显式约束 → …），
/// 它需要的事实（显式约束、注册表关系、作用域）来源手上没有。故本层不提供这个口子。
///
/// 这条守卫**有照片**，且**两侧都钉**
/// （`tests/type_level.rs` 驱动，样例在 `tests/compile_fail/` 与 `tests/compile_pass/`）：
/// 反例 `source.pick_one(&m)` 编不过；正例「只实现 `retrieve` 的实现」编得过
/// ——后者钉的是**必需**方法集恰好是那一个（只钉反例的话，
/// 把 `retrieve` 改名、或再加一个必需方法，都仍会绿）。
///
/// # 这条守卫的限度（写出来，免得被读成覆盖得更广）
///
/// 它钉的是「**本层**不为单实体定义方法」，**不钉**「实现者内部返回唯一候选」。
/// 一个来源内部只交出一枚候选，本层看不出、也不该看出：§197 的高召回是**手段**层面的性质，
/// 而本层没有召回率判据（设计 §15.2 第 3 条，`design:2264-2266`）。
/// 「候选集非空且唯一合格才 `Resolved`」那一侧由 Task 5 的四条断言在**解析侧**钉。
///
/// # 本层不提供任何默认实现，也不内置任何一个来源
///
/// §197 列了七种手段（edit distance / phonetic similarity / ASR hypotheses / pinyin /
/// embedding retrieval / alias registry / entity registry），其中**只有 alias registry 在本层**
/// （Task 7）。其余六种是**外部**来源，以 `dyn CandidateSource` 的形式由调用方装配。
/// **真实来源今天零个**（设计 §16 第 5 条，`design:2331-2334`），故本层若内置一个「示例来源」，
/// 那个来源就是凭空发明的一个判据；若提供默认实现，则「忘装配」会静默变成「零候选」而不是编不过。
pub trait CandidateSource {
    /// 给一枚 mention，交出**一批**候选（可以是空集，也可以是多枚）。
    ///
    /// 空集是合法返回：§201 的「0 candidate → NIL / Expand Retrieval」正是从空集起手的
    /// （它的处置在 §4.2 的断言 C，Task 5），故这里不把空集当异常、不 panic、不造占位候选。
    ///
    /// **本方法今天没有任何调用方**：解析器（Task 4/5）还没建，真实来源也零个。
    /// 这不影响接口的形状——本层今天定的是 §197 的分工面，不是某一次解析的调用链。
    fn retrieve(&self, m: &Mention) -> Vec<Candidate>;
}

/// 装配方交出的若干来源（§4.3：候选集以**值**进入本层）。
///
/// 设计 §4.3 给它的全部内容就是「由装配方给出」这一句；**方法集是本层自定的**
/// （见 [`CandidateSources::retrieve_all`] 的文档，那里写明拼接口径的来历与边界）。
///
/// 不派生 `Debug`：里子装的是 `Box<dyn CandidateSource>`，而 [`CandidateSource`] 没有
/// `Debug` 上界。给 trait 加一个 `Debug` 上界会**对所有装配方**施加一条没有判据的要求
/// （本层没有一处要打印来源），故这里宁可让 `CandidateSources` 不带 `Debug`。
#[derive(Default)]
pub struct CandidateSources {
    sources: Vec<Box<dyn CandidateSource>>,
}

impl CandidateSources {
    /// 按装配顺序收下一批来源。
    ///
    /// **`new` 与 [`Default`] 同义**（零来源 = 空装配）。两条都留着：`Default` 让
    /// `CandidateSources::default()` 在「先建空的、再逐个装」的装配代码里读起来更顺，
    /// 而 `new` 是收下一整批的那一条。**这里不是「两个入口各有一套语义」**：它们都只建一个容器，
    /// 容器本身没有不变量（零来源是合法的，见 `retrieve_all`）。
    pub fn new(sources: Vec<Box<dyn CandidateSource>>) -> Self {
        CandidateSources { sources }
    }

    /// 把每个来源交出的候选**按装配顺序拼接**，得到 §4.2 断言 A 所说的**检索集**
    /// （`design:608-698`；A 的闭集判据由 Task 5 钉，本函数只负责「检索集是怎么来的」）。
    ///
    /// # 拼接口径是本层自定的（三条，都写出来）
    ///
    /// 1. **顺序 = 装配顺序**，来源内部的顺序照原样。**不排序**：
    ///    §272 说 `retrieval_score` **MAY** 用于排序，那是**消费者**的选择，
    ///    且本层的检索集还没经过任何门槛判定；在装配处先排一次，会把
    ///    「来源给的顺序」这一份事实抹掉，而下游再排一次也回不到它。
    /// 2. **不去重**：两个来源各自给出同一个 `entity_id` 时，两枚都在。
    ///    §197 的七种手段是**互相独立的高召回来源**，本层**没有判据**决定它们的票怎么合
    ///    （合并分数、取最大、取并集——三条都是本层编出来的判定）。去重会把
    ///    「两个手段各自认为可能」压成一件事，而那份重叠本身是下游可能要看的事实。
    /// 3. **不解释 `evidence` / `retrieval_score`**：原样带过去。
    ///
    /// **这里不是「把候选集合并成一个」**：本函数的返回值仍然是**候选集**，
    /// 它没有挑出任何一枚——「挑」属于 §201 的约束解析（Task 5）。
    ///
    /// 零来源 ⇒ 空向量。§201 的「0 candidate」由此起手，本层**不**在这里造占位候选
    /// （§273 要求每次解析保留 `UNKNOWN / NIL` 的可能，造占位候选正是那句话要防的）。
    pub fn retrieve_all(&self, m: &Mention) -> Vec<Candidate> {
        let mut out = Vec::new();
        for source in &self.sources {
            out.extend(source.retrieve(m));
        }
        out
    }
}
