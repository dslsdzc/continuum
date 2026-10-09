//! Task 3 的**编译期**用例：三条「crate 外写不出来」的不可表达性命题，
//! 各配一份 `tests/compile_fail/*.rs` ＋ 同名 `.stderr`。
//!
//! **不可表达性只有编译失败样例能钉**：运行期用例无论怎么写，都只能证明「我没这么构造过」，
//! 证明不了「做不到」——`EntityId` 的里子串若真是 `pub`，一份运行期用例照样绿。
//! 故用 `trybuild`：样例必须**编不过**，且必须**因为预期的那条错**编不过。
//! 每份样例配一份同名 `.stderr`，由 `trybuild` **逐字比对**（只有路径与行号会被规范化，
//! **错误码不会**）——否则「因为拼错函数名而编译失败」也会让用例变绿，
//! 那样例钉的就是拼写，不是私有性。
//!
//! # 本目录的样例**逐份点名**（便于对数：这段点名的项数必须等于目录里 `.rs` 的份数）
//!
//! `tests/compile_fail/`（**三份**，应编不过）：
//!
//! - **`entity_id_has_no_public_literal_construction`** —— `EntityId` 的里子串私有，
//!   故 crate 外写不出元组构造 `EntityId("x".to_owned())`。它是简报里
//!   `entity_id_round_trips_and_has_no_public_literal_construction` 的 **(b) 半边**
//!   （(a) 半边的运行期断言在 `tests/candidate.rs` 的同名用例里）。
//!   预期是「元组结构体构造子是私有的」那一类；**判据是实跑产出的那份 `.stderr` 本身**，
//!   不是某个码——本项目为「凭记忆写错误码」付过代价。
//! - **`candidate_source_has_no_pick_one_method`** —— crate 外调 `source.pick_one(&m)`
//!   编不过（那份同名 `.stderr` 逐字比对的正是 `E0599: no method named \`pick_one\``）。
//!   **名字与它钉的东西相等**：它钉的就是 `pick_one` 这个名字不存在，不是
//!   「本层没有任何单实体方法」那句更宽的话——方法名是开放集，枚举不完
//!   （详见 `src/candidate.rs` 的「这条守卫的限度」；样例文件名曾叫
//!   `…_has_no_single_entity_method`，那句比照片宽，已按本仓判据「用例名也是一种断言」订正）。
//!   它是简报里 `a_candidate_source_can_only_return_a_set` 的**反例那半边**。
//! - **`canonical_representation_is_not_constructible_outside`** ——
//!   `CanonicalRepresentation` 的字段私有、唯一构造入口 `new` 是 `pub(crate)`
//!   （`src/canonical.rs`），故 crate 外拼不出结构体字面量。
//!   这一份是 **Task 2 转来的 P-2-1**（`docs/superpowers/p4-followups.md`）：
//!   那半张照片要 `trybuild`，而 `trybuild` 这个 dev-dep 是本 task 才引入的。
//!
//! `tests/compile_pass/`（**两份**，**应编得过**，是上面那两份**反例的正控制**）：
//!
//! - **`entity_id_from_str_is_constructible`** —— `EntityId::from_str` 是公开构造入口，
//!   crate 外编得过。**没有这一条**，上面第一份样例的绿可以来自「整个 crate 的 API 名字都对不上」
//!   这类与私有性无关的原因（恒绿的守卫比没有更坏）。
//! - **`a_candidate_source_with_only_retrieve`** —— 一个**只实现 `retrieve`** 的实现编得过
//!   ⇒ `CandidateSource` 的**必需**方法集恰好是那一个。
//!   这一份是「trait 上多加一个必需方法」那枚变异体的哨兵：加了就会红。
//!
//! # 本 crate 为何有 `tests/compile_pass/`（另两个 crate 没有）
//!
//! `crates/continuum-node/tests/` 与 `crates/continuum-model-registry/tests/` 下**只有
//! `compile_fail/`**（本仓现状）。那不是笔误，也不靠本 crate 去统一：本 crate 需要一份编译
//! **通过**的样例，是因为 `a_candidate_source_with_only_retrieve` 钉的命题**只有编译期能表达**
//! ——「一个只实现 `retrieve` 的实现编得过」⇔「该 trait 的**必需**方法集恰好是那一个」；
//! 运行期用例只能证明「我没写第二个方法」，证明不了「写不出来」。
//! 那两个 crate 的样例钉的都是「某条通道写不出来」，其**正面那半各有各的落点**，都不需要一个
//! 编译通过的目录。`continuum-node` 的正面那半在 crate 外的**运行期**用例里就有照片
//! （`tests/node.rs` 经 `ComputeNode::new` 造一枚节点）。`continuum-model-registry` 则没有一条
//! **需要**写成「编得过」的正面样例——**可写的正面样例是有的、只是不必要**（如
//! `TaskSkillRequirement::try_new` 公开且 crate 外可用，`tests/router.rs` 已在集成测试里这么用）；
//! 而本 crate 的 `ModelProfile` 在 crate 外没有构造入口（`try_new` 是 `pub(crate)`、十二个字段私有），
//! 集成测试里那份画像只能经公开的 `load_profile` 从库里读回（这一句写在 `tests/persist.rs`
//! **中段一节的 `///`** 里，`:827-828`；**不是**该文件的头注释——该文件头是 `//!`、不讲这件事），
//! 而需要画像的闸门用例住在 crate 内的 `#[cfg(test)] mod tests`
//! （`tests/lifecycle.rs` 的头注释点明了这条后果）。
//!
//! # 这份清单钉的是「本层没有 `pick_one` 这个口子」，不是「唯一候选不许出现」
//!
//! 前者的限度（它只钉一个**名字**）写在 `src/candidate.rs` 的「这条守卫的限度」一节；
//! 后者的限度写在 `tests/candidate.rs` 的 `a_candidate_source_can_only_return_a_set` 头注释里：
//! 一个来源**内部**只交出一枚候选，本层看不出、也不该看出（§197 的高召回是**手段**层面的事）。
//!
//! # 「通配自动收」不等于「清单自动更新」
//!
//! 下面 `type_level_guarantees_hold` 里的两条 `t.compile_fail` / `t.pass` 是**通配**，
//! 新增一份样例**不需要改那两行驱动代码**；但**本文件的 `COMPILE_FAIL_SAMPLES` /
//! `COMPILE_PASS_SAMPLES` 与上面这段点名是手写的**，加样例的人必须同时加上。
//! 这一处有守卫：两个常量、本文件头的点名、两个目录三者由
//! `the_sample_lists_match_the_directories` 对钉——加了样例忘了改清单，会有一条用例红，
//! 它的消息就写着要改哪里。新增样例后**要重跑本文件**来接收它生成的 `.stderr`
//! （首跑落 `wip/*.stderr`，搬进 `tests/compile_fail/` 才算接受）。

