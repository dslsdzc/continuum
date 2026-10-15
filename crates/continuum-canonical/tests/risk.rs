//! Task 6：§276 的风险门槛（[`ResolutionRisk`] / [`RequiredStrength`]）、
//! §277 的 `ExplicitBinding` 最高优先序，与 §4.6 的**集合外注入**（[`binding::apply`]）。
//!
//! # 注入形态：**调用方传参**（本 task 的开工前置判据，先定这一条再动代码）
//!
//! 设计 §4.6 逐字给出的是
//!
//! ```text
//! let working = retrieved.clone();
//! if let Some(b) = binding { working.push(injected_candidate(b)); }   // 唯一允许的注入点
//! ```
//!
//! （`docs/superpowers/specs/2026-10-06-p4-semantic-layer-design.md:634-638`，行号本 task 实测）
//! ——`binding` 是**调用方带来的入参**，**不是**由 `BoundTo` 约束自动发生的东西：
//! 约束是判据（「候选必不必须绑到某实体」），`ExplicitBinding` 是**事实**（「用户点名了某一枚」），
//! 两者的位置相反（约束进 `constraints`，binding 进 `injection`）。故本 task 取**前一支**。
//!
//! **它对 Task 5 那条守卫的后果**（`tests/resolver.rs::assertion_a_injection_guard`）：
//! 那条用例的调用点传 `None`，而「不带 binding 就没有注入」正是本层今天的正确行为
//! ⇒ **守卫有效，不重写**。它钉的第二个来源（`injected`）今天仍是 `None`，
//! 而 `binding::apply` 落地之后那条判据依然成立：**注入只从 `ExplicitBinding` 来**
//! （另一条路——由 `BoundTo` 自动注入——在本层不存在，本文件末尾的照片 (b) 钉的就是这件事的相反面）。

use continuum_canonical::{
    BindingSource, Candidate, CandidateFacts, EntityId, EntityKind, ExplicitBinding,
    ExplicitConstraint, Mention, MentionKind, Opaque, RequiredStrength, ResolutionConfidence,
    ResolutionResult, ResolutionRisk, ResolutionScope, UnknownReason, binding, resolve, satisfies,
};

/// 夹具：一枚候选。
fn cand(id: &str, score: Option<f64>) -> Candidate {
    Candidate {
        entity_id: EntityId::from_str(id),
        evidence: Vec::new(),
        retrieval_score: score,
    }
}

/// 夹具：一枚候选的判定事实。
fn facts(scope: ResolutionScope, kind: EntityKind) -> CandidateFacts {
    CandidateFacts { scope, kind }
}

/// 夹具：一枚「已达」的置信值（§275；本层不解释它的取值域，见 `src/resolver.rs`）。
fn conf(level: &str) -> ResolutionConfidence {
    ResolutionConfidence::new(Opaque::from_str(level))
}

/// 夹具：一枚「要求」的强度（§276 的 `required_resolution_strength`）。
fn req(level: &str) -> RequiredStrength {
    RequiredStrength::new(Opaque::from_str(level))
}

/// 夹具：一枚 mention（§270 的 [`Mention`] 三字段都是 `pub`，故这里直接拼）。
fn mention(text: &str) -> Mention {
    Mention {
        span: 0..text.len(),
        text: text.to_owned(),
        kind: MentionKind::from_str("noun_phrase"),
    }
}

/// 夹具：一条绑定（§277 的四字段照录）。
fn binding_to(entity: &str, scope: ResolutionScope) -> ExplicitBinding {
    ExplicitBinding {
        mention: mention(entity),
        entity: EntityId::from_str(entity),
        scope,
        source: BindingSource::User,
    }
}

