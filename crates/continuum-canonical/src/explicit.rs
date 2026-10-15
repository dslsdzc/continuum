//! §4.1.1 的显式约束面：`ExplicitConstraint` 三枚、候选的判定事实 `CandidateFacts`，
//! 以及「满足」这个判据。
//!
//! # 规范给到哪一步（先说清，因为下面几乎每一句都是本设计的决定）
//!
//! 设计 §4.1.1（`docs/superpowers/specs/2026-10-06-p4-semantic-layer-design.md:450-606`，
//! 行号本 task `grep -n` 实测）自己写明：规范只给了**流程名**（§201 的
//! `Candidate Set → Explicit Constraints → …`）、**一组值域**（§274 的九个作用域）、
//! **一个形状**（§277 的 `ExplicitBinding`）与**一句性质**（§274 的「显式 scope 优先级最高」）；
//! **没给**约束的类型、能用哪几种、以及「满足」怎么判。故本文件里的三处判据
//! ——**约束是闭集三枚**、**判据是合取**、**`Scope` 是等值不是层级**——都是设计的决定，
//! 来历逐条写在各自的注释里，不重抄到本文件头（同一句话两个家会静默失真）。
//!
//! # 本模块不查任何东西
//!
//! [`CandidateFacts`] 由调用方**与候选一并给出**：注册表不在本层（设计 §1.2.3 的规则 3），
//! 故 `scope` 与 `kind` 只能随值进来。这一条不是措辞——它是签名：本模块里
//! **没有任何函数收 `&Tx`、也没有任何函数收 `dyn CandidateSource`**（对比 §4.3 的
//! [`crate::CandidateSources`]，那是**装配方**交给解析器的候选集，不是本模块的输入）。
//! 「本层查不到它们」这句话因此有一个可核的形状：喂进两枚**同一个 `Candidate`** 而
//! `CandidateFacts` 不同，`qualifying` 的结果必须不同（`tests/explicit.rs` 的同名用例）。

use crate::EntityId;
use crate::candidate::Candidate;

/// §274 的九个作用域（`docs/spec/05-normative.md:1408-1426`）。**九值封闭**。
///
/// # 它是 §274 的那九枚，一个不多一个不少
///
/// §274 的原文是一个九行的代码块，**没有别的取值**。本类型**只取那九个名字**：
/// 那句「显式 scope 优先级最高」**不归这里**——设计 §4.1.1（`design:526-537`）
/// 写明它是一条**决胜规则**（多个 binding 落在不同 scope 时取哪一个），
/// **不是准入规则**（哪些候选合格）。故本类型里没有「优先级」这一维、
/// 也没有任何函数用它排序。
///
/// # 派生里的哪一枚是判据要的
///
/// `PartialEq` / `Eq` 是**判据本身要的**：[`satisfies`] 对 `Scope` 做的是等值过滤
/// （`f.scope == *s`），没有它就写不出这条判据。`Debug` 是断言失败时打得出来，
/// `Clone` 是随值传递。
///
/// **不是 `Ord` / `PartialOrd`**：§274 的九个值是**一条优先级层级**，
/// 而**「哪一枚比哪一枚更广」的序规范没有给**（§274 只给了九个名字与那一句决胜规则）。
/// **本层不发明那个序**——给出了 `Ord`，就等于给出了一个没有规范出处的层级，
/// 而它会被下一个人当作判据用。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolutionScope {
    /// §274 的 `explicit`。
    Explicit,
    /// §274 的 `session`。
    Session,
    /// §274 的 `conversation`。
    Conversation,
    /// §274 的 `project`。
    Project,
    /// §274 的 `workspace`。
    Workspace,
    /// §274 的 `tool`。
    Tool,
    /// §274 的 `plugin`。
    Plugin,
    /// §274 的 `user`。
    User,
    /// §274 的 `global`。
    Global,
}

