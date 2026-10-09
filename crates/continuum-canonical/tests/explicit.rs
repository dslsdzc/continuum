//! Task 4 的约束面用例（§4.1.1，设计自定）与 §278↔§274 的映射。
//!
//! 每条用例的头注释写明**它钉的是什么**与**红条件是什么**；红条件在本 task 的变异轮里
//! 逐条真的取过（真写变异体、真跑到红），取到的结果记在
//! `.superpowers/sdd/p4-task-4-report.md`。
//!
//! # 这些用例钉的是判据本身，不是某一次解析的结果
//!
//! `satisfies` / `qualifying` 是**纯函数**（输入全是值，不查表、不做 I/O），
//! 故这里的断言与「真实候选来源零个」（设计 §16 第 5 条）无关——
//! 与 `tests/canonical.rs` 那批「夹具直接给四态」的用例不同，本文件不需要任何
//! 「谁来解析」的前提：判据就是判据。
//!
//! # 一处**不**在本文件的用例
//!
//! 「`CandidateFacts` 的字段清单逐项」由**签名**给出（`qualifying` 的入参形状）＋
//! 下面 `candidate_facts_are_given_by_the_caller` 的**行为**那一半；这里**没有**再写
//! 一条对 `CandidateFacts` 的穷尽解构——设计 §14 的表（`design:2200`）对本节的票据要求是
//! 「`CandidateFacts` 的 `scope` / `kind` 各一条」，那是**行为**要求（两枚事实各能改变判定），
//! 不是字段清单要求（它与 `Candidate` 不同：`Candidate` 那张清单是 §272 照录的，故要点名）。

use continuum_canonical::{
    AliasScope, Candidate, CandidateFacts, CandidateSet, EntityId, EntityKind, ExplicitConstraint,
    ResolutionScope, alias_scope_to_resolution_scope, qualifying, resolution_scope_to_alias_scope,
    satisfies,
};

/// 夹具：一枚候选。`evidence` / `retrieval_score` 与本节要钉的判据无关，取最省的形状。
fn cand(id: &str) -> Candidate {
    Candidate {
        entity_id: EntityId::from_str(id),
        evidence: Vec::new(),
        retrieval_score: None,
    }
}

/// 夹具：一枚候选的判定事实。
fn facts(scope: ResolutionScope, kind: EntityKind) -> CandidateFacts {
    CandidateFacts { scope, kind }
}

/// `ExplicitConstraint` 三枚**各一条满足 ＋ 各一条不满足**（共 6 条断言）。
///
/// # 为什么是三枚而不是「一枚代表」
///
/// 三枚的判据**来源不同**：`Scope` 是作用域等值、`EntityKind` 是种类等值、
/// `BoundTo` 是实体串相等（`EntityId` 没有 `PartialEq`，故走 `as_str`）。
/// 一枚写错，另外两枚照旧绿——**抽代表会让两枚没有照片**
/// （本仓「枚举断言须逐项有照片」的同一条）。
///
/// # 每一枚的**反面**都要有，且都要**只差那一维**
///
/// 每条反面只改被判的那一维（`Scope` 改 scope、`EntityKind` 改 kind、
/// `BoundTo` 改实体串），其余照旧。这样一条反面红，能唯一地指向那一枚的判据。
///
/// 红条件（**移除档**）：把 `satisfies` 里某一枚写成恒真（例如 `EntityKind` 那一臂
/// 直接 `true`）⟹ **那一枚的反面红**，另两枚（正反两面）不受影响。
/// 实测见报告 §变异轮。
#[test]
fn the_three_explicit_constraints_each_have_a_positive_and_a_negative_case() {
    let c = cand("entity.one");

    // ---- Scope：正面（同值）／反面（异值）----
    let f_project = facts(ResolutionScope::Project, EntityKind::Tool);
    assert!(
        satisfies(&c, &f_project, &ExplicitConstraint::Scope(ResolutionScope::Project)),
        "Scope 正面：候选落在 `project`，约束也取 `project` ⟹ 满足",
    );
    assert!(
        !satisfies(&c, &f_project, &ExplicitConstraint::Scope(ResolutionScope::Global)),
        "Scope 反面：候选是 `project`、约束是 `global` ⟹ 不满足（等值判据，不是包含）",
    );

    // ---- EntityKind：正面（同值）／反面（异值）----
    let f_tool = facts(ResolutionScope::Project, EntityKind::Tool);
    assert!(
        satisfies(&c, &f_tool, &ExplicitConstraint::EntityKind(EntityKind::Tool)),
        "EntityKind 正面：候选是 `Tool`，约束也取 `Tool` ⟹ 满足",
    );
    assert!(
        !satisfies(&c, &f_tool, &ExplicitConstraint::EntityKind(EntityKind::Skill)),
        "EntityKind 反面：候选是 `Tool`、约束是 `Skill` ⟹ 不满足",
    );

    // ---- BoundTo：正面（同一串）／反面（不同串）----
    let f_any = facts(ResolutionScope::Session, EntityKind::Relation);
    assert!(
        satisfies(&c, &f_any, &ExplicitConstraint::BoundTo(EntityId::from_str("entity.one"))),
        "BoundTo 正面：候选的实体串与约束给的那一枚相同 ⟹ 满足",
    );
    assert!(
        !satisfies(&c, &f_any, &ExplicitConstraint::BoundTo(EntityId::from_str("entity.two"))),
        "BoundTo 反面：两个串不同 ⟹ 不满足",
    );
}