/// §4.5：`required_resolution_strength` 是**必填入参，没有默认值**。
///
/// # 这一条的两半各自落在哪（**签名层面 ＋ 编译期**）
///
/// 1. **签名层面**：下面那个调用把 `&RequiredStrength` 直接传进去——
///    若 `resolve` 的该参数改成 `Option<&RequiredStrength>`，这一行**编译不过**
///    （`&RequiredStrength` 不是 `Option<…>`）。**类型注解即断言**。
/// 2. **编译期**：`RequiredStrength` 没有 `Default`、也没有 `from_risk` 一类的基线，
///    故反例住在 `tests/compile_fail/`（各配一份实跑取值的 `.stderr`）：
///    - `required_strength_has_no_default.rs` —— `RequiredStrength::default()` 编不过；
///    - `resolve_requires_the_strength_argument.rs` —— 少传该参数编不过；
///    - `required_strength_has_no_from_risk.rs` —— `RequiredStrength::from_risk(&risk)` 编不过。
///    三份样例的**驱动与正控制**都在 `tests/type_level.rs`（那是本 crate 唯一的 `trybuild` 入口）。
///
/// # 为什么**没有运行期照片**：不是「没有照片」，是「没有判据」
///
/// §276 的原文是 `required_resolution_strength = f(effect, cost, reversibility, privacy)`
/// （`docs/spec/05-normative.md:1450-1462`），而 §4.5 写明**`f` 的形态与四个风险维之间的
/// 序与量纲都没有给**（`design:763-773`）。于是「风险越高要求越严」这条**单调性**
/// 在本层**没有任何函数**能拍出来——要拍它就得先有一个四维上的序，而那个序没有规范出处
/// （本仓口径：无中生有的序会把「高成本」判成「低风险」，比没有更坏）。
/// **故这一条与「本层没有实现某个性质」不是一回事**：本层能做到的是把
/// `required_resolution_strength` 做成**不可省略的输入**（这一条有上面两半照片），
/// 而不是替调用方算它。设计 §15.2 第 2 条把这一类记为「不是没有照片，是没有判据」。
///
/// 红条件（**取反档**）：给 `RequiredStrength` 加 `Default`，或把 `resolve` 的该参数
/// 改成 `Option<&RequiredStrength>` ⟹ 第 1 份反例红（`default()` 变合法），
/// 且本用例下面那一行也编不过（`Option` 与 `&RequiredStrength` 不同型）。
/// 实测见报告 §变异轮。
#[test]
fn required_strength_is_a_required_argument_with_no_default() {
    let k = [ExplicitConstraint::Scope(ResolutionScope::Project)];
    let cs = vec![(
        cand("e-gate", None),
        facts(ResolutionScope::Project, EntityKind::Tool),
    )];

    // 类型注解即断言：收的是 `&RequiredStrength`（不是 `Option<…>`）；五个参数一个不少。
    let r: ResolutionResult = resolve(&cs, None, &k, &conf("high"), &req("high"));
    assert!(
        matches!(&r, ResolutionResult::Resolved(_)),
        "门槛是必填的第五个入参，传进来就参与判定（同档 ⟹ 放行）：{r:?}",
    );
}

