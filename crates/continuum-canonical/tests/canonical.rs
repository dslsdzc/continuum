//! Task 2 的用例：`Canonicalized` 的两支（§3.2）、Ambiguity Manager 的 `finish`（§3.1），
//! 以及 Temporal Resolver 的「`Now` 是入参」（§3.1）。
//!
//! 每条用例的头注释写明**它钉的是什么**与**红条件是什么**；红条件在本 task 的变异轮里
//! 逐条真的取过，取到的结果记在 `.superpowers/sdd/p4-task-2-report.md`。
//!
//! # 这些用例今天钉的是**分支机制**，不是规范化的真实质量
//!
//! 真实候选来源今天**零个**（设计 §16 第 5 条，`design:2319-2322`），
//! 故下面每一条 `ResolutionResult` 都是**夹具直接给的**——本层今天没有任何东西
//! 真的去解析一枚 mention。这与设计 §15.2 第 3 条（`design:2264-2266`）写的是同一件事：
//! 「能拍的只有机制：四个来源接口、闭集不变量、四态分支」。

use continuum_canonical::{
    Canonicalized, Mention, MentionKind, Now, ResolutionResult, TemporalBinding, detect, finish,
    normalize, resolve_temporal,
};

/// 一枚 UTC 日的毫秒数。本层按**恒定** 86400000 毫秒/日推进（不处理时区与夏令时）——
/// 这是本层自己标的假设，规范未给日历（见 `src/temporal.rs` 的相应注释）。
const DAY_MS: i64 = 86_400_000;

/// 夹具：两枚**真实切出来**的 mention（`请把它和 this 都改掉` → `它`、`this`）。
///
/// 用它而不是手写 `Mention`，是为了让「至少两枚 mention」这一侧也走 `detect` 的真实路径。
fn two_mentions() -> Vec<Mention> {
    let surface = normalize("请把它和 this 都改掉");
    let mentions = detect(&surface);
    assert_eq!(
        mentions.len(),
        2,
        "夹具前提：这一句要切出两枚 mention（它是 Task 1 的既有夹具）",
    );
    mentions
}

/// 每一枚 mention 都是 `Resolved` ⟹ **`Canonical` 支**。
///
/// 钉的是 §3.2（`design:340-354`）的 `Canonical` 支的**入口条件**。
///
/// 红条件（放宽档）：把 `finish` 写成恒返回 `Canonical`（不看每枚的状态）。
/// 届时本条仍绿——**它单独不足以钉住这条**，另一侧由
/// `one_unresolved_mention_gives_the_needs_resolution_arm_*` 三条钉住。
#[test]
fn every_mention_resolved_gives_the_canonical_arm() {
    let mentions = two_mentions();
    let parts: Vec<(Mention, ResolutionResult)> = mentions
        .iter()
        .cloned()
        .map(|m| (m, ResolutionResult::Resolved))
        .collect();

    match finish(parts) {
        Canonicalized::Canonical(representation) => {
            // 载荷里照录的正是那两枚 mention，按输入次序——**不抽代表**。
            assert_eq!(representation.mentions(), mentions.as_slice());
        }
        need_resolution => panic!("每个 mention 都是 Resolved，不该挂起：{need_resolution:?}"),
    }
}

/// 逐条断定「挂起时列表里**恰好**是那一枚没解的 mention」。三条各钉一枚未解态。
///
/// 钉的是两件事：
/// 1. 有任何一枚没解 ⟹ **`NeedsResolution` 支**（不是 `Canonical`）；
/// 2. 列表**恰好**是那一枚——**不是「列表非空」**。列表里混进已解的 mention，
///    会让「挂起时只说清哪些没解」这件事失真；故断言长度与**逐项**身份。
///
/// **为什么拆成三条而不是一条**：一个 `#[test]` 里断言**顺序执行**，第一条失败即 panic，
/// 后面两枚未解态根本不会被执行——全量门因此只给得出「这个测试红了」，
/// 给不出「哪几条臂落空」。本条分三枚，是为了让红集能逐臂分辨（Task 1 修复轮的同一条教训）。
///
/// 红条件（放宽档）：`finish` 把未解的 mention 也塞进 `Canonical` 支的表示
/// （或恒返回 `Canonical`），三条全红。
#[test]
fn one_unresolved_mention_gives_the_needs_resolution_arm_confirm_required() {
    let mentions = two_mentions();
    let parts = vec![
        (mentions[0].clone(), ResolutionResult::Resolved),
        (mentions[1].clone(), ResolutionResult::ConfirmRequired),
    ];

    match finish(parts) {
        Canonicalized::NeedsResolution(unresolved) => {
            assert_eq!(unresolved.len(), 1, "列表里只该有那一枚没解的 mention");
            assert_eq!(unresolved[0].mention(), &mentions[1]);
            assert_eq!(unresolved[0].state(), &ResolutionResult::ConfirmRequired);
        }
        canonical => panic!("有一枚 ConfirmRequired，不该落 Canonical 支：{canonical:?}"),
    }
}