/// 判据是**合取**：`K` 有两枚、候选只满足其一 ⟹ **不合格**。
///
/// 钉的是设计 §4.1.1 写死的那条（`design:517-521`）：「把每一条约束都当作**必要条件**
/// 是唯一不引入优先级的读法」。**这一条是 §4.2 断言 B 与 D 的定义基础**
/// ——「全部违反」＝合取下的空集，「唯一合格」＝合取下恰一个。
///
/// # 两个方向各一条，不抽代表
///
/// 「只满足其一」有两个方向（满足第一枚不满足第二枚／满足第二枚不满足第一枚）。
/// **只写一条的话，把判据写成「只判第一枚」也照样绿**——那不是合取，
/// 而是一条恰好也对得上的更弱判据。两条都写，才把「每一枚都是必要条件」钉住。
///
/// 顺带把「怎么才算合格」的正面对照也放进来（两枚都满足 ⟹ 合格），
/// 否则「恒不合格」的实现能满足上面两条反面。
///
/// 红条件（**取反档**）：把 `qualifying` 的 `all` 改成 `any`（析取）
/// ⟹ 上面两条反面断言**都红**（两枚各只满足其一，析取下属合格）。
/// 实测见报告 §变异轮。
#[test]
fn the_constraint_set_is_a_conjunction() {
    let k = [
        ExplicitConstraint::Scope(ResolutionScope::Project),
        ExplicitConstraint::EntityKind(EntityKind::Tool),
    ];

    // 方向一：满足 `Scope`、不满足 `EntityKind`。
    let only_scope = vec![(cand("e-1"), facts(ResolutionScope::Project, EntityKind::Skill))];
    assert!(
        qualifying(&only_scope, &k).is_empty(),
        "只满足 `Scope` 一枚 ⟹ 不合格（合取：两枚都是必要条件）",
    );

    // 方向二：满足 `EntityKind`、不满足 `Scope`。
    let only_kind = vec![(cand("e-2"), facts(ResolutionScope::Global, EntityKind::Tool))];
    assert!(
        qualifying(&only_kind, &k).is_empty(),
        "只满足 `EntityKind` 一枚 ⟹ 不合格（两个方向都断，免得「只判第一枚」也绿）",
    );

    // 正面对照：两枚都满足 ⟹ 合格（否则「恒不合格」也能过上面两条）。
    let both = vec![(cand("e-3"), facts(ResolutionScope::Project, EntityKind::Tool))];
    let got = qualifying(&both, &k);
    assert_eq!(got.len(), 1, "两枚都满足 ⟹ 恰有一枚合格");
    assert_eq!(got[0].entity_id.as_str(), "e-3", "合格的那一枚就是它");
}