/// §4.5：本层**不提供**任何把 [`ResolutionRisk`] 映射成 [`RequiredStrength`] 的基线。
///
/// # 公开面清单逐条（本用例的机制）
///
/// 设计 §4.5 写死「本层不提供任何把 `ResolutionRisk` 映射成门槛的基线实现」
/// （`design:767-771`），理由是无中生有的序会把「高成本」判成「低风险」——
/// **基线比没有更坏**。这一条要在**公开面**上逐条核，故本用例做两件事：
///
/// 1. **构造面**：`RequiredStrength` 的**唯一**公开构造入口是 `RequiredStrength::new(Opaque)`
///    （下面第一段用它，那是**正控制**：证明「名字对得上、类型确实可构造」，
///    免得下面第 2 条的扫描绿在一个与判据无关的原因上）；
/// 2. **反面清单**：扫 `src/` 下的源码，两枚模式各断言**零命中**——
///    `impl From<ResolutionRisk>` 与 `fn from_risk`。
///    它们是那条基线**只要有就必然写出**的两个形状（一个是 `From` 那条路、
///    一个是具名构造函数那条路）；trybuild 反例把同样两件事在**编译期**再钉一遍
///    （`tests/compile_fail/required_strength_has_no_from_risk.rs`，
///    它逐条试 `from_risk(...)` 与 `RequiredStrength::from(risk)`）。
///    **扫的是代码、不是注释**（`src/risk.rs` 的文档正是在用这两个模式说明「本模块没有它们」）
///    ——剥注释的机制与它的限度见 [`read_all_sources`]。
///
/// # 这一条守卫的限度（写出来，免得被读成覆盖得更广）
///
/// 第 2 条是**词法扫描**，不是类型证明：它钉的是「这两个形状的字符串不在 `src/` 里」，
/// **不钉**「任何一个别的形状的基线也不存在」（比如一个叫 `baseline_of` 的函数，
/// 或一条写在 `resolve` 内部的 `match risk { … }`）。**方法名是开放集，枚举不完**
/// （与 `CandidateSource` 的 `pick_one` 守卫同一限度，见 `src/candidate.rs`）。
/// 编译期那一半（trybuild）钉的也**只是那几个名字**。
/// **故本层能说的只是**：这两条最显然的路走不通；「本层没有发明任何序」那句更宽的话
/// **本层拿不出照片**，也就不写。
///
/// 红条件（**移除档**，本仓口径：加一条「保守的」基线正是要防的那一格）：
/// 在 `src/` 里写下 `impl From<ResolutionRisk> for RequiredStrength`（或一个
/// `fn from_risk`）⟹ 第 2 条的扫描红，且 `required_strength_has_no_from_risk.rs`
/// 那份反例的 `.stderr` 也变（那两行从 `E0599` / `E0308` 变成编得过）。
/// 实测见报告 §变异轮。
#[test]
fn the_layer_provides_no_baseline_mapping_from_risk_to_strength() {
    // —— 第 1 件：正控制。唯一的公开构造入口可用，且它收的是**不透明值**（不是风险事实）。 ——
    let risk = ResolutionRisk {
        effect: Opaque::from_str("effect"),
        cost: Opaque::from_str("cost"),
        reversibility: Opaque::from_str("reversibility"),
        privacy: Opaque::from_str("privacy"),
    };
    let strength = RequiredStrength::new(Opaque::from_str("calibrated"));
    let met: bool = conf("calibrated").meets(&strength);
    assert!(
        met,
        "正控制：构造入口可用、门槛判定可用（本层的公开面确实存在）"
    );
    // 风险事实**是入参、不是门槛的来源**：本层没有读它做判定的代码——唯一会那样做的是一条
    // 「风险 → 门槛」的基线，而那条路正是下面第 2 件钉的。字段是 `pub` 的，
    // 穷尽解构那张照片住在下面那条用例里。
    assert_eq!(risk.effect.as_str(), "effect");

    // —— 第 2 件：反面清单，逐条扫 ——
    let sources = read_all_sources();
    assert!(
        sources
            .iter()
            .any(|(_, s)| s.contains("RequiredStrength::new")),
        "正控制（对扫描机制本身）：`src/` 里找得到 `RequiredStrength::new`——\
         否则下面的零命中可以来自「扫描根本没读到文件」这类与判据无关的原因",
    );
    for (name, body) in &sources {
        assert!(
            !body.contains("From<ResolutionRisk>"),
            "`{name}` 里出现了 `From<ResolutionRisk>`：§4.5 写死本层**不提供**任何\
             把风险事实映射成门槛的基线（无中生有的序会把「高成本」判成「低风险」）",
        );
        assert!(
            !body.contains("fn from_risk"),
            "`{name}` 里出现了 `fn from_risk`：同上——那条基线本层不提供",
        );
    }
}