/// 同 `one_unresolved_mention_gives_the_needs_resolution_arm_confirm_required`，
/// 未解态取 `Ambiguous`。
#[test]
fn one_unresolved_mention_gives_the_needs_resolution_arm_ambiguous() {
    let mentions = two_mentions();
    let parts = vec![
        (mentions[0].clone(), ResolutionResult::Resolved),
        (mentions[1].clone(), ResolutionResult::Ambiguous),
    ];

    match finish(parts) {
        Canonicalized::NeedsResolution(unresolved) => {
            assert_eq!(unresolved.len(), 1, "列表里只该有那一枚没解的 mention");
            assert_eq!(unresolved[0].mention(), &mentions[1]);
            assert_eq!(unresolved[0].state(), &ResolutionResult::Ambiguous);
        }
        canonical => panic!("有一枚 Ambiguous，不该落 Canonical 支：{canonical:?}"),
    }
}

/// 同 `one_unresolved_mention_gives_the_needs_resolution_arm_confirm_required`，
/// 未解态取 `Unknown`。
#[test]
fn one_unresolved_mention_gives_the_needs_resolution_arm_unknown() {
    let mentions = two_mentions();
    let parts = vec![
        (mentions[0].clone(), ResolutionResult::Resolved),
        (mentions[1].clone(), ResolutionResult::Unknown),
    ];

    match finish(parts) {
        Canonicalized::NeedsResolution(unresolved) => {
            assert_eq!(unresolved.len(), 1, "列表里只该有那一枚没解的 mention");
            assert_eq!(unresolved[0].mention(), &mentions[1]);
            assert_eq!(unresolved[0].state(), &ResolutionResult::Unknown);
        }
        canonical => panic!("有一枚 Unknown，不该落 Canonical 支：{canonical:?}"),
    }
}

/// §3.2（`design:352-354`）：**一支是另一支的补集**，不是两个可以同时为真
/// 也可以同时为假的标志位。
///
/// # 这条用例的两半，证明力不同，分开写
///
/// **运行期那一半**（本条真的有照片的部分）：**夹具集必须两支都走到**——
/// 即 `arms` 那两个断言（「有落 `Canonical` 支的」「有落 `NeedsResolution` 支的」）。
/// 它们钉的是 `finish` 对这批夹具**不是恒返回同一支**。
/// 实测红条件：M1（`NeedsResolution` 那一支消失）下红的是
/// `arms.contains(&"needs_resolution")` 那一条，另加三条 `one_unresolved_*`
/// 与 `four_kinds_*`（报告 §9 的变异实测）。
///
/// **「恰有一支成立」不在本条的运行期照片里**——**不要**在这里写
/// `is_canonical ^ is_needs_resolution`：`Canonicalized` 今天恰有两支，
/// 那个异或对 `finish` 能返回的**任何**值都恒真，任何实现改动都响不了它；
/// 一条恒真的断言比没有断言更坏（后来者会以为那里钉着互斥性）。
/// 「不能两支都真」由**类型**给：`Canonicalized` 是和类型（一支就是一个值），
/// 不是两个可同真的标志位。
///
/// **编译期那一半**：`arm` 里的 `match` **穷尽、不带 `..`**——
/// 给 `Canonicalized` 加第三支即**编译不过**；「不能两支都假」（没有第三支）
/// 就是靠这一条钉在源码里的。
/// **编译不过不算变红**（本仓纪律：那是「编译器拒绝了另一份代码」，不是「这份代码的行为变了」），
/// 故这一半是**类型保证**，不是本用例运行期能取到的红。
///
/// **「`Canonical` 里带着未解 mention」这种值造不出来**这一半，
/// 本 task 只做到「`CanonicalRepresentation` 的字段私有、构造入口只有 `finish`」；
/// 它在外部的不可构造性由 **Task 3 的 `tests/type_level.rs`** 用 trybuild 钉
/// （`plans/2026-10-06-p4-semantic-layer.md:515-516` 把该文件与 `trybuild` 这个 dev-dep
/// 排在 Task 3）——**本 task 不动 `Cargo.toml`**，故那半张照片不在本 task。
///
/// 红条件（移除档）：给 `Canonicalized` 加一个 `bool` 标志位、让两支可同真。
/// **这一枚取不到**：那要改掉和类型的签名（层③——「只能靠改签名表达」），
/// 按纪律 0 的可观察者只是调用侧编译失败，**不算红**。
/// 本 task 实际取到的红条件记在报告的变异轮里（见 `finish` 那两枚）。
#[test]
fn the_two_arms_are_complements() {
    /// 穷尽、**不带 `..`**：加第三支即编译不过。
    fn arm(c: &Canonicalized) -> &'static str {
        match c {
            Canonicalized::Canonical(_) => "canonical",
            Canonicalized::NeedsResolution(_) => "needs_resolution",
        }
    }

    let mentions = two_mentions();
    let fixtures: Vec<Vec<(Mention, ResolutionResult)>> = vec![
        // 零 mention：每个 mention 都是 `Resolved` **平凡成立** ⟹ 落 `Canonical`。
        // 零命中 fail-closed 那一侧的形态由 Task 1 的
        // `an_input_without_any_mention_yields_an_empty_vec` 钉住。
        vec![],
        vec![(mentions[0].clone(), ResolutionResult::Resolved)],
        vec![
            (mentions[0].clone(), ResolutionResult::Resolved),
            (mentions[1].clone(), ResolutionResult::Resolved),
        ],
        // 混一枚未解 ⟹ 另一支。
        vec![
            (mentions[0].clone(), ResolutionResult::Resolved),
            (mentions[1].clone(), ResolutionResult::Unknown),
        ],
        vec![(mentions[0].clone(), ResolutionResult::Ambiguous)],
    ];

    // 两支**都**要被走到——否则下面那句「恰有一支」会被一个恒返回同一支的实现满足。
    let arms: Vec<&'static str> = fixtures
        .iter()
        .map(|parts| arm(&finish(parts.clone())))
        .collect();
    assert!(
        arms.contains(&"canonical"),
        "夹具集里必须有落 Canonical 支的：{arms:?}",
    );
    assert!(
        arms.contains(&"needs_resolution"),
        "夹具集里必须有落 NeedsResolution 支的：{arms:?}",
    );
    // 到这里为止。**不写「恰有一支」的异或断言**：见上面头注释，
    // `Canonicalized` 恰有两支时那个异或恒真，是一条没有照片的断言。
}