/// `Scope` 做的是**等值过滤，不是层级**（设计点名的反面）。
///
/// # 钉的是哪一格
///
/// 一个在 `project` 层可见的候选，在 `Scope(explicit)` 下**不合格**。
/// 若有人把 `==` 顺手写成「更广的 scope 包含更窄的」（`explicit` 在这条优先级层级里
/// 排在最前，看起来「最广」），这一格就会判成**合格**——那时本用例红。
///
/// # 为什么不该有层级（判据的三条，照录设计 §4.1.1）
///
/// - §274 的九个值是一条**优先级层级**，而**「哪一枚比哪一枚更广」的序规范没有给**
///   （§274 只给了九个名字与一句「显式 scope 优先级最高」，`docs/spec/05-normative.md:1408-1426`）。
///   **本层不发明那个序**——这也是 `ResolutionScope` 不派生 `Ord` 的同一条理由。
/// - **那句「优先级最高」是一条决胜规则**（多个 binding 落在不同 scope 时取哪一个），
///   **不是准入规则**（哪些候选合格）。把它当准入规则，会把「`project` 层可见的候选」
///   在 `explicit` 约束下判成不合格——**而规范没这么说**。
/// - **等值比凭空发明的序更严**（fail-closed）：合格候选更少 ⟹ 结果更容易落到
///   `Unknown` / `Ambiguous`，而两者都不会自动绑定；发明的序若偏宽，会把本该
///   `Ambiguous` 的情形判成 `Resolved`（§273 的正对面）。
///
/// **两侧都断**（只断反面的话，「恒不满足」的实现照样绿）：
/// 反面这一格不满足，而同一枚候选在 `Scope(project)` 下满足。
///
/// 红条件（**移除档**）：把 `f.scope == *s` 换成「更广的 scope 包含更窄的」
/// ⟹ 本条第一条断言红。实测见报告 §变异轮。
#[test]
fn scope_is_equality_not_hierarchy() {
    let c = cand("e-project");
    let f = facts(ResolutionScope::Project, EntityKind::Tool);

    assert!(
        !satisfies(&c, &f, &ExplicitConstraint::Scope(ResolutionScope::Explicit)),
        "`project` 层的候选在 `Scope(explicit)` 下不合格——等值，不是层级",
    );
    // 另一侧：同一个候选在它自己那一层下合格。没有这一条，「恒不满足」也能过上面那句。
    assert!(
        satisfies(&c, &f, &ExplicitConstraint::Scope(ResolutionScope::Project)),
        "同一枚候选在 `Scope(project)` 下合格（免得上面那句被一个恒假的判据满足）",
    );

    // 再加一格：**合取下的后果**（层级若被顺手实现进去，这一格会漏出去）。
    let cs: CandidateSet = vec![(c.clone(), f)];
    assert!(
        qualifying(&cs, &[ExplicitConstraint::Scope(ResolutionScope::Explicit)]).is_empty(),
        "合格集为空——层级没有从 `satisfies` 漏进 `qualifying`",
    );
}