/// §276 的四个风险事实：字段名**逐字照录**，四个都是不透明值。
///
/// # 机制：**不带 `..` 的穷尽解构**
///
/// 解构的是 [`ResolutionRisk`] 自己的字段——`effect` / `cost` / `reversibility` / `privacy`，
/// 与 §276 原文的 `f(effect, cost, reversibility, privacy)` 逐字同形
/// （`docs/spec/05-normative.md:1450-1462`）。四枚字段都是 `pub`，故这份解构在 crate 外写得出。
///
/// 加字段、少字段、改名 ⟹ **编译不过**（加字段 `E0027`、写不存在的字段 `E0026`／`E0609`）。
/// **照实记**：这是**编译期照片，运行期无照片**——四个字段是纯数据声明，
/// 运行期能断言的只有「字段装得进、读得回」，而那与「字段清单恰好是这四个」不是一回事。
///
/// # 为什么四个都是 [`Opaque`]
///
/// §16 第 6 条：四者的**取值域规范未给**（§276 只给了名字与那条单调性要求）。
/// 故本层的处置是**保存事实**、不是**发明判据**——与 `MentionKind` / `Opaque` 同一条理由。
/// 取值域没给这件事**具体地**落在下面：四个串原样读回来，本层不解释它们。
#[test]
fn the_risk_facts_are_four_and_their_field_names_are_recorded_verbatim() {
    let risk = ResolutionRisk {
        effect: Opaque::from_str("e-1"),
        cost: Opaque::from_str("c-1"),
        reversibility: Opaque::from_str("r-1"),
        privacy: Opaque::from_str("p-1"),
    };

    // **不带 `..`**：这一行就是本用例的机制（清单里恰好是这四个字段）。
    let ResolutionRisk {
        effect,
        cost,
        reversibility,
        privacy,
    } = risk;

    assert_eq!(effect.as_str(), "e-1", "第一个字段是 §276 的 `effect`");
    assert_eq!(cost.as_str(), "c-1", "第二个字段是 §276 的 `cost`");
    assert_eq!(
        reversibility.as_str(),
        "r-1",
        "第三个字段是 §276 的 `reversibility`",
    );
    assert_eq!(privacy.as_str(), "p-1", "第四个字段是 §276 的 `privacy`");
}

/// §277：「最高优先级」落在**取值顺序**上——`ExplicitBinding` 把候选集**缩到 `entity` 这一个**，
/// **不是给它加权**。
///
/// # 夹具刻意选成「加权再排序」会输的那一格
///
/// 检索集里 `e-other` 的 `retrieval_score`（`0.99`）**高于**绑定的 `e-named`（`0.10`），
/// 且两枚都满足同一条非空约束 `K`。故：
///
/// - **不传 binding**：两枚都合格 ⟹ `Ambiguous`（下面先断言这一条，
///   把「binding 改变了结果」变成**被证明的**，而不是被假定的——本仓「正控制不得自证」）；
/// - **传 binding**：工作集缩到 `e-named` 那一枚 ⟹ `Resolved(e-named)`。
///
/// 高分那一枚在这里**没有任何机会**：本层的选取不按分数（`retrieval_score` 只 MAY 用于排序，
/// 而那是消费者的事，§272／§4.4）。若把这一条实现成「给 `entity` 加权后再排序、取最高分」，
/// 则 `0.99` 那一枚胜出 ⟹ 本用例红。
///
/// # 红条件（**取反档**）
///
/// 把 `ExplicitBinding` 实现成「给 `entity` 加权后再排序」（**取反档**——那时高分候选胜出），
/// 或让 `apply` 在 entity 已在检索集里时把检索集原样返回（那时结果是 `Ambiguous`）。
/// 实测见报告 §变异轮。
#[test]
fn an_explicit_binding_shrinks_the_candidate_set_instead_of_weighting_it() {
    let k = [ExplicitConstraint::Scope(ResolutionScope::Project)];
    let retrieved = vec![
        (
            cand("e-other", Some(0.99)),
            facts(ResolutionScope::Project, EntityKind::Tool),
        ),
        (
            cand("e-named", Some(0.10)),
            facts(ResolutionScope::Project, EntityKind::Tool),
        ),
    ];
    // 夹具前提：**两枚都合格**（`K` 非空，故这不是恒真的一格），且高分那一枚分数确实更高。
    assert!(
        satisfies(&retrieved[0].0, &retrieved[0].1, &k[0]),
        "夹具前提：e-other 满足 K（它是「按分数选」会选中的那一枚）",
    );
    assert!(
        satisfies(&retrieved[1].0, &retrieved[1].1, &k[0]),
        "夹具前提：e-named 满足 K",
    );
    assert!(
        retrieved[0].0.retrieval_score > retrieved[1].0.retrieval_score,
        "夹具前提：e-other 的检索分数**高于**绑定的那一枚——否则「加权再排序」与正确答案不可分",
    );

    let b = binding_to("e-named", ResolutionScope::Project);

    // —— 前提：不传 binding，两枚都合格 ⟹ `Ambiguous` ——
    let without = resolve(&retrieved, None, &k, &conf("high"), &req("high"));
    assert!(
        matches!(&without, ResolutionResult::Ambiguous(_)),
        "夹具前提：没有 binding 时两枚都合格，结果是 `Ambiguous`——\
         有了这一条，「binding 把结果改成 `Resolved`」才是被证明的：{without:?}",
    );

    // —— 传 binding：工作集缩到 `e-named` 那一枚 ——
    let with = resolve(
        &retrieved,
        Some((&b, EntityKind::Tool)),
        &k,
        &conf("high"),
        &req("high"),
    );
    let x = match &with {
        ResolutionResult::Resolved(x) => x,
        other => panic!(
            "§277：`ExplicitBinding` 在过滤之前把候选集缩到 `entity` 这一个，\
             结果应是 `Resolved(e-named)`，实得 {other:?}",
        ),
    };
    assert_eq!(
        x.entity().as_str(),
        "e-named",
        "解出的是**被点名的那一枚**，不是检索分数更高的那一枚（`ExplicitBinding` 不是加权）",
    );
}