/// §2.4 第 1 条（`docs/02-工程.md:130`）的落点：含**错别字／简称／中英混写／代词**
/// 的输入**各一条**，逐条断言结果落在哪一支，且**不 panic、不产出空 mention 的 `Canonical`**。
///
/// # 这四条钉的是**分支机制**，不是规范化的真实质量
///
/// 四条输入里被 `detect` 切出来的，**都不是那枚「脏」词本身**，而是同一句里的代词
/// （`那个`＋`this` / `这个` / `it` / `它`）——本层的两张字面表只覆盖代词与上下文引用
/// （Task 1 的 `PRONOUNS` / `CONTEXT_REFERENCES`），**错别字、简称、中英混写今天
/// 没有任何一张表**（设计 §16 第 5b 条，`design:2309-2318`）。故这四条走通的是
/// 「输入进得来、分支照走」，**不是**「`teh` 被认成了 `the`」。
/// 设计 §15.2 第 3 条（`design:2264-2266`）把这件事写死了：
/// 真实候选来源零个，能拍的只有机制。
///
/// **每一枚 `ResolutionResult` 都是夹具直接给的**：本 task 没有解析器
/// （`resolve` 在 Task 4/5）。故「落哪一支」在本条里**完全由夹具决定**——
/// 这正是为什么它钉的是分支机制。
///
/// 红条件（放宽档）：把两类输入吞成同一支而不区分（`finish` 恒返回 `Canonical`），
/// 四条里「留一枚未解」的那半边全红。
///
/// 「半边二」的 `unresolved.len() == 1` 只在**有一条夹具切出两枚 mention** 时才可观察
/// （否则挂起列表里本来就只有那一枚）；前提二把那件事写成断言，
/// 第一条夹具（`那个`＋`this`）就是为它准备的。见报告 §9 的 M2 实测。
#[test]
fn four_kinds_of_dirty_input_each_yield_one_of_the_two_arms() {
    // 每条输入都**同时**带一处脏词与一枚代词，故四条都非空命中——
    // 否则「留一枚未解」那半边会退化成恒真的空集断言。
    // 第一条**带两枚代词**（`那个`、`this`），理由见下面前提二。
    let inputs = [
        (
            "把 teh 那个 test 和 this 都删掉",
            "错别字（teh）＋两枚代词（那个、this）",
        ),
        ("把 cfg 这个改掉", "简称（cfg）＋代词（这个）"),
        ("please update it 的配置", "中英混写＋代词（it）"),
        ("请把它改掉", "代词（它）"),
    ];

    let fixtures: Vec<(&str, Vec<Mention>)> = inputs
        .iter()
        .map(|(raw, why)| (*why, detect(&normalize(raw))))
        .collect();

    // 前提一：每条都切出 mention。
    for (why, mentions) in &fixtures {
        assert!(!mentions.is_empty(), "{why}：夹具前提，这句要切出 mention");
    }
    // 前提二：**至少有一条切出两枚**。`unresolved.len() == 1` 这条断言只在
    // `resolved` 侧非空时才可观察：单 mention 夹具下挂起列表里本来就只有那一枚，
    // 「已解的不能混进挂起列表」这件事**无从发生**（报告 §9 的 M2 实测：
    // 全用单 mention 夹具时，把已解的并进挂起列表的变异体在这一条上不红）。
    assert!(
        fixtures.iter().any(|(_, mentions)| mentions.len() >= 2),
        "夹具集里必须有一条切出两枚 mention，否则 len==1 是平凡断言：{:?}",
        fixtures
            .iter()
            .map(|(why, m)| (*why, m.len()))
            .collect::<Vec<_>>(),
    );

    for (why, mentions) in &fixtures {
        // 「**不产出空 mention 的 `Canonical`**」：切出来的每一枚都要有非空 text。
        for m in mentions {
            assert!(
                !m.text.is_empty(),
                "{why}：mention 的 text 不得为空（不产出空 mention）",
            );
        }

        // 半边一：每一枚都解出 ⟹ `Canonical` 支。
        let all_resolved: Vec<(Mention, ResolutionResult)> = mentions
            .iter()
            .cloned()
            .map(|m| (m, ResolutionResult::Resolved))
            .collect();
        let first = finish(all_resolved);
        assert!(
            matches!(&first, Canonicalized::Canonical(_)),
            "{why}：全解应落 Canonical 支，实得 {first:?}",
        );

        // 半边二：把**最后一枚**留成未解 ⟹ `NeedsResolution` 支，且列表**恰是那一枚**。
        let mut parts: Vec<(Mention, ResolutionResult)> = mentions
            .iter()
            .cloned()
            .map(|m| (m, ResolutionResult::Resolved))
            .collect();
        let last = parts.len() - 1;
        parts[last].1 = ResolutionResult::Unknown;
        match finish(parts) {
            Canonicalized::NeedsResolution(unresolved) => {
                assert_eq!(unresolved.len(), 1, "{why}：只该挂起那一枚");
                assert_eq!(unresolved[0].mention(), &mentions[last], "{why}");
            }
            canonical => panic!("{why}：留了一枚未解，不该落 Canonical 支：{canonical:?}"),
        }
    }
}