/// `EntityKind` **恰好四枚**：`Tool` / `Skill` / `Relation` / `ConnectorService`。
///
/// # 机制分两半，证明力不同
///
/// - **编译期那一半**（真正的「恰好四枚」）：下面 `kind_name` 的 `match`
///   **穷尽、不带 `..`**——加第五枚即**编译不过**。按本仓纪律
///   **「编译不过」不算变红**，故这一半是**类型保证**，运行期取不到它的红。
/// - **运行期那一半**：四枚**各一条**断言，钉的是「这四枚各自可分辨」
///   （名字两两不同、且各能通过自己那一条 `EntityKind` 约束）。
///   **运行期钉不住「没有第五枚」**——那件事在运行期不可见，故不假装钉住。
///
/// 红条件（**取反档**）：把 `Tool` 与 `Skill` 两枚的 `kind_name` 写成同一个串
/// ⟹ 「四个名字两两不同」那条断言红。实测见报告 §变异轮。
#[test]
fn entity_kind_has_exactly_four_variants() {
    /// 穷尽、**不带 `..`**：加第五枚即编译不过（编译期照片）。
    fn kind_name(k: &EntityKind) -> &'static str {
        match k {
            EntityKind::Tool => "tool",
            EntityKind::Skill => "skill",
            EntityKind::Relation => "relation",
            EntityKind::ConnectorService => "connector_service",
        }
    }

    let all = [
        EntityKind::Tool,
        EntityKind::Skill,
        EntityKind::Relation,
        EntityKind::ConnectorService,
    ];
    let names: Vec<&str> = all.iter().map(kind_name).collect();
    assert_eq!(names.len(), 4, "四枚各一条，不抽代表");

    // 四枚**两两不同**：把两枚的名字写成同一个串，这一条红。
    for (i, a) in names.iter().enumerate() {
        for b in &names[i + 1..] {
            assert_ne!(a, b, "四枚的名字必须两两不同：{names:?}");
        }
    }

    // 四枚**各能通过自己那一条约束**（否则名字对得上、判据却错的实现也能绿）。
    let c = cand("e-kind");
    for k in &all {
        let f = facts(ResolutionScope::Project, k.clone());
        assert!(
            satisfies(&c, &f, &ExplicitConstraint::EntityKind(k.clone())),
            "`{k:?}` 这一枚要能通过它自己那条约束",
        );
    }
}

/// 正面照片：`GitHub` 这类服务名解出的候选，其 `CandidateFacts.kind` **恰是
/// `ConnectorService`**。
///
/// # 夹具为什么是手写的（照实写，免得被读成漏了真实路径）
///
/// 本层今天**没有任何真实候选来源**（设计 §16 第 5 条，`design:2331-2334`），
/// 也没有把服务名切出来的字面表（§16 第 5b 条：本层只有代词与上下文引用两张表）
/// ——故这一份事实是**手工夹具**给的。这条用例钉的是**判据那一侧**：
/// 「一枚被声明为 `ConnectorService` 的候选，在 `EntityKind(ConnectorService)` 约束下合格」
/// ＋「在 `EntityKind(Tool)` 约束下不合格」。**它不钉**「某个检索来源会把 `GitHub`
/// 认成服务」——那件事今天在本层不存在（没有来源、没有表），也不该由本层假装。
///
/// 红条件（**移除档**）：把 `EntityKind` 的 `ConnectorService` 那一枚删掉、
/// 让服务名硬塞进 `Tool` ⟹ 本用例与下一条一起红（更直接地：本文件编不过，
/// 因为 `ConnectorService` 这个构造子没有了——按本仓纪律，**编译不过不算红**，
/// 故「删掉第四枚」这件事的性质是**编译期照片**；本用例的运行期红条件落在
/// `satisfies` 的 `EntityKind` 那一臂被写成恒真／恒假上）。
#[test]
fn a_connector_service_mention_resolves_to_the_connector_service_kind() {
    let c = cand("connector.github");
    let f = facts(ResolutionScope::Global, EntityKind::ConnectorService);

    assert_eq!(
        f.kind,
        EntityKind::ConnectorService,
        "夹具前提与正面断言：这一枚候选的判定事实**恰是** `ConnectorService`",
    );
    assert!(
        satisfies(&c, &f, &ExplicitConstraint::EntityKind(EntityKind::ConnectorService)),
        "服务名的候选在 `EntityKind(ConnectorService)` 约束下合格",
    );
}

