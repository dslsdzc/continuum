//! Verifier Adversary 的类型与消费规则（设计 §9.1；计划 Task 8 的 Step 1）。
//!
//! 三条口径（照计划的测试纪律）：
//! - 谓词的**边界**由四条一起钉住：恒真（**移除档**）红前两条、恒假（**取反档**）红后两条、
//!   只拒 `Passed`（**放宽档**）红第二条——**四条一起才钉住「不得」的边界**；
//! - **本清单是函数 `all()`，不是 `const ALL`**：`CounterexampleFound` 的载荷含
//!   `BackendId`（`continuum-operator`：字段私有、`new` 不是 `const fn`），本 crate 在
//!   常量上下文里构造不出 `AdversaryOutcome` 的值——照 P5e `2543aeb` 的订正与 P5c
//!   `all_operators()` 的形状；数目 `2` 写在返回类型里，删一臂（同步改数目）会让数目断言在
//!   **运行期**红（锚点只取 `all()`；点名走切片比较，不靠越界索引——评审 I-1 的修复）；
//! - §9.2 的 mutation 插槽**没有运行期照片**：本文件末的注释块照实写明三条「不存在的东西」
//!   由 Task 11 的 grep 与人工通读钉，本 task 不写一条会跑的用例来覆盖它们。

use continuum_operator::BackendId;
use continuum_verify::{AdversaryOutcome, Claim, VerdictAggregate, adversary_permits_passing};

/// 反例臂的样本。载荷取空串：编码只覆盖判别串，载荷不进 `strategy_verdict` 列
/// （计划 `## 遗留` 第二节记这一处的取舍），故往返照片的样本也取空载荷。
fn counterexample_arm() -> AdversaryOutcome {
    AdversaryOutcome::CounterexampleFound {
        passed_verifier: BackendId::new(""),
        counterexample: Claim::from_str(""),
    }
}

/// §9.1 第 3 条：`CounterexampleFound` 时该轮聚合**不得**取 `Passed` / `PassedWithUnknown`；
/// 其余组合放行（§9.1 第 4 条：本轮它**不是判定反转**——**只拒绝那一对组合，不改写聚合值**）。
///
/// **四条逐条**：`(CounterexampleFound, Passed)` ⇒ 不允许；`(CounterexampleFound, PassedWithUnknown)`
/// ⇒ 不允许；`(CounterexampleFound, Failed)` ⇒ 允许；`(NoCounterexample, Passed)` ⇒ 允许。
///
/// 红条件：谓词写成恒真（**移除档**）⇒ 前两条红；写成恒假（**取反档**）⇒ 后两条红；
/// 写成只拒 `Passed`（**放宽档**）⇒ 第二条红。**四条一起才钉住「不得」的边界**。
#[test]
fn a_counterexample_forbids_a_passing_aggregate() {
    // 第一条：找到反例 + 本轮聚合为 Passed ⇒ 不允许。
    assert!(
        !adversary_permits_passing(&VerdictAggregate::Passed, &counterexample_arm()),
        "CounterexampleFound 时该轮不得取 Passed（§9.1 第 3 条）"
    );
    // 第二条：找到反例 + 本轮聚合为 PassedWithUnknown ⇒ 不允许（放宽档在这里红）。
    assert!(
        !adversary_permits_passing(&VerdictAggregate::PassedWithUnknown, &counterexample_arm()),
        "CounterexampleFound 时该轮不得取 PassedWithUnknown——「不得」覆盖这一臂（§9.1 第 3 条）"
    );
    // 第三条：找到反例 + Failed ⇒ 允许——拒绝只针对两个通过臂，不是「反例否决一切聚合」。
    assert!(
        adversary_permits_passing(&VerdictAggregate::Failed, &counterexample_arm()),
        "Failed 本就不放行完成，且「不得」不覆盖它——谓词不得把它也拒掉（只拒绝那一对组合）"
    );
    // 第四条：没有反例 + Passed ⇒ 允许——谓词不得恒假（本轮无触发）。
    assert!(
        adversary_permits_passing(
            &VerdictAggregate::Passed,
            &AdversaryOutcome::NoCounterexample
        ),
        "NoCounterexample 时不拦任何聚合（本轮无触发；谓词不是恒假）"
    );
}

