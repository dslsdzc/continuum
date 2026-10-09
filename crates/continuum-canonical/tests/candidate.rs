//! Task 3 的用例：候选面（§4.3 的 `EntityId` 与 `CandidateSource`、§272 的 `Candidate`）。
//!
//! 每条用例的头注释写明**它钉的是什么**与**红条件是什么**；红条件在本 task 的变异轮里
//! 逐条真的取过，取到的结果记在 `.superpowers/sdd/p4-task-3-report.md`。
//!
//! # 本文件只有运行期那半边，编译期那半边在 `tests/type_level.rs`
//!
//! 简报给 `entity_id_round_trips_and_has_no_public_literal_construction` 与
//! `a_candidate_source_can_only_return_a_set` 各写了**两半**：(a) 运行期断言、(b) `trybuild` 样例。
//! `trybuild` 必须由一个 `#[test]` 去驱动（它自己会建一个临时工程并跑 `cargo`），
//! 故 (b) 那半边落在 `tests/type_level.rs`，本文件是 (a)；两半合起来才是简报里那一条要求。
//! **分成两处是本仓既有的体例**：`continuum-node` 的「`ComputeNode` 字段私有」
//! 同样只有 `tests/compile_fail/compute_node_fields_are_private.rs` 那一半是直接的
//! 不可表达性证据（运行期那一半证明不了「写不出来」）。
//!
//! # 「没有指向 `ToolId` / `ModelId` / `ConnectorId` 的转换」那一半**不靠 `trybuild` 钉**
//!
//! 简报点名的这一句照录在此，免得后来者以为它漏了：本 crate 的 `ALLOWED` 条目
//! （`crates/continuum-runtime/tests/dependency_direction.rs:156`，行号本 task 实测）
//! 里没有 `continuum-core`，**那三个类型在本 crate 里根本写不出来**——写了即 `E0432`
//! 未解析导入，而**按本仓纪律「编译不过」不算变红**，那种样例证明不了任何东西。
//! 故这一半的判据是 **`ALLOWED` 条目 ＋ Task 21 的 grep**（依赖边为零），
//! 不是一条 `trybuild` 样例。

use continuum_canonical::{
    Candidate, CandidateSource, CandidateSources, EntityId, Mention, detect, normalize,
};

/// 夹具：一枚**真实切出来**的 mention（`请把它改掉` → `它`）。
///
/// 与 `tests/canonical.rs` 的 `two_mentions` 同一用意：让走 `CandidateSource::retrieve` 的
/// 那些用例用的是 `detect` 真产出的 `Mention`，而不是手拼的一枚
/// （手拼的话，`retrieve(&self, m: &Mention)` 的入参形状就没有被真实路径核过）。
fn a_mention() -> Mention {
    let surface = normalize("请把它改掉");
    let mut mentions = detect(&surface);
    assert_eq!(
        mentions.len(),
        1,
        "夹具前提：这一句要恰好切出一枚 mention（`it` 类代词）",
    );
    mentions.remove(0)
}

/// 夹具：一枚候选。`evidence` 与 `retrieval_score` 由调用方给，免得每个用例都写全三个字段。
fn candidate(entity_id: &str, score: Option<f64>) -> Candidate {
    Candidate {
        entity_id: EntityId::from_str(entity_id),
        evidence: Vec::new(),
        retrieval_score: score,
    }
}

/// 夹具：**只实现 `retrieve`** 的一个来源，返回固定的一批候选。
///
/// 它同时是「本层不为单实体定义方法」在**运行期**这一侧的对照物：
/// 一个只能交出集合的来源，本层拿它没有任何「取唯一那枚」的口子。
struct FixedSource(Vec<Candidate>);

impl CandidateSource for FixedSource {
    fn retrieve(&self, _m: &Mention) -> Vec<Candidate> {
        self.0.clone()
    }
}

/// 读回一批候选的 `entity_id`（**经 `as_str`**）——`EntityId` 没有 `PartialEq`
/// （与 `crate::Opaque` 同一条理由：本类型不提供语义判定），故比较一律走 `as_str`。
fn ids(candidates: &[Candidate]) -> Vec<&str> {
    candidates
        .iter()
        .map(|c| c.entity_id.as_str())
        .collect()
}