/// 反面**逐枚**：同一条服务名候选**不是** `Tool`、**不是** `Skill`、**不是** `Relation`。
///
/// # 逐枚断言，不写「不等于某个别的值」
///
/// 「不等于 `Tool`」与「不是三枚中的任何一枚」是两句话；前者可以靠一个恰好也
/// 不等于 `Tool` 的错值满足。故这里**逐枚**写出四条断言的头三条
/// （用例名只点了两条，第三条 `Relation` 一并断——设计 §4.1.1 的理由段
/// （`design:549-556`）说的是「`GitHub` / `Email` **不是 `Tool` / `Skill` / `Relation`
/// 中任何一个**」，三枚都在那句话里，只断两枚会让第三枚没有照片）。
///
/// # 依据（为什么 `EntityKind` 是四枚、不是 §198 的三枚）
///
/// 设计 §16 第 26 条（`design:2431-2444`）本层**认领了「服务名作为 mention 的解析目标」**
/// 并写明「与其它实体**同形**处理」。而 `GitHub` / `Email` 这些服务名**不是
/// `Tool` / `Skill` / `Relation` 中任何一个**——若 `EntityKind` 只有三枚，
/// 调用方只能**硬塞**一枚进去，于是**静默错型，且没有任何断言会红**（本仓最忌的形状）。
/// 第四枚的出处是 §124（`docs/spec/02-positioning.md:1803-1826`）的六项服务与 §125 的「按操作细分」。
///
/// 红条件（**移除档**）：「硬塞」的形态——让服务名候选的事实取 `kind: Tool`。
/// 那时本条第一条断言红（`Tool` 的服务名候选会被 `EntityKind(ConnectorService)` 拒掉，
/// 而正面那条用例也会红）。实测见报告 §变异轮。
#[test]
fn a_connector_service_is_neither_a_tool_nor_a_skill() {
    let c = cand("connector.github");
    let f = facts(ResolutionScope::Global, EntityKind::ConnectorService);

    assert_ne!(f.kind, EntityKind::Tool, "服务名**不是** `Tool`（逐枚）");
    assert_ne!(f.kind, EntityKind::Skill, "服务名**不是** `Skill`（逐枚）");
    assert_ne!(
        f.kind,
        EntityKind::Relation,
        "服务名**不是** `Relation`（逐枚；用例名只点了两枚，第三枚的出处见文档注释）",
    );

    // 反向再钉一次（走判据而不是走 `!=`）：这三条约束都不接受它。
    for k in [EntityKind::Tool, EntityKind::Skill, EntityKind::Relation] {
        assert!(
            !satisfies(&c, &f, &ExplicitConstraint::EntityKind(k.clone())),
            "`{k:?}` 这一枚不该接受服务名候选",
        );
    }
}

