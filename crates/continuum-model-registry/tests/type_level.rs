//! 类型层只读保证的唯一直接证据（设计 §2.1「唯一产生点」、§2.3 第 3 条 (a)、§4.4 完成判据）。
//!
//! 「画像在 crate 外构造不出来」「读不到总分」都是**不可表达性**命题：运行期用例无论怎么写
//! 都只能证明「我没这么构造过 / 我没这么读过」，证明不了「做不到」。故用 `trybuild`
//! 的编译失败用例。每个 `compile_fail/*.rs` 配一份同名 `.stderr`，把失败钉在
//! 预期的报错上——否则「因为拼错函数名而编译失败」也会让用例变绿。
//!
//! **一份样例只钉一条通道，不合并**——本目录现有**十份**，下面**逐份点名**（便于对数：
//! 这份清单的项数必须等于目录里 `.rs` 的份数）：
//!
//! - **E0451 字段私有**（**四份**）：`model_profile_cannot_be_built`（`ModelProfile` 字面量）、
//!   `a_requirement_cannot_be_built_from_a_bare_vec`（`TaskSkillRequirement` 字面量）、
//!   `routable_model_fields_are_private`（`RoutableModel` 字面量）、
//!   `ranked_candidates_cannot_be_built`（排序输出的字面量）；
//! - **E0624 非 `pub`**（一份）：`model_profile_has_no_constructor`（`ModelProfile::try_new`）；
//! - **E0599 没有这个方法 / 变体**（**两份**）：`overall_score_cannot_be_read`（**画像上**读不到
//!   总分——`ModelProfile::overall_score` 这个方法不存在）、`tier_one_low_is_not_a_step`
//!   （Task 13：`EscalationStep::Tier1Low` 写不出来）；
//! - **E0609 没有这个字段**（一份）：`candidate_score_has_no_total_score`（Task 14：**`CandidateScore`
//!   上**没有总分子段——`overall` 这个字段不存在）。§248 的「不依赖单一总分」在**两个类型**上各有一条
//!   禁令，故**两份样例各钉一侧**：E0599 那份钉画像（方法），本份钉路由的输出接口面（字段）；
//! - **E0277 trait 未实现**（一份）：`a_requirement_cannot_be_built_by_conversion`（`From<Vec<_>>`）；
//! - **E0063 缺字段**（一份）：`a_routing_request_without_a_budget`（`RoutingRequest.budget`）。
//!
//! 合并成一份会让「改掉其中一条通道」只在 `.stderr` 的整体比对里漂移，读不出红在哪条上；
//! 且实测（rustc 1.95）同一函数体里 rustc **只报第一条错**，合并会让没被报出来的那条**没有照片**。
//!
//! **本目录由下面 `type_level_guarantees_hold` 里的 `tests/compile_fail/*.rs` 通配收走**：
//! 新增一份样例**不需要改本文件的代码**（通配自动收），但**要重跑本用例**来接收它生成的
//! `.stderr`——首次运行会落 `wip/*.stderr`，把它搬进 `tests/compile_fail/` 才算接受。
//!
//! **「通配自动收」不等于「清单自动更新」**：上面那份**按码分组的点名清单是手写的**，
//! 加样例的人必须**同时把它加上**。**这一处现在有守卫了**：`LISTED_SAMPLES`（下面那份常量）
//! 与**文件头这份清单**、**`tests/compile_fail/` 目录**三者由 `the_sample_list_matches_the_directory`
//! 对钉——**加了样例忘了改清单，会有一条用例红，它的消息就写着要改哪里**。
//! （Task 14 加第十份时这一处还是「靠人记得」；评审判它该有守卫，故补上。守卫落地前的那一版
//! 在这里写过「没有任何机械守卫」，那句**已经作废**，来历留此。）

/// 清单里点到的**十份**样例名（`tests/compile_fail/` 下 `.rs` 的文件名主干）。
///
/// **它与文件头那份「按码分组的点名清单」是同一份清单的两处表示**，两处由下面
/// `the_sample_list_matches_the_directory` 对钉。加样例时**两处都要加**（本常量与文件头的清单）。
const LISTED_SAMPLES: [&str; 10] = [
    "a_requirement_cannot_be_built_by_conversion",
    "a_requirement_cannot_be_built_from_a_bare_vec",
    "a_routing_request_without_a_budget",
    "candidate_score_has_no_total_score",
    "model_profile_cannot_be_built",
    "model_profile_has_no_constructor",
    "overall_score_cannot_be_read",
    "ranked_candidates_cannot_be_built",
    "routable_model_fields_are_private",
    "tier_one_low_is_not_a_step",
];

#[test]
fn type_level_guarantees_hold() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
}

/// 清单与目录对钉——**加样例时此清单要同步，本用例会告诉你**。
///
/// # 它守的是哪一个坏法
///
/// 加一份样例要对齐三处：`tests/compile_fail/` 下的 `.rs`、同名 `.stderr`、**手写的清单**。
/// 前两处本来就有守卫（`type_level_guarantees_hold` 的**通配**收走 `.rs`，`.stderr` 由 trybuild
/// 逐字比对，缺了会红）；**只有清单原先没人守**，而实际会发生的坏法只有一个方向：
/// **加了样例、忘了改清单**。本用例补的就是那一处。
///
/// # 它**不**查什么（免得被读成覆盖得更广）
///
/// **只查名字集合**：不解析分组、不解析错误码、不比对 `.stderr` 的内容——那些由
/// `type_level_guarantees_hold` 与各份样例自身负责。也**不查 `.stderr` 是否配齐**
/// （那是 trybuild 的事）。
#[test]
fn the_sample_list_matches_the_directory() {
    let mut in_dir: Vec<String> = std::fs::read_dir("tests/compile_fail")
        .expect("tests/compile_fail/ 应可读（用例的工作目录是 crate 根，同 trybuild 的相对路径）")
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
    let mut listed: Vec<String> = LISTED_SAMPLES
        .iter()
        .map(|name| (*name).to_string())
        .collect();
    in_dir.sort();
    listed.sort();
    assert_eq!(
        listed, in_dir,
        "清单与 tests/compile_fail/*.rs 的名字集合必须相等：加了样例，就往 LISTED_SAMPLES **和**文件头那份点名清单各加一行"
    );

    // 第二条：清单里的名字**还必须在文件头的散文清单里点名**——否则「改了常量、忘了散文」
    // 一样静默。**只截到 `const LISTED_SAMPLES` 之前**：否则常量自己那十行会让本断言恒真
    // （这是本守卫自身的假绿形状，故把截断的理由写在这里）。
    let source = include_str!("type_level.rs");
    let header = source
        .split("const LISTED_SAMPLES")
        .next()
        .expect("本文件里应有 LISTED_SAMPLES 的定义");
    for name in LISTED_SAMPLES {
        assert!(
            header.contains(name),
            "文件头的点名清单里没有 `{name}`：加样例时那两份清单都要同步（本用例就是那个守卫）"
        );
    }
}