/// `EntityId` 经 `as_str` / `from_str` **往返**，且**没有公开的字面量构造**。
///
/// # (a) 往返这一半（本文件）
///
/// 钉的是：`from_str` 与 `as_str` 互为逆，**不改写、不裁剪**；
/// 且**空串的处置是「接受」**——不是 `Err`、不是 `panic`、不是静默换成别的东西。
/// 「接受」是本类型**具体**取的那一种处置（另一条路是 fail-closed 拒掉空串），
/// 本用例把取的那一种**逐条断言出来**，而不是只断言「不 panic」。
///
/// 红条件（**取反档**，本 task 实测取过）：把 `EntityId::from_str` 改成拒空串
/// （例如 `assert!(!s.is_empty(), …)`）⇒ 空串那一条红。
/// 另一枚（**取反档**）：在 `from_str` 里 `s.trim()` ⇒ 「不改写、不裁剪」那一条红。
///
/// # (b) 不可表达性那一半（`tests/type_level.rs`）
///
/// `EntityId` 的里子串私有 ⇒ crate 外**写不出** `EntityId("x".to_owned())`。
/// 这半边的用例在 `tests/type_level.rs`（同一个要求名出现在那里的头注释里），
/// 本文件证明不了它——运行期用例无论怎么写都只能证明「我没这么构造过」。
#[test]
fn entity_id_round_trips_and_has_no_public_literal_construction() {
    // 多条往返。含空串、含前后空白、含非 ASCII、含与 Surface 大小写折叠后不同的形态
    // （`EntityId` 不做任何折叠：它是**不透明**的，规范化过的东西才进本层，见 `src/candidate.rs`）。
    let values = [
        "",
        "tool.github",
        "  tool.github  ",
        "带空格的 id",
        "Mixed_Case-1",
        "汉字与 ASCII 混写",
        "\u{ff21}\u{ff22}", // 全角 ＡＢ：`EntityId` **不**把它折成 `ab`
    ];
    for raw in values {
        let id = EntityId::from_str(raw);
        assert_eq!(
            id.as_str(),
            raw,
            "`from_str` / `as_str` 必须互为逆，且不改写、不裁剪：{raw:?}"
        );
    }

    // 空串那一条**单独写出来**，因为它的处置是本层**选定**的一种，不是「顺带如此」。
    let empty = EntityId::from_str("");
    assert_eq!(
        empty.as_str(),
        "",
        "空串是**被接受**的（往返得空串）——本类型无失败路径，不接受空串的那种处置要写成另一条判据，本层没取"
    );

    // 长串也走一遍：`from_str` 是 `to_owned()`，长度不该成为判据。
    let long = "x".repeat(4096);
    assert_eq!(EntityId::from_str(&long).as_str(), long);

    // 两枚**同字面**的 `EntityId` 读回同一个串——`EntityId` 不做去重也不做规范化，
    // 这两件事都不在本类型的判据里（没有 `PartialEq`，故这里比的是 `as_str`）。
    assert_eq!(
        EntityId::from_str("tool.github").as_str(),
        EntityId::from_str("tool.github").as_str()
    );
}

/// `CandidateSource` **只能返回集合**（§197：候选生成负责高召回，不直接决定最终语义）。
///
/// # 钉的两件事
///
/// 1. **本层不为单实体定义方法**——这一半的照片是 `trybuild`
///    （正例：只实现 `retrieve` 的实现编译通过；反例：`source.pick_one(&m)` 编译不过），
///    在 `tests/type_level.rs`；本文件证明不了它。
/// 2. **本层拿到集合之后也不会把它塌成一枚**——这一半是本文件的运行期断言：
///    多个来源各交一批，装配后的**检索集按装配顺序拼接**，空来源不贡献、
///    重复的 `entity_id` **不去重**（去重会把「两个来源各自认为可能」这一事实抹掉，
///    而那是一条本层没有判据的判定，见 `src/candidate.rs` 的 `retrieve_all`）。
///
/// # 这条守卫的**限度**（简报要求写明）
///
/// 它钉的是「**本层**不为单实体定义方法」，**不钉**「实现者内部返回唯一候选」——
/// 一个来源内部只交出一枚候选，本层看不出、也不该看出（§197 的高召回要求是**手段**层面的事，
/// 本层没有召回率判据，设计 §15.2 第 3 条）。「候选集非空且唯一合格才 `Resolved`」
/// 那一侧由 Task 5 的四条断言钉，不在本用例的射程里。
///
/// 红条件（**取反档**，本 task 实测取过）：把 `CandidateSources::retrieve_all` 的拼接改成
/// 「只取第一枚」（如 `.take(1)`）⇒ 本用例红在条数那一条上。
#[test]
fn a_candidate_source_can_only_return_a_set() {
    let m = a_mention();

    let sources = CandidateSources::new(vec![
        Box::new(FixedSource(vec![candidate("a", None)])),
        Box::new(FixedSource(Vec::new())), // 空来源：高召回手段完全可能零命中，本层不许把它当异常
        Box::new(FixedSource(vec![
            candidate("b1", Some(0.9)),
            candidate("b2", Some(0.1)),
        ])),
    ]);

    let retrieved = sources.retrieve_all(&m);
    assert_eq!(
        ids(&retrieved),
        ["a", "b1", "b2"],
        "检索集按**装配顺序**拼接：三个来源共交出三枚，空来源不贡献、顺序不重排"
    );
    assert_eq!(
        retrieved.len(),
        3,
        "**不塌成一枚**：本层没有「挑出唯一那枚」的口子，集合交出来就是集合"
    );

    // 空装配（一个来源都没有）⇒ 空检索集。§201 的「0 candidate → NIL / Expand Retrieval」
    // 由下游的 §4.2 断言 C 处置，本层不在这里发明一个替代品（不 panic、不造占位候选）。
    let none = CandidateSources::new(Vec::new());
    assert!(
        none.retrieve_all(&m).is_empty(),
        "零来源 ⇒ 空检索集；本层不造占位候选"
    );

    // **不去重**：两个来源各自给出同一个 `entity_id` 时，两枚都在。
    // 「去重」会把「两个手段各自认为可能」压成一件事，而 §197 的七种手段是**互相独立的高召回来源**，
    // 本层没有判据决定它们的票该怎么合——故不合并。
    let duplicated = CandidateSources::new(vec![
        Box::new(FixedSource(vec![candidate("same", None)])),
        Box::new(FixedSource(vec![candidate("same", Some(0.7))])),
    ]);
    let retrieved = duplicated.retrieve_all(&m);
    assert_eq!(
        ids(&retrieved),
        ["same", "same"],
        "**不去重**：两个来源各自给出同一枚 `entity_id` 时两枚都在"
    );
    // 两枚虽然 `entity_id` 相同，各自的 `retrieval_score` 照原样留着——
    // 本层不做「合并分数」这种判定（§272 只把 `retrieval_score` 定为排序用）。
    assert_eq!(retrieved[0].retrieval_score, None);
    assert_eq!(retrieved[1].retrieval_score, Some(0.7));
}