/// `alias_scope_to_resolution_scope` 的**定义域恰是 §278 的四枚**，
/// `resolution_scope_to_alias_scope` 在**没有 alias 对应的五枚**上各给 `None`。
///
/// # 四枚各一条、五枚逐枚（不抽代表）
///
/// 正向：`TURN`→`conversation`、`SESSION`→`session`、`PROJECT`→`project`、
/// `USER_PERSISTENT`→`user`——四枚**各一条**断言。
/// 反向：`workspace` / `tool` / `plugin` / `global` / `explicit` **五枚逐枚**断言 `None`。
///
/// # 定义域封闭那一半是编译期的
///
/// 下面 `exhaustive` 那个 `match` **穷尽、不带 `..`** ⇒ 加第五枚即编译不过；
/// 又因为本函数的入参是 `&AliasScope`（不是 `&str`），
/// **「没有第五枚可以喂进去」在类型上成立**——这正是「取闭集、不取串」的理由：
/// 开放取值域下「哪个串换不出来」是一条由调用方定义的性质，本层无从断言。
/// 按本仓纪律，编译不过**不算红**，故这一半是**类型保证**。
///
/// # 这张表是本设计的决定（规范没给九值与四值的关系）
///
/// §274（`docs/spec/05-normative.md:1408-1426`）与 §278（同文件 `:1485-1499`）
/// **各自成文、互不引用**——九值与四值的关系**规范没有给**。
/// 设计 §4.1.1 补的那张对照表（`design:574-604`）就是本函数的实现，
/// 其方向（哪一对是「放宽」）与失效条件写在那里，这里不重抄。
///
/// **用途**：一条以 alias scope 表述的约束，**必须先经上表换成 §274 的值**
/// 才能进 `ExplicitConstraint::Scope`；换不出来的五枚**不能用 alias 表述**。
///
/// 红条件（**取反档**）：把 `Session` 与 `Project` 两条映射对调 ⟹ 对应的两条断言红。
/// 实测见报告 §变异轮。
#[test]
fn the_alias_scope_mapping_has_exactly_the_four_alias_scopes_as_its_domain() {
    /// 穷尽、**不带 `..`**：加第五枚即编译不过（定义域封闭的编译期照片）。
    fn exhaustive(s: &AliasScope) -> ResolutionScope {
        alias_scope_to_resolution_scope(s)
    }

    // 四枚各一条。
    assert_eq!(
        exhaustive(&AliasScope::Turn),
        ResolutionScope::Conversation,
        "`TURN` → `conversation`（最弱的一对：§274 没有「轮次」这一值，取语义最近的）",
    );
    assert_eq!(
        exhaustive(&AliasScope::Session),
        ResolutionScope::Session,
        "`SESSION` → `session`（同名）",
    );
    assert_eq!(
        exhaustive(&AliasScope::Project),
        ResolutionScope::Project,
        "`PROJECT` → `project`（同名）",
    );
    assert_eq!(
        exhaustive(&AliasScope::UserPersistent),
        ResolutionScope::User,
        "`USER_PERSISTENT` → `user`（§278 的「持久」对应 §274 的 user 层）",
    );

    // 反向：四枚有对应。
    for (alias, scope) in [
        (AliasScope::Turn, ResolutionScope::Conversation),
        (AliasScope::Session, ResolutionScope::Session),
        (AliasScope::Project, ResolutionScope::Project),
        (AliasScope::UserPersistent, ResolutionScope::User),
    ] {
        assert_eq!(
            resolution_scope_to_alias_scope(&scope),
            Some(alias),
            "`{scope:?}` 的 alias 对应是 `{alias:?}`",
        );
    }

    // 反向：**五枚逐枚**没有对应。
    assert_eq!(
        resolution_scope_to_alias_scope(&ResolutionScope::Workspace),
        None,
        "`workspace` 在 §278 里没有对应",
    );
    assert_eq!(
        resolution_scope_to_alias_scope(&ResolutionScope::Tool),
        None,
        "`tool` 在 §278 里没有对应",
    );
    assert_eq!(
        resolution_scope_to_alias_scope(&ResolutionScope::Plugin),
        None,
        "`plugin` 在 §278 里没有对应",
    );
    assert_eq!(
        resolution_scope_to_alias_scope(&ResolutionScope::Global),
        None,
        "`global` 在 §278 里没有对应",
    );
    assert_eq!(
        resolution_scope_to_alias_scope(&ResolutionScope::Explicit),
        None,
        "`explicit` 在 §278 里没有对应",
    );
}