/// Temporal Resolver（§3.1 的表，`design:332`）：**「`Now`」是入参**，
/// 本层不取系统时钟——与 P2 的入口同一条纪律。
///
/// 钉的是：**结果随入参 `now` 变**。同一枚 mention 配两个相隔很远的 `Now`，
/// 得到两个不同的 `TemporalBinding`；且两个值各自**逐字**等于「基准 ＋ 表里那一格」。
/// 后两个 `assert_eq!` 不能省——只有 `assert_ne!` 的话，任何**把 `now` 掺进结果**
/// 的写法（哪怕掺法毫无意义）都能过。
///
/// 红条件（取反档）：`resolve_temporal` 的函数体内部取系统时钟
/// （如 `SystemTime::now()`），**不使用入参 `now`**——那时两个不同的 `Now` 给出**相同**结果，
/// 本用例的 `assert_ne!` 与两条 `assert_eq!` 全红。
///
/// # 第二侧
///
/// 本条是**运行期**那一侧。**源码面**那一侧（本 crate 里 `SystemTime::now` 零命中）
/// 由 **Task 21 的收尾复核**跑（`plans/2026-10-06-p4-semantic-layer.md:2275`）。
///
/// # 夹具为什么是手写的
///
/// `detect` 的两张表（代词、上下文引用）里**没有时态表达**，故真实路径今天切不出一枚
/// 时态 mention——**这一条的照片因此是手工夹具给的**，本层今天没有真实的时态 mention 来源
/// （与设计 §15.2 第 3 条同一体的限度）。要补得给 `detect` 加一张时态字面表，
/// 那是 Task 1 的地盘、不在本 task。
#[test]
fn temporal_resolution_takes_now_as_an_argument() {
    // `明天` 两个三字节字符 ⇒ 字节偏移 0..6。
    let tomorrow = Mention {
        span: 0..6,
        text: "明天".to_owned(),
        kind: MentionKind::from_str("temporal"),
    };
    let pronoun = Mention {
        span: 0..3,
        text: "它".to_owned(),
        kind: MentionKind::from_str("pronoun"),
    };

    let now = Now::from_millis(0);
    let far = Now::from_millis(30 * DAY_MS);

    let near_binding = resolve_temporal(&tomorrow, &now);
    let far_binding = resolve_temporal(&tomorrow, &far);

    assert_eq!(near_binding, TemporalBinding::At(DAY_MS));
    assert_eq!(far_binding, TemporalBinding::At(31 * DAY_MS));
    assert_ne!(
        near_binding, far_binding,
        "同一 mention 配两个相隔很远的 Now 必须得到不同结果——否则入参没被用上",
    );

    // 反面：mention 里没有本层认识的时态表达 ⟹ **不绑定**（fail-closed，不发明一个时刻）。
    // 两侧都断：两个 `Now` 下都是 `Unbound`。
    assert_eq!(resolve_temporal(&pronoun, &now), TemporalBinding::Unbound);
    assert_eq!(resolve_temporal(&pronoun, &far), TemporalBinding::Unbound);

    // 注：**英文那一半**（`RELATIVE_DAYS` 里 `// 英文` 块下的 7 条）不在这条里，
    // 它们逐条有照片，在 `english_relative_expressions_each_resolve_to_a_hand_written_offset`。
}