/// 候选的实体种类。**闭集四枚**——**不是 §198 的三枚**（`docs/spec/04-method.md:552-593`）。
///
/// # 为什么必须有第四枚 `ConnectorService`（这一枚是补的）
///
/// 设计 §16 第 26 条（`design:2431-2444`）本层**认领了「服务名作为 mention 的解析目标」那一半**，
/// 并写明对它的处理**与其它实体同形**。而 `GitHub` / `Email` 这类服务名
/// **不是 `Tool` / `Skill` / `Relation` 中任何一个**——只有三枚时，调用方只能**硬塞**一枚进去，
/// 于是**静默错型，且没有任何断言会红**（本仓最忌的形状：没有照片的失真）。
///
/// **为什么不是「三枚 ＋ `Opaque`」**：那会让本类型**不再是闭集**，
/// 而 §4.1.1 里那些逐枚断言（「三枚各一条」「四枚各一条」）**全建立在闭集上**。
///
/// **第四枚的出处**：§124（`docs/spec/02-positioning.md:1803-1826`）的六项服务与
/// §125（同文件 `:1828`，标题为 Connector Permission，落在服务目录那一支）的「按操作细分」；
/// §124 明写「Connector 不等于 Agent，它只是 Capability Provider」，
/// 故服务名是一个**有自身身份**的解析目标。
/// **§198 的引用只取词汇、不取算法**：§198 是设计 §16 第 4 条判「没有落点、未实现」的那一节
/// （联合解析算法本层不建，`design:2305-2309`），此处只借它的三个候选名当词。
///
/// # 失效条件（照录设计，免得被读成本层已覆盖更广）
///
/// **若规范再点名一类解析目标，照 `ConnectorService` 的先例补一枚，不得塞进现有三枚**
/// ——判据就是这一枚存在的理由（硬塞是静默错型）。设计点名的第一个候选是 §222 的 `product_id`
/// （§5.1 把它取作不透明 `Opaque`，今天不与本类型相交）。**收件人：规范维护者 ＋ 实现者。**
///
/// # 「恰好四枚」的照片
///
/// **编译期**：`tests/explicit.rs::entity_kind_has_exactly_four_variants` 里那个
/// **不带 `..`** 的穷尽 `match` ——加第五枚即**编译不过**（按本仓纪律，**编译不过不算变红**，
/// 故这一半是**类型保证**）。
/// **运行期**：同一用例逐枚断言「四枚的名字两两不同、且各自映到自身」——
/// 它钉不住「没有第五枚」（那件事运行期不可见），钉的是**这四枚各自可分辨**。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntityKind {
    /// §198 的 `ToolCandidate` 那一枚（只取词汇，见上）。
    Tool,
    /// §198 的 `SkillCandidate` 那一枚（只取词汇，见上）。
    Skill,
    /// §198 的 `Relation` 那一枚（只取词汇，见上）。
    Relation,
    /// **连接器服务名**（`GitHub` / `Email` / …）。**这一枚是补的**，出处不在 §198，见上。
    ConnectorService,
}

/// 一条显式约束。**取值域是闭集，三枚**——每一枚都能追到规范里已有的一个概念
/// （作用域 §274、实体种类 §124／§125、手工绑定 §277），
/// **不引入开放谓词语言**：一个「任意谓词」的接口会让 §4.2 的四条断言全部失去判据
/// （「满足」变成调用方定义的，本层就无从断言 B／D）——设计 §4.1.1 末段把这条写死了
/// （`design:604-606`）：**闭集是让断言可写的前提**。
///
/// # `BoundTo` 的判据经 `as_str` 比，不经 `==`
///
/// §277 的 `ExplicitBinding { mention, entity, scope, source = USER }` 里 `entity` 是实体，
/// 这里也只取实体（`scope` 那一维归 `Scope` 那一枚，`source = USER` 归 Task 6 的 binding）。
/// 与 [`EntityId`] 的比较走 `as_str()`：`EntityId` **没有** `PartialEq`
/// （`src/candidate.rs` 的理由：设计没给「两个实体是同一个」以外的任何判据），
/// 故调用点看得到自己在比**那两个串**，而不是在比一个被本层背书过的「同一个实体」关系。
/// # 派生：**没有 `PartialEq` / `Eq`**（这一条是判据层面的，不是遗漏）
///
/// `BoundTo` 装的是 [`EntityId`]，而 `EntityId` 没有 `PartialEq`
/// （`src/candidate.rs` 的理由：设计没给「两个实体是同一个」这条判据，
/// 本层因此也不给它）。若这里派生 `PartialEq`，`BoundTo` 就会**隐式地**
/// 把「两个实体是同一个」定义成「两个串相等」——那正是 Task 3 明确拒绝过的那一步
/// （它的文档把这称为「凭空给出一枚没有用例的表达力」）。
/// 于是要判「一组约束是不是那一组」，只能**逐枚拆开看**（`tests/explicit.rs`
/// 的 `describe` 就是这么做的）——调用点因此看得见自己在比什么。
#[derive(Debug, Clone)]
pub enum ExplicitConstraint {
    /// 候选必须落在该作用域内。出处：**§274 的九值**（只取九值，那句决胜规则不归这一枚）。
    ///
    /// **判据是等值**（`f.scope == *s`），**不是层级**——二选一里选定的那一侧，
    /// 理由与「为什么不是层级」写在 [`satisfies`] 的文档里（那里是用判据的地方）。
    Scope(ResolutionScope),
    /// 候选的实体种类必须是这一种。出处：[`EntityKind`]（见它的文档）。
    EntityKind(EntityKind),
    /// mention 必须绑到该实体。出处：§277 的 `ExplicitBinding`。
    BoundTo(EntityId),
}