/// §4.6 **照片 (a)**：`entity` 不在检索给的候选集里 ⟹ **注入**工作集，再照常过滤，得 `Resolved`。
///
/// # 为什么是「注入」而不是「判 `Unknown`」
///
/// §277 的「该 Binding SHOULD 在当前 scope 拥有**最高优先级**」正是要防这一格：
/// 若因为注册表没检索到就把用户点名的实体拒在集合外，等于**让检索结果否决用户**。
/// 故处置是**不新增分支，走同一条路径**：把该实体**作为一条候选注入工作集**
/// （`evidence` 标 `user_explicit`），再照常过滤（`design:783-798`）。
///
/// # 两个读数：`Resolved(entity)` ＋ `candidate_count` 比检索集多 1
///
/// 只断前一个读数的话，「注入」与「本来就在检索集里」不可分；第二个读数
/// （工作集的规模 = `|检索集| + 1`）**证明它确实被加入了**。
/// 本用例第三个读数直接读注入点自己交出的工作集（[`binding::apply`]）：
/// 最后那一枚是注入的、且它的 `evidence` **恰是 `user_explicit`**
/// ——设计 §4.6 明确要求这个标记，而结果类型（`Resolved`）只带实体，读不到它。
///
/// # 红条件（**取反档**）
///
/// 把「不在检索集」判成 `Unknown`（或让 `apply` 在这种情况下原样返回检索集）
/// ⟹ 本用例红。实测见报告 §变异轮。
#[test]
fn a_binding_whose_entity_is_not_in_the_retrieved_set_is_injected_and_resolved() {
    let k = [ExplicitConstraint::Scope(ResolutionScope::Project)];
    let retrieved = vec![(
        cand("e-other", Some(0.99)),
        facts(ResolutionScope::Global, EntityKind::Tool),
    )];
    // 夹具前提：被点名的那一枚**确实不在**检索集里；且检索集那一枚违反 `K`。
    assert!(
        !retrieved
            .iter()
            .any(|(c, _)| c.entity_id.as_str() == "e-named"),
        "夹具前提：`e-named` 不在检索集里（否则本用例钉的不是注入那一格）",
    );
    assert!(
        !satisfies(&retrieved[0].0, &retrieved[0].1, &k[0]),
        "夹具前提：检索集那一枚违反 K（故它不会是解出的那一枚）",
    );

    let b = binding_to("e-named", ResolutionScope::Project);

    let r = resolve(
        &retrieved,
        Some((&b, EntityKind::Tool)),
        &k,
        &conf("high"),
        &req("high"),
    );
    let x = match &r {
        ResolutionResult::Resolved(x) => x,
        other => panic!(
            "§4.6 照片 (a)：点名的实体不在检索集里，它被注入工作集后照常过滤，\
             应得 `Resolved(e-named)`，实得 {other:?}",
        ),
    };
    assert_eq!(
        x.entity().as_str(),
        "e-named",
        "解出的正是**被注入的那一枚**（检索结果不否决用户）",
    );
    assert_eq!(
        x.candidate_count(),
        retrieved.len() + 1,
        "第二个读数：工作集的规模比检索集**多 1**——这是「它确实被加入了」的证据",
    );

    // —— 第三个读数：注入点自己交出的工作集 ——
    let working = binding::apply(&retrieved, Some((&b, EntityKind::Tool)));
    assert_eq!(
        working.len(),
        retrieved.len() + 1,
        "`binding::apply` 的工作集 = 检索集 ＋ 注入的那一枚（至多一次）",
    );
    let injected = working.last().expect("工作集非空（检索集 ＋ 注入）");
    assert_eq!(
        injected.0.entity_id.as_str(),
        "e-named",
        "注入的那一枚就是被点名的实体",
    );
    assert_eq!(
        injected.0.evidence,
        vec!["user_explicit".to_owned()],
        "设计 §4.6 要求注入的那一枚把 `evidence` 标成 `user_explicit`",
    );
}