/// 一枚手写的时态 mention（`detect` 切不出时态表达——见上一条的「夹具为什么是手写的」）。
///
/// `span` 取整串的字节区间；`resolve_temporal` 今天只看 `text`，
/// 这里给一个**与 text 相符**的 span，免得夹具自身写着一处假坐标。
fn temporal_mention(text: &str) -> Mention {
    Mention {
        span: 0..text.len(),
        text: text.to_owned(),
        kind: MentionKind::from_str("temporal"),
    }
}

/// `RELATIVE_DAYS`（`src/temporal.rs:106-127`）的**英文那一半**：7 条逐条有照片。
///
/// # 期望值为什么是本文件手写的
///
/// 下面 7 个期望值是**本文件里独立写下的字面常量**，**不是**从 `RELATIVE_DAYS` 读出来的。
/// 从被测清单里取语料的「正控制」是恒真的假照片——那样写的话，把表里的值改错
/// 甚至删掉整条，用例仍旧全绿（报告 §4 的 M7 实测正是这个形态）。
///
/// # 红条件
///
/// 改掉 `RELATIVE_DAYS` 里任一英文条目的**值**，或删掉那一条，本条对应行即红；
/// 报告 §9 记了其中一条的实测。基准 `Now` 取**非零**（1000 日），
/// 使「恒返回 `At(0)`」或「恒返回基准」这两种写法也各自会红。
#[test]
fn english_relative_expressions_each_resolve_to_a_hand_written_offset() {
    // 1000 日。这三条「偏移 0」的条目（today / this week / now）与基准同值，
    // 但它们与「偏移非 0」的四条同处一表，后者能把「表根本没被读」挡下来。
    let now = Now::from_millis(1000 * DAY_MS);

    // 每条一个手写字面期望值，7 条不漏。
    assert_eq!(
        resolve_temporal(&temporal_mention("today"), &now),
        TemporalBinding::At(1000 * DAY_MS),
        "today = +0 日",
    );
    assert_eq!(
        resolve_temporal(&temporal_mention("tomorrow"), &now),
        TemporalBinding::At(1001 * DAY_MS),
        "tomorrow = +1 日",
    );
    assert_eq!(
        resolve_temporal(&temporal_mention("yesterday"), &now),
        TemporalBinding::At(999 * DAY_MS),
        "yesterday = -1 日",
    );
    assert_eq!(
        resolve_temporal(&temporal_mention("this week"), &now),
        TemporalBinding::At(1000 * DAY_MS),
        "this week = +0 日",
    );
    assert_eq!(
        resolve_temporal(&temporal_mention("next week"), &now),
        TemporalBinding::At(1007 * DAY_MS),
        "next week = +7 日",
    );
    assert_eq!(
        resolve_temporal(&temporal_mention("last week"), &now),
        TemporalBinding::At(993 * DAY_MS),
        "last week = -7 日",
    );
    assert_eq!(
        resolve_temporal(&temporal_mention("now"), &now),
        TemporalBinding::At(1000 * DAY_MS),
        "now = +0 日",
    );
}