/// 候选的**判定事实**，由调用方与候选一并给出。**本层查不到它们**——注册表不在本层
/// （设计 §1.2.3 的规则 3），故 `scope` 与 `kind` 只能随值进来（见本文件头）。
///
/// # 为什么另起一枚类型，而不是往 `Candidate` 里加两个字段
///
/// §272（`docs/spec/05-normative.md:1378-1392`）给的 `Candidate` 是**照录的三字段**
/// （`entity_id` / `evidence[]` / `retrieval_score?`）。往它里面加字段会**改掉一张照录的形状**
/// ——于是「本层有没有悄悄扩过 §272」这件事再也没有对照物。
/// 故判定事实走这个**并列**的类型：`Candidate` 保 §272 的形，事实在这一枚上，
/// 两者由调用方配对送进 [`qualifying`]。
///
/// # 字段是 `pub` 的（与 [`crate::CanonicalRepresentation`] 相反）
///
/// 同 `Candidate` 的理由：**没有不变量可挂**。本类型是两个已封闭的值域的笛卡尔积
/// （`ResolutionScope` × `EntityKind`），每一个组合都是合法的判定事实——
/// 把它锁起来只会让每个调用方多写一个构造函数而不变量一条也不会多。
///
/// 派生：`Debug` / `Clone`（随值传递）与 `PartialEq` / `Eq`
/// （用例要断言「同一枚候选配两套不同的事实时结果不同」，那需要两套事实可分辨）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateFacts {
    /// 该候选在注册表里的作用域（§274 的九值）。
    pub scope: ResolutionScope,
    /// 该候选的实体种类。
    pub kind: EntityKind,
}

/// §4.3 的候选集（以**值**进入本层）：候选与它的**判定事实**配对。
///
/// # 这个名字的定义从哪来
///
/// 设计 §3.1 的组件表（`design:329`）只给了这个名字（`retrieve(&Mention, &CandidateSources)
/// -> CandidateSet`），**没有给它的定义**。它的形状取自 §4.1.1 逐字给出的 `qualifying` 签名
/// （`design:516`：`&[(Candidate, CandidateFacts)]`）——那张签名说明「候选集」在本层
/// 指的正是「候选 ＋ 事实」的序列，而不是光秃秃的一串 `Candidate`。
/// **本类型是别名、不是新类型**：一个 newtype 会在每个边界处要求一次包装／解包，
/// 而不变量一条也不会多（这个序列没有不变量可挂）。
pub type CandidateSet = Vec<(Candidate, CandidateFacts)>;