/// §4.6 **照片 (b)，另一侧**：注入的候选**同样**要过显式约束——binding 不是「绕过约束」的口子。
///
/// # 这一对的两侧都必须有（缺一侧会放进哪一种错实现）
///
/// - **只钉 (a)**（上面那条）：会让「注入即放行」的实现**全绿**——那种实现把注入的候选
///   直接当合格，于是 (a) 照样得 `Resolved`；
/// - **只钉 (b)**（本条）：会让「注入的那一枚不受约束」的实现**全绿**——(b) 的**结果**那一读数
///   （`reason == AllCandidatesRejected`）在「注入即放行」的实现下确实变了，
///   但它**单独**挡不住另一个方向：**注入路径整个不存在**（`apply` 什么都不注入）时，
///   本条的结果**仍是** `AllCandidatesRejected`（检索集那一枚本来就不合格）。
///
/// **故本用例多断了一个读数**（不是简报要求的两个读数之一，是本 task 补的）：
/// 工作集的**规模**必须比检索集多 1。这一条让「注入路径不存在」在本条也**变红**
/// ——实测见报告 §变异轮（那枚变异体在本条上红的正是**这一条**断言，
/// `reason` 那一条反而过了）。**没有它，(b) 就只是一条「检索集不合格」的用例，
/// 与 binding 无关。**
///
/// **照实记一条与简报措辞的出入**：简报写「只钉 (b) 会让注入路径整个不存在也全绿」，
/// 而加了上面那条规模断言之后**不再成立**。原句的判据本意是对的（(b) 的结果读数
/// 不为「注入存在」承重），但结论要按「本条还断了什么」来读。见报告。
///
/// # 夹具与上面那条只差一处：绑定的 `scope`
///
/// 上面那条绑 `Project`（与 `K` 相符 ⟹ 合格）；本条绑 `Global`（与 `K` 相悖 ⟹ 不合格）。
/// 检索集里那一枚同样违反 `K`，故**工作集里两枚都不合格** —— 这正是「它同罪」的可观察形态。
///
/// # 这是一次**扩张**，不是 §277 的原文（照录设计，收件人：规范维护者）
///
/// §277 的字面前提是「用户**选择候选**」（候选已由检索给出），而本设计处理的是
/// 「用户**直接点名**一个检索没给出的实体」。§277 **只覆盖前一半**，
/// 后一半（集合外注入）是本设计补的——`design:787-793` 把这条名分写死了，
/// 本用例的 (a)／(b) 两条就是那一次扩张的两侧照片。
///
/// # 红条件（**放宽档**）
///
/// 让注入的候选跳过约束过滤（例如 `apply` 把它单独放进一条「直接合格」的通道）
/// ⟹ 结果变成 `Resolved(e-named)`，本用例红。实测见报告 §变异轮。
#[test]
fn a_binding_does_not_exempt_the_entity_from_the_explicit_constraints() {
    let k = [ExplicitConstraint::Scope(ResolutionScope::Project)];
    let retrieved = vec![(
        cand("e-other", Some(0.99)),
        facts(ResolutionScope::Global, EntityKind::Tool),
    )];
    assert!(
        !satisfies(&retrieved[0].0, &retrieved[0].1, &k[0]),
        "夹具前提：检索集那一枚违反 K",
    );

    // 与照片 (a) 只差这一处：绑定的 scope 是 `Global`，与 `K` 的 `Project` 相悖。
    let b = binding_to("e-named", ResolutionScope::Global);

    let r = resolve(
        &retrieved,
        Some((&b, EntityKind::Tool)),
        &k,
        &conf("high"),
        &req("high"),
    );
    let u = match &r {
        ResolutionResult::Unknown(u) => u,
        other => panic!(
            "§4.6 照片 (b)：注入的那一枚同样要过显式约束（它违反 `Scope(Project)`），\
             应得 `Unknown {{ AllCandidatesRejected }}`，实得 {other:?}",
        ),
    };
    assert_eq!(
        u.reason(),
        &UnknownReason::AllCandidatesRejected,
        "binding **不豁免** `ExplicitConstraint`：两枚都不合格，条数非空 ⟹ `AllCandidatesRejected`",
    );
    assert_eq!(
        u.candidate_count(),
        retrieved.len() + 1,
        "注入**确实发生了**（工作集多了一枚）——本条钉的是「它照样被否掉」，\
         不是「注入路径不存在」（后者会让本用例不承重，见头注释）",
    );
}

