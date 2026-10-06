//! Task 13：§251 的**升级阶梯的数据形状**（设计 `docs/superpowers/specs/2026-10-05-p3d-model-registry-router-design.md` §7.1）。
//!
//! # 它的消费者本阶段不存在，故照片只能是对纯函数的直接调用
//!
//! §7.1 把三件事留在了别处：**失败检测与升级触发**归**子项目 G**（模型调用路径）、
//! §251 的前提检查「仍满足 Contract 和 Budget」归**语义层的 Budget Validator**（§110）、
//! **§86 的降级调度**是「用一个新的 `TaskSkillRequirement` **再调一次 `rank`**」。
//! 故 `next_step` 今天**既没有产生方也没有消费方**（设计 §10 第 3 条）。
//! 本文件里的照片只能是**对纯函数的直接调用**——它钉住了「阶梯是有序的、末档是 `None`」，
//! **钉不了**「驱动真的在一次失败之后升级了」。后者要等 G 的触发路径存在。
//! 这是本层能拿出的**全部**证据，不是漏了一处端到端用例。
//!
//! # 本层没有落库编码可钉
//!
//! 设计 `:586-587` 具名本类型：`RoutableState` / `FamilyRelation` / `EscalationStep` /
//! `RoutingReason` / 四枚错误类型「**都不进任何列，没有落库编码，也就没有字面量可给**」。
//! Task 11 对 `FamilyRelation` 正是照此办理（未写 `as_str`）。故本文件**不钉任何编码**：
//! 逐档钉的是**变体本身**（见 [`the_ladder_has_exactly_the_five_steps_of_251`]）。
//!
//! # 「`Tier1Low` 写不出来」的照片住在 `tests/compile_fail/`
//!
//! §86 的降级例写「Tier 1 Low」，而 §251 的阶梯五档里没有它（「Low」是 §18 的**推理强度**取值，
//! 不是 §17 的 Tier）——见 `tests/compile_fail/tier_one_low_is_not_a_step.rs`。
//! 那是**签名层**命题：运行期用例只能证明「我没写过这个名字」，证明不了「写不出来」。
//! 那份样例**由 `tests/type_level.rs` 的 `tests/compile_fail/*.rs` 通配收走**，
//! 本文件不另写一份 `t.compile_fail(...)`：同一份样例跑两遍不多出任何证据，
//! 只会让「变红的是哪一处」变模糊（同 `tests/router.rs` 的处置）。

use continuum_model_registry::{EscalationStep, next_step};

/// §251 的五档**逐档点名**，且「恰好这五枚」是**编译期**事实。
///
/// 设计 `:586-587` 定本类型**没有落库编码**（不进任何列），故这里**没有字面量可钉**。
/// 能钉的是变体本身，分两层：
///
/// 1. **逐档**：下面五次 `assert_eq!` 各点名一枚变体并对 [`name`] 的映射取值。五次**不抽代表**；
///    每次都能被一个**仍然编得过**的变异体打红——改 [`name`] 里**那一枚自己的臂**即可
///    （例：`Tier2High => "Tier2"`），故每次都是**守卫**（有具名变异体）而不是恒真断言。
/// 2. **恰好五枚**：[`name`] 的 `match` **穷尽且无 `..`**。加第六枚变体时它**编译不过**
///    （E0004 non-exhaustive patterns）——**这才是「恰好五枚」的照片**。
///    若只写一张五元素清单（例如一个 `[EscalationStep; 5]`），那只是把五枚**抄一遍**，
///    加第六枚时清单照样编得过、用例照样绿：抄写不是钉。
#[test]
fn the_ladder_has_exactly_the_five_steps_of_251() {
    /// 逐枚点名。**无 `..`**：新增变体即 E0004（见测试文档第 2 层）。
    fn name(step: EscalationStep) -> &'static str {
        match step {
            EscalationStep::Tier1 => "Tier1",
            EscalationStep::Tier2 => "Tier2",
            EscalationStep::Tier2High => "Tier2High",
            EscalationStep::CrossFamily => "CrossFamily",
            EscalationStep::Specialized => "Specialized",
        }
    }

    assert_eq!(name(EscalationStep::Tier1), "Tier1");
    assert_eq!(name(EscalationStep::Tier2), "Tier2");
    assert_eq!(name(EscalationStep::Tier2High), "Tier2High");
    assert_eq!(name(EscalationStep::CrossFamily), "CrossFamily");
    assert_eq!(name(EscalationStep::Specialized), "Specialized");
}

/// §251 的阶梯是**有序**的：`Tier1 → Tier2 → Tier2High → CrossFamily → Specialized`。
///
/// 这是**顺序断言，不是集合断言**：本用例把 [`next_step`] 的链**走到底**，收集成一个**有序**序列
/// 再比对。集合断言（「这五枚都在」）会让「顺序被改乱」照样绿，而顺序正是 §251 那张
/// 箭头图要表达的东西。
///
/// **顺序不来自变体的声明序**：[`next_step`] 的 `match` 显式写出每一档的去向，
/// 声明序是一种没说出口的规格（同 `family_rank` 的理由）。
#[test]
fn the_ladder_is_ordered_as_251_states() {
    let mut walked = vec![EscalationStep::Tier1];
    let mut current = EscalationStep::Tier1;

    // 步数上限 8：给这条链一个**终止保证**。若 [`next_step`] 哪天不再收敛（例如末档回卷），
    // 本用例会在 8 步后停下、由下面的 `assert_eq!` **变红**，而不是**挂死**——
    // 「挂死」不是一种红，读报告的人会以为用例没问题。
    for _ in 0..8 {
        match next_step(current) {
            Some(next) => {
                walked.push(next);
                current = next;
            }
            None => break,
        }
    }

    assert_eq!(
        walked,
        vec![
            EscalationStep::Tier1,
            EscalationStep::Tier2,
            EscalationStep::Tier2High,
            EscalationStep::CrossFamily,
            EscalationStep::Specialized,
        ]
    );
}

/// [`next_step`] 的**逐档**行为：五枚各一次，给出下一档；**末档给 `None`**。
///
/// **逐项有照片，不抽代表**：五条断言一一对应五枚变体。每条都能被一个仍然编得过的变异体打红
/// ——改 [`next_step`] 里**那一枚自己的臂**即可；特别是末档那条，把
/// `Specialized => None` 改成 `Specialized => Some(EscalationStep::Tier1)` 就打红它
/// （该变异体仍然编得过——`Tier1` 是一枚真变体）。
///
/// **末档是 `None` 而不是自环或回卷**：[`EscalationStep::Specialized`] 之后没有档。
/// 「回卷到 `Tier1`」是**降级**，走的是「用一个新的 `TaskSkillRequirement` 再调一次 `rank`」，
/// **不经过 [`next_step`]**（设计 §7.1 末段、本模块头）。
#[test]
fn next_step_walks_the_ladder_in_order() {
    assert_eq!(
        next_step(EscalationStep::Tier1),
        Some(EscalationStep::Tier2)
    );
    assert_eq!(
        next_step(EscalationStep::Tier2),
        Some(EscalationStep::Tier2High)
    );
    assert_eq!(
        next_step(EscalationStep::Tier2High),
        Some(EscalationStep::CrossFamily)
    );
    assert_eq!(
        next_step(EscalationStep::CrossFamily),
        Some(EscalationStep::Specialized)
    );
    assert_eq!(next_step(EscalationStep::Specialized), None);
}
