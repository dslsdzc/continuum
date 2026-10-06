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

/// §251 的五档**逐档点名**——且本用例钉的全是**编译期**事实，**没有一条运行期断言**。
///
/// 设计 `:586-587` 定本类型**没有落库编码**（不进任何列），故这里**没有字面量可钉**。
/// 于是能钉的只剩**拼写**，而「拼写」有两种钉法，本用例只取后者：
///
/// # 为什么删掉了字符串比较（原稿的五条 `assert_eq!`）
///
/// 「`EscalationStep::Tier1` 的拼写是 `"Tier1"`」是一句关于**测试自己**的话：它**没有实现侧的变异体**
/// ——改 `src/` 里任何一行都不会让它红（唯一能让它红的是改测试自己那几行）。按本项目刚定的守卫判据
/// （*B 是守卫 ⟺ 存在仍能编译的具名变异体 M，使 M 下 B 之前断言全过、B 失败*），它**不是守卫**。
///
/// **留下这样一条断言、只在注释里写「它没有实现侧变异体」，是最坏的一种**：一句关于测试自己的话
/// 留在原地，后来者会重新把它误标成守卫（**这是假绿的书面背书**）。
/// 记录拼写用**注释**比用**恒真断言**更诚实——故这里只**点名**，不比较。
///
/// # 编译期锚点（本用例真正钉住的东西）
///
/// [`name`] 的 `match` **穷尽且无 `..`**，且对五枚变体**逐一具名调用**。这给出两条**编译期**事实：
///
/// - **改名**任一枚变体（例：`Tier2High`）→ 本测试文件编译不过（E0599：`match` 的臂与下面的调用
///   都指向一个不存在的变体）；
/// - **新增第六枚变体** → [`name`] 的 `match` 编译不过（E0004 non-exhaustive patterns）。
///
/// **`src` 的 [`next_step`] 与测试的 `name` 是两个 `match`，覆盖不同的改法**，故本用例是**独立**锚点、
/// 不与 `next_step` 那处重复：
///
/// - **`src` 的 [`next_step`] 挡「加变体**未**兜底」**：新变体没写去向，那处即 E0004；
/// - **测试的 `name` 挡「加变体**并**用 `_` 兜底」**。具名变异体
///   **M ＝ 给枚举加第六枚 `Tier1Low`，同时给 `src/escalation.rs` 的 [`next_step`] `match` 末尾加
///   `_ => None`。** M 下 `src` 编译通过、[`next_step`] 对原五档**逐字不变**（测试 2 / 3 全绿——
///   即本仓「行为等价变异体不算红」的那一类），也不触发 `0 warning`（新变体是**公开 API**，
///   不落 `dead_code`；`_` 可达，无 `unreachable_patterns`）。**唯一红点：本用例 `name` 的无 `..`
///   `match` → E0004。** 「加一档顺手用 `_` 兜底」是**真实会发生的编辑形状**，故本用例**保留、
///   不降级进注释**——注释不会 E0004。
///
/// 若只写一张五元素清单（例如一个 `[EscalationStep; 5]`）而不写这个 `match`，那只是把五枚**抄一遍**，
/// 加第六枚时清单照样编得过、用例照样绿（M 就更打不红它了）：**抄写不是钉**。
///
/// # 逐档的**运行期**照片不在这里
///
/// 简报 Step 1 要的「逐档运行期断言」由 [`next_step_walks_the_ladder_in_order`] 承担：它对五档各一条，
/// **每条都能被 `src/` 侧的具名变异体打红**（变异 (a)(b) 两轮已证）。本用例**不重复承担**这件事
/// ——同一个性质在两处各写一遍，只会让「红在哪一处」变模糊。
#[test]
fn the_ladder_has_exactly_the_five_steps_of_251() {
    /// 逐枚点名。**无 `..`**：新增变体即 E0004；改名任一变体即 E0599（见测试文档）。
    fn name(step: EscalationStep) -> &'static str {
        match step {
            EscalationStep::Tier1 => "Tier1",
            EscalationStep::Tier2 => "Tier2",
            EscalationStep::Tier2High => "Tier2High",
            EscalationStep::CrossFamily => "CrossFamily",
            EscalationStep::Specialized => "Specialized",
        }
    }

    // 五枚逐一具名调用——**编译期锚点**（改名即 E0599），**不比较**返回值（比较是没有实现侧
    // 变异体的恒真断言，见测试文档）。五次**不抽代表**。
    let _ = name(EscalationStep::Tier1);
    let _ = name(EscalationStep::Tier2);
    let _ = name(EscalationStep::Tier2High);
    let _ = name(EscalationStep::CrossFamily);
    let _ = name(EscalationStep::Specialized);
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