/// `Candidate` **恰好**记着 §272 的三个字段，一个不多一个不少。
///
/// 机制 = **不带 `..` 的穷尽解构**：本字段清单是**编译期**照片，
/// 任一侧加字段即**编译不过**（改结构体 ⇒ `E0027` pattern 未覆盖新字段；
/// 改 pattern ⇒ `E0026` 结构体没有那个字段）。
///
/// **明写**：这是**编译期**照片，**运行期无照片**——`Candidate` 是纯数据声明
/// （三个 `pub` 字段，没有方法、没有不变量），运行期能断言的就只有「字段装得进、读得回」，
/// 而那与「字段清单**恰好**是这三个」不是一回事。加字段这件事**没有变红的观测**，
/// 它只有一条编译错误，而按本仓纪律「编译不过」不算红。
///
/// 字段名照 §272（`docs/spec/05-normative.md:1378-1392`，行号本 task 实测）：
/// `entity_id` / `evidence[]` / `retrieval_score?`。
#[test]
fn the_candidate_records_exactly_the_three_fields_of_272() {
    let c = Candidate {
        entity_id: EntityId::from_str("tool.github"),
        evidence: vec!["alias registry: gh".to_owned()],
        retrieval_score: Some(0.75),
    };

    // **不带 `..`**：这一行就是本用例的全部机制。
    let Candidate {
        entity_id,
        evidence,
        retrieval_score,
    } = c;

    assert_eq!(entity_id.as_str(), "tool.github");
    assert_eq!(evidence, vec!["alias registry: gh".to_owned()]);
    assert_eq!(retrieval_score, Some(0.75));
}

/// `retrieval_score` 是**可选**的，且**缺席与 `Some(0.0)` 可区分**。
///
/// 两条：`None` 一条、`Some(0.0)` 一条，各自断言**读回的是哪一枚**。
/// 这不是把同一件事写两遍：「缺席」与「分数是零」在 §272 里是两件事
/// （`retrieval_score?` 的 `?` 就是「可以没有」），而把它们塌成一个默认 `0.0`
/// 会让「这个来源没给分数」与「这个来源给了 0 分」再也分不开——
/// 下游任何要按分数排序的地方都会把前者当成后者。
///
/// 红条件（**移除档**，简报写死）：把 `retrieval_score: Option<f64>` 换成 `f64`（默认 `0.0`）。
/// **实测结果要照实记**：这一枚变异体**不是运行期变红**，它让本文件**编译不过**
/// （`None` 与 `matches!(…, None)` 在新类型下都不成立），而按本仓纪律
/// 「`could not compile` 不算红」。故本条守卫的性质是**编译期照片**：
/// 它的力量来自类型，不来自断言——报告里把这两件事分开记。
#[test]
fn a_score_is_optional_and_absent_is_not_zero() {
    let absent = candidate("e-absent", None);
    let zero = candidate("e-zero", Some(0.0));

    assert!(
        matches!(absent.retrieval_score, None),
        "**缺席**要读回 `None`：来源没给分数就是没给"
    );
    assert!(
        matches!(zero.retrieval_score, Some(v) if v == 0.0),
        "**`Some(0.0)`** 要读回 `Some(0.0)`：给了零分就是给了零分"
    );

    // 「可区分」这一条是本用例的名字所在：两枚的 `is_none` 必须**不同**。
    assert_ne!(
        absent.retrieval_score.is_none(),
        zero.retrieval_score.is_none(),
        "缺席与 `Some(0.0)` 必须可区分——塌成同一个默认值即红"
    );
}