/// 两臂的编码：逐臂字面串名 ＋ 两臂各一次往返 ＋ 表外串一律 `None`。
///
/// `AdversaryOutcome` 落 `verification_round.strategy_verdict` 列（Task 9），故编码在
/// `as_str` / `parse` 上（小写、多词以 `_` 连接）。**编码只覆盖判别串**：
/// `CounterexampleFound` 的载荷（`passed_verifier` / `counterexample`）不进编码、
/// 也不往返（往返照片的样本载荷取空串；取舍见计划 `## 遗留` 第二节）。
///
/// 红条件：改某一臂的串（**取反档**）⇒ 红——只改 `as_str` / `parse` 一侧即破往返；
/// 两侧一致改名则由两枚字面串断言钉住。
#[test]
fn the_two_arms_round_trip_through_their_encoding() {
    // 逐臂串名，不抽代表。
    assert_eq!(
        AdversaryOutcome::NoCounterexample.as_str(),
        "no_counterexample",
        "第一臂的编码"
    );
    assert_eq!(
        counterexample_arm().as_str(),
        "counterexample_found",
        "第二臂的编码"
    );

    // 两臂各一次往返。
    assert_eq!(
        AdversaryOutcome::parse(AdversaryOutcome::NoCounterexample.as_str()),
        Some(AdversaryOutcome::NoCounterexample),
        "no_counterexample 应解回 NoCounterexample"
    );
    assert_eq!(
        AdversaryOutcome::parse(counterexample_arm().as_str()),
        Some(counterexample_arm()),
        "counterexample_found 应解回 CounterexampleFound（载荷取空串的样本）"
    );

    // 表外串一律 None（不取默认臂——取默认会把一份别的东西当成本臂处理，同 VerdictAggregate）。
    assert_eq!(
        AdversaryOutcome::parse("CounterexampleFound"),
        None,
        "编码一律小写：大写形是表外串"
    );
    assert_eq!(AdversaryOutcome::parse(""), None, "空串是表外串");
}

/// 两臂的数目与逐字点名。**本清单是函数 `all()` 而不是 `const ALL`**（文件头的理由），
/// 数目 `2` 在返回类型里。
///
/// 红条件（**实测回填**，报告 fix 轮 1）：删掉 `all()` 里的一臂并**同步**把返回类型
/// `[AdversaryOutcome; 2]` 改成 `[…; 1]`（**移除档**）⇒ 数目断言在**运行期**红
/// （`left: 1, right: 2`；锚点只取 `all()`，纪律 1(a)）。**不同步**则数组字面量编不过——
/// 那是编译失败、不是变红（纪律 1(c)），故「删一臂」的锚定解法是**同步改数后运行期红**；
/// 点名用**切片比较**（先例 `tests/verifier.rs:704-751`），不靠越界的常量索引（索引形态会被
/// `unconditional_panic` 拒在编译期、跑不到断言——评审 I-1）。
/// **另一侧**：两臂**对调**（数目不变）⇒ 切片比较红、数目断言绿（两条互不代偿，亦实测）。
#[test]
fn the_two_arms_are_named() {
    let all = AdversaryOutcome::all();

    assert_eq!(all.len(), 2, "AdversaryOutcome 的名单与臂数不符");

    // 逐臂点名，不抽代表（右手是**手写**期望名单，不从 `all()` 派生）。
    assert_eq!(
        all.as_slice(),
        [
            AdversaryOutcome::NoCounterexample,
            AdversaryOutcome::CounterexampleFound {
                passed_verifier: BackendId::new(""),
                counterexample: Claim::from_str(""),
            },
        ]
        .as_slice(),
        "AdversaryOutcome::all() 的两臂与设计 §9.1 第 2 条不符（顺序即声明序）"
    );
}

// ---------- §9.2 mutation 的插槽：**没有运行期照片**（Task 11 钉） ----------
//
// `the_mutation_slot_is_the_test_arm_and_a_threshold_that_nobody_reads`
// ——**本 task 不写一条会跑的用例**（§9.2：本 task 不判 mutation）。三条都是
// 「不存在的东西」，跑不出断言；它们落成一条**源码面**的核对，由 **Task 11 的 grep 与
// 人工通读**钉（这里只留这一条注释指向它）：
//
//   1. 本 crate 的 `src/` 里 `mutation_score` **零命中**（量纲与取值域规范未给，§258 的
//      字段表里没有它——Task 11 的 grep 钉）；
//   2. `EvidenceStrength`（`src/evidence.rs`）上**没有 `Ord`**（§265 的「strength ↓」
//      没有可依赖的序——Task 11 对源码的 grep 与通读钉）；
//   3. `VerificationProfile.mutation_threshold`（`src/profile.rs`）**没有任何读者**
//      （只存不判，§5.2——Task 11 的 grep 与通读钉）。
//
// 「`mutation_threshold` 没有读者」另有一条**编译期照片**的说明：给它写一个读者即编译过，
// 而**今天没有**——也就是说，这一条**没有守卫**：一旦有人加了读者，不会有任何地方变红。
// 这一处今天只有约定，明写在此（据实记账，不假称它有照片）。