/// 判据：候选 `c` 在事实 `f` 下是否满足约束 `k`。
///
/// # 判据是**合取**（在 [`qualifying`] 那一层），单枚判据是这三行
///
/// 返 `bool` 而不是 `Option<Err>`：本层没有「判不了」这一态——三枚约束各自都有
/// 一个确定的两值判据（等值），故没有第三值。
///
/// # `Scope` 这一枚为什么是等值、不是层级（二选一里选定的那一侧）
///
/// - **选定**：`f.scope == *s`。§274 的层级**不由这一枚承担**。
/// - **为什么不做层级**：`==` 是等值，而 §274 的九个值是一条**优先级层级**；
///   要做层级就必须先有**「哪一枚比哪一枚更广」的序**，而**规范没有给这个序**
///   （§274 只给了九个名字与一句「显式 scope 优先级最高」，`docs/spec/05-normative.md:1408-1426`）。
///   **本层不发明这个序**（同上，故 `ResolutionScope` 也不派生 `Ord`）。
/// - **等值这一侧落在 fail-closed**：等值比「凭空发明的序」**更严**——合格候选**更少**，
///   结果**更容易落到 `Unknown` / `Ambiguous`**，而两者都不会自动绑定（§4.1）。
///   发明的序若偏宽，会把本该 `Ambiguous` 的情形判成 `Resolved`，那是 §273
///   （`docs/spec/05-normative.md:1394`，Open-World Resolution）的正对面。
/// - **那句「优先级最高」为什么不归这一枚**：它是一条**决胜规则**
///   （多个 binding 落在不同 scope 时取哪一个），**不是准入规则**。
///   把它当准入规则，会把「`project` 层可见的候选」在 `explicit` 约束下判成不合格
///   ——**而规范没这么说**。故出处收窄为「§274 的九值」。
///   `tests/explicit.rs::scope_is_equality_not_hierarchy` 逐字钉这条反面。
///
/// **缺的是「九个 scope 之间的包含关系」这一步。收件人：规范维护者。**
///
/// # 这里不是 X：为什么 `BoundTo` 用 `as_str` 而 `Scope` 用 `==`
///
/// 不是随手两种写法：`ResolutionScope` 的等值是**判据本身**（同一枚枚举值），
/// 而 `EntityId` 之间**本层没有「同一个实体」这条判据**（见 `src/candidate.rs`），
/// 故那里只能比调用方看得见的那个串。两处的写法不同，是两处的判据不同。
pub fn satisfies(c: &Candidate, f: &CandidateFacts, k: &ExplicitConstraint) -> bool {
    match k {
        ExplicitConstraint::Scope(s) => f.scope == *s,
        ExplicitConstraint::EntityKind(t) => f.kind == *t,
        ExplicitConstraint::BoundTo(e) => c.entity_id.as_str() == e.as_str(),
    }
}

/// 「合格」＝候选集里**满足全部约束**的那些。
///
/// # 合取是本设计的决定（§201 只给了流程的名字）
///
/// 把每一条约束都当作**必要条件**是唯一不引入优先级的读法；若将来要加权或分级，
/// 那是**新增判据**，须有规范出处（设计 §4.1.1，`design:517-521`）。
/// **这一条正好是 §4.2 断言 B 与 D 的定义基础**（「全部违反」＝合取下的空集，
/// 「唯一合格」＝合取下恰一个），故 `tests/explicit.rs::the_constraint_set_is_a_conjunction`
/// 钉的就是「一枚满足、一枚不满足 ⟹ 不合格」这一格。
///
/// # 空约束集下每一条候选都合格
///
/// 合取在空集上的中性元是「真」——**这是定义的直接后果，不是一条新判据**：
/// 「满足全部约束」在一条约束都没有时平凡成立。**反面（`k` 为空时谁也不合格）
/// 是另一个读法，本层取中立那侧**：拒绝一条约束都没有的调用方，会让「不设约束」
/// 这个完全合法的输入变成一条本层编出来的错误。
///
/// # 返回值是 `Vec<Candidate>`（不带事实）
///
/// 判定的**输出**是「哪些候选合格」，事实是判定的**输入**：把它们混在输出里，
/// 会让调用方拿到一份自己已经给过的数据，而 `surviving` 那几个字段（§4.1）
/// 的形状也是「候选」而不是「候选＋事实」。
///
/// **顺序照输入次序**，**不去重**：同 `CandidateSources::retrieve_all` 的口径——
/// 候选集里两枚同名实体是两份事实，本层没有判据决定它们的票怎么合
/// （见 `src/candidate.rs` 的 `retrieve_all` 注释第 2 条）。
pub fn qualifying(cs: &[(Candidate, CandidateFacts)], k: &[ExplicitConstraint]) -> Vec<Candidate> {
    cs.iter()
        .filter(|(c, f)| k.iter().all(|k| satisfies(c, f, k)))
        .map(|(c, _)| c.clone())
        .collect()
}