/// `CandidateFacts` 由调用方**与候选一并给出**——**本层查不到它们**。
///
/// # 这一条的照片是**行为**，不是措辞
///
/// 同一枚 `Candidate`（同一个 `entity_id`、同一份 `evidence`、同一个分数）
/// 配**两套不同的事实**，`qualifying` 的结果必须不同：
/// 事实若来自本层自己（查注册表／猜），这个差别就取不到。
/// 同理，「换个事实不改结果」的实现在这里红。
///
/// # 签名那一半（编译期）
///
/// `qualifying` 的入参是 `&[(Candidate, CandidateFacts)]` 与 `&[ExplicitConstraint]`
/// ——**不收 `&Tx`、不收 `dyn CandidateSource`**（对比 §4.3 的
/// `CandidateSources::retrieve_all`：那是**装配方**交给解析器的一步，不是本模块的输入）。
/// 这一半是类型层面的：本模块里没有任何一处拿得到连接或来源，
/// 而按本仓纪律**编译不过不算红**，故它**没有运行期照片**——
/// 本条能拍到的是上面那一半（事实是输入、且它改变结果）。
///
/// 红条件（**移除档**）：把 `qualifying` 改成只看候选、忽略 `f`
/// （即 `k` 恒真）⟹ 本条红。实测见报告 §变异轮。
#[test]
fn candidate_facts_are_given_by_the_caller() {
    // **同一枚候选**，两套事实。
    let c = cand("e-facts");
    let k = [ExplicitConstraint::EntityKind(EntityKind::Tool)];

    let as_tool: CandidateSet = vec![(c.clone(), facts(ResolutionScope::Project, EntityKind::Tool))];
    let as_skill: CandidateSet =
        vec![(c.clone(), facts(ResolutionScope::Project, EntityKind::Skill))];

    assert_eq!(
        qualifying(&as_tool, &k).len(),
        1,
        "给 `Tool` 的事实时，它在 `EntityKind(Tool)` 约束下合格",
    );
    assert_eq!(
        qualifying(&as_skill, &k).len(),
        0,
        "同一枚候选、换成 `Skill` 的事实 ⟹ 不合格——事实来自调用方，本层查不到",
    );
    assert_eq!(
        as_tool[0].0.entity_id.as_str(),
        as_skill[0].0.entity_id.as_str(),
        "两侧的候选本体是同一枚（差别只在事实那一维）",
    );
}

/// `Candidate` 仍是 §272 的三字段——本 task **不往它里面加** `scope` / `kind`。
///
/// # 机制 = 不带 `..` 的穷尽解构（编译期照片）
///
/// 往 `Candidate` 里加第四个字段 ⟹ `E0027`（pattern 未覆盖新字段）；
/// 反过来，这里的 pattern 里写一个不存在的字段 ⟹ `E0026`。
/// 两枚字段 `scope` / `kind` 是**判定事实**，它们住在并列的 [`CandidateFacts`] 上。
///
/// **明写**：这是**编译期**照片，运行期无照片（`Candidate` 是纯数据声明）。
/// 加字段这件事**没有变红的观测**，只有一条编译错误——按本仓纪律「编译不过」不算红。
/// Task 3 的 `tests/candidate.rs::the_candidate_records_exactly_the_three_fields_of_272`
/// 已经拍过同一张照片；**本条不是重复**：Task 3 那条钉的是「§272 的三字段」，
/// 本条钉的是**本 task 没有动它**（同一个 pattern、不同的命题），
/// 故这里只解构一次、不再断言每枚的取值。
///
/// 红条件（**移除档**）：把 `CandidateFacts` 的两枚塞进 `Candidate`
/// （加 `pub scope: ResolutionScope` / `pub kind: EntityKind`）⟹ 本文件
/// **编译不过**（本用例的 pattern 与下面这个字面量都少字段）。
///
/// # 为什么走 `CandidateFacts` 而不动 `Candidate`
///
/// 往那里加字段会**改掉一张照录的形状**（§272，`docs/spec/05-normative.md:1378-1392`）
/// ——于是「本层有没有悄悄扩过 §272」这件事再也没有对照物。
#[test]
fn the_272_candidate_shape_is_untouched() {
    // 字面量恰好三个字段（少一个 ⟹ E0063，多一个不存在的 ⟹ E0560）。
    let c = Candidate {
        entity_id: EntityId::from_str("e-272"),
        evidence: vec!["alias registry".to_owned()],
        retrieval_score: Some(0.5),
    };

    // **不带 `..`**：这一行就是本用例的全部机制。
    let Candidate {
        entity_id,
        evidence,
        retrieval_score,
    } = c;

    assert_eq!(entity_id.as_str(), "e-272");
    assert_eq!(evidence, vec!["alias registry".to_owned()]);
    assert_eq!(retrieval_score, Some(0.5));

    // 判定事实**在另一枚类型上**（这就是「不往 `Candidate` 里加」的落点）。
    let f = CandidateFacts {
        scope: ResolutionScope::Project,
        kind: EntityKind::Tool,
    };
    assert_eq!(f.kind, EntityKind::Tool, "事实住在 `CandidateFacts` 上");
}