// 本文件**不 `use continuum_canonical`**：它只驱动 `trybuild` 并核对清单，
// 那些「编得过 / 编不过」的话都由 `tests/compile_pass/` 与 `tests/compile_fail/` 下的样例去说。
// **公开 API 从 crate 外真的拿得到**那一侧的证据因此不在这里，而在
// `tests/candidate.rs`（它整份都是 crate 外代码）＋ 两份 `compile_pass` 样例。

/// 清单里点到的、**应编不过**的样例名（`tests/compile_fail/` 下 `.rs` 的文件名主干）——**三份**。
const COMPILE_FAIL_SAMPLES: [&str; 3] = [
    "entity_id_has_no_public_literal_construction",
    "candidate_source_has_no_pick_one_method",
    "canonical_representation_is_not_constructible_outside",
];

/// 清单里点到的、**应编得过**的样例名（`tests/compile_pass/` 下 `.rs` 的文件名主干）——**两份**。
const COMPILE_PASS_SAMPLES: [&str; 2] = [
    "entity_id_from_str_is_constructible",
    "a_candidate_source_with_only_retrieve",
];

/// 两份清单与它们各自的目录对钉——**加样例时此清单要同步，本用例会告诉你**。
#[test]
fn type_level_guarantees_hold() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
    t.pass("tests/compile_pass/*.rs");
}