/// §277：`ExplicitBinding` 的四字段**逐项**照录，且 `source = USER`。
///
/// # 机制：**不带 `..` 的穷尽解构**（四字段）＋ **不带通配的穷尽 `match`**（`source`）
///
/// - 四个字段 `mention` / `entity` / `scope` / `source` 的清单由第一段解构钉住：
///   加字段、少字段、改名 ⟹ **编译不过**（`E0027` / `E0026`）；
/// - [`BindingSource`] 的**变体集**由第二段的穷尽 `match`（**没有 `_` 臂**）钉住：
///   §277 只给了 `source = USER` 这一个值，故本层只定义 `User` 一枚——
///   加第二枚即**编译不过**（`E0004`）。
///
/// **两处都是编译期照片，运行期无红**（本仓纪律：编译不过不算变红）——
/// 运行期这一半能断的只是「装得进、读得回」。
///
/// # 为什么 `scope` 取 §274 的 [`ResolutionScope`]（而 `BindingSource` 只有一枚）
///
/// §277 的 `ExplicitBinding` 给了 `scope` 一维，而 §4.1.1 把「scope 那一维」归给
/// `ExplicitConstraint::Scope`（`src/explicit.rs` 的文件头逐字写了这条分工），
/// 后者收的是 §274 的九值——故这里同一维、同一套词汇。
#[test]
fn an_explicit_binding_is_recorded_with_its_scope_and_source() {
    let b = ExplicitBinding {
        mention: mention("GitHub"),
        entity: EntityId::from_str("connector.github"),
        scope: ResolutionScope::Session,
        source: BindingSource::User,
    };

    // **不带 `..`**：字段清单恰好是这四个。
    let ExplicitBinding {
        mention: m,
        entity,
        scope,
        source,
    } = b;

    assert_eq!(m.text, "GitHub", "`mention` 是 §270 的那一枚 mention");
    assert_eq!(m.kind.as_str(), "noun_phrase", "mention 带着它的 kind");
    assert_eq!(
        entity.as_str(),
        "connector.github",
        "`entity` 是被点名的那一枚实体",
    );
    assert_eq!(
        scope,
        ResolutionScope::Session,
        "`scope` 走 §274 的九值（与 `ExplicitConstraint::Scope` 同一套词汇）",
    );

    // **穷尽 `match`、没有 `_` 臂**：§277 只给了 `USER` 这一个取值。
    let src = match source {
        BindingSource::User => "USER",
    };
    assert_eq!(src, "USER", "§277 照录：`source = USER`");
}