/// 清单与目录对钉，且清单还得在文件头的散文里点名。
///
/// # 它守的是哪一个坏法
///
/// 加一份样例要对齐三处：目录下的 `.rs`、同名 `.stderr`（`compile_pass` 那侧没有 `.stderr`）、
/// **手写的清单**。前两处本来就有守卫（上面那条的**通配**收走 `.rs`，`.stderr` 由 `trybuild`
/// 逐字比对，缺了会红）；**只有清单原先没人守**，而实际会发生的坏法只有一个方向：
/// **加了样例、忘了改清单**。本用例补的就是那一处。
///
/// # 它**不**查什么（免得被读成覆盖得更广）
///
/// **只查名字集合**：不解析分组、不解析错误码、不比对 `.stderr` 的内容——那些由
/// `type_level_guarantees_hold` 与各份样例自身负责。也**不查 `.stderr` 是否配齐**
/// （那是 `trybuild` 的事），**也不查 `compile_pass/` 下有没有多余的 `.stderr`**。
/// 更细一层：第二条断言用的是 `str::contains`(**子串匹配**)，故它**不校验名字出现在哪一行**、
/// 也不校验它周围的文字——见下面对那条断言的说明。
#[test]
fn the_sample_lists_match_the_directories() {
    assert_eq!(
        listed(&COMPILE_FAIL_SAMPLES),
        samples_in("tests/compile_fail"),
        "清单与 tests/compile_fail/*.rs 的名字集合必须相等：加了样例，就往 COMPILE_FAIL_SAMPLES **和**文件头那段点名各加一行"
    );
    assert_eq!(
        listed(&COMPILE_PASS_SAMPLES),
        samples_in("tests/compile_pass"),
        "清单与 tests/compile_pass/*.rs 的名字集合必须相等：加了样例，就往 COMPILE_PASS_SAMPLES **和**文件头那段点名各加一行"
    );

    // 第二条：两份清单里的名字**还都必须在文件头的散文清单里点名**——否则「改了常量、忘了散文」
    // 一样静默。**只截到第一个 `const` 之前**：否则常量自己那几行会让本断言恒真
    // （这是本守卫自身的假绿形状，故把截断的理由写在这里）。
    //
    // **它实际钉的比上面那句话弱**：判据是 `header.contains(name)`，即**子串**——名字只要在
    // 该段散文的任何位置出现一次即满足（出现在别处的行文里也算数），它**不校验**名字落在那条
    // 点名清单的行上，也不校验那行写了什么。把名字从点名处挪走、却留在文件头别处，本断言仍绿。
    // 且**本守卫自身没有照片**（没有一条负控制把某个名字挪出清单看它变红）——它是一处
    // 「靠人记得」的对钉。射程到此为止，故把这段写在这里，免得被读成「校对了点名清单」。
    let source = include_str!("type_level.rs");
    let header = source
        .split("const COMPILE_FAIL_SAMPLES")
        .next()
        .expect("本文件里应有 COMPILE_FAIL_SAMPLES 的定义");
    for name in COMPILE_FAIL_SAMPLES.iter().chain(COMPILE_PASS_SAMPLES.iter()) {
        assert!(
            header.contains(name),
            "文件头那段点名里没有 `{name}`：加样例时那两处都要同步（本用例就是那个守卫）"
        );
    }
}

/// 清单常量 → 排序后的 `Vec<String>`。
fn listed(names: &[&str]) -> Vec<String> {
    let mut out: Vec<String> = names.iter().map(|n| (*n).to_string()).collect();
    out.sort();
    out
}

/// 目录下的 `.rs` 文件名主干，排序后。
fn samples_in(dir: &str) -> Vec<String> {
    let mut out: Vec<String> = std::fs::read_dir(dir)
        .expect("样例目录应可读（用例的工作目录是 crate 根，同 trybuild 的相对路径）")
        .map(|entry| {
            entry
                .expect("目录项应可读")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter(|name| name.ends_with(".rs"))
        .map(|name| name.trim_end_matches(".rs").to_string())
        .collect();
    out.sort();
    out
}