/// 把 `src/` 下的每一份源码读成 `(文件名, 去掉注释的代码)`。
///
/// 用例的工作目录是 crate 根（`cargo test` 的约定，`tests/type_level.rs` 的
/// `samples_in` 用的是同一个约定），故相对路径 `src` 可读。
///
/// # 为什么要先把注释剥掉
///
/// 判据是「`src/` 里**没有**那个形状」——而**注释里写着这句话本身就是必须的**：
/// `src/risk.rs` 的文档逐条列出了「本模块没有的东西」，那几行**必然**含这两个模式
/// （不然那段文档说不清自己在说什么）。若连注释一起扫，这条用例会**被自己的说明文字判红**。
/// **故判据取「代码行」**——与 `tests/resolver.rs::resolve_returns_the_result_type_with_no_err_arm`
/// 那条「去掉注释之后 `grep unwrap` 零命中」同一条口径：注释不是实现。
///
/// # 这个剥法到哪为止（照实记）
///
/// 它是一个**极简词法扫描**，认四种状态：代码 / 行注释 / 块注释 / 字符串字面量
/// （字符串里出现 `//` 不算注释）。**它不认识**：原始串 `r"…"`、字符字面量 `'/'`、
/// 字节串、以及嵌套块注释。扫的语料是本 crate 自己的 `src/`（今天没有那几种形态），
/// 且调用方有一条**正控制**（某个确实存在的串必须能被找到）兜住「扫描把整个文件剥没了」
/// 这一类坏法。它**不**主张「扫描结果等于编译器眼里的代码」。
fn read_all_sources() -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = std::fs::read_dir("src")
        .expect("`src/` 应可读（用例的工作目录是 crate 根）")
        .map(|entry| {
            let path = entry.expect("目录项应可读").path();
            let name = path
                .file_name()
                .expect("路径应有文件名")
                .to_string_lossy()
                .into_owned();
            let body = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("`{}` 应可读：{e}", path.display()));
            (name, strip_comments(&body))
        })
        .collect();
    out.sort();
    assert!(
        !out.is_empty(),
        "`src/` 下一份源码都没读到——扫描机制本身坏了，下面的零命中没有意义",
    );
    out
}

/// 剥掉源码里的行注释与块注释（字符串字面量内不算注释），只留代码。
///
/// 限度见 [`read_all_sources`] 的文档——它是**扫描工具**，不是词法器。
fn strip_comments(src: &str) -> String {
    // 四种状态：0 = 代码，1 = 行注释，2 = 块注释，3 = 字符串字面量。
    let chars: Vec<char> = src.chars().collect();
    let mut out = String::with_capacity(src.len());
    let mut state = 0u8;
    let mut i = 0usize;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        match state {
            0 if c == '/' && next == Some('/') => {
                state = 1;
                i += 2;
            }
            0 if c == '/' && next == Some('*') => {
                state = 2;
                i += 2;
            }
            0 => {
                if c == '"' {
                    state = 3;
                }
                out.push(c);
                i += 1;
            }
            1 => {
                if c == '\n' {
                    state = 0;
                    out.push(c);
                }
                i += 1;
            }
            2 if c == '*' && next == Some('/') => {
                state = 0;
                i += 2;
            }
            2 => i += 1,
            _ => {
                // 字符串字面量：整段照抄，含转义（`\"` 不算结束）。
                out.push(c);
                if c == '\\' {
                    if let Some(escaped) = next {
                        out.push(escaped);
                        i += 2;
                        continue;
                    }
                } else if c == '"' {
                    state = 0;
                }
                i += 1;
            }
        }
    }
    out
}
